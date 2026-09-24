//! Build-time inventory and installer for the offline regional map archives.
//! The map renderer never calls a network service.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    error::Error,
    fs::{self, File},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
#[derive(Deserialize)]
struct Catalog {
    pack: Vec<CatalogPack>,
}

#[derive(Deserialize)]
struct CatalogPack {
    path: PathBuf,
    manifest: PathBuf,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
struct Inventory {
    version: u8,
    pack: Vec<Pack>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
struct Pack {
    path: String,
    manifest: String,
    manifest_sha256: String,
    bytes: u64,
    sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    download_url: Option<String>,
}

fn relative_path(base: &Path, raw: &Path) -> Result<String> {
    if raw.is_absolute() {
        return Err("absolute pack path".into());
    }
    let mut parts = Vec::new();
    for component in base.join(raw).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop().ok_or("pack path escapes repository")?;
            }
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            _ => return Err("invalid pack path".into()),
        }
    }
    if parts.is_empty() {
        return Err("empty pack path".into());
    }
    Ok(parts.join("/"))
}

fn regional_catalog_files(directory: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for item in fs::read_dir(directory)? {
        let item = item?;
        let name = item.file_name();
        let name = name.to_string_lossy();
        if item.file_type()?.is_file()
            && (name == "regional_packs.toml" || name.ends_with("_regional_packs.toml"))
        {
            files.push(item.path());
        }
    }
    files.sort();
    if files.is_empty() {
        return Err(format!("no regional pack catalogs in {}", directory.display()).into());
    }
    Ok(files)
}

fn catalog_paths(root: &Path) -> Result<BTreeMap<String, String>> {
    let mut entries = BTreeMap::new();
    for catalog in regional_catalog_files(&root.join("assets/map"))? {
        let value: Catalog = toml::from_str(&fs::read_to_string(catalog)?)?;
        for pack in value.pack {
            let archive = relative_path(Path::new("assets/map"), &pack.path)?;
            let manifest = relative_path(Path::new("assets/map"), &pack.manifest)?;
            if !archive.ends_with(".pmtiles") || !manifest.ends_with(".toml") {
                return Err(format!("unexpected pack or manifest extension: {archive}").into());
            }
            if !root.join(&manifest).is_file() {
                return Err(format!("missing source manifest: {manifest}").into());
            }
            if entries.insert(archive.clone(), manifest).is_some() {
                return Err(format!("duplicate regional pack: {archive}").into());
            }
        }
    }
    Ok(entries)
}

fn digest(path: &Path) -> Result<(u64, String)> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut bytes = 0u64;
    let mut buffer = [0u8; 256 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        bytes = bytes.checked_add(read as u64).ok_or("file size overflow")?;
        hasher.update(&buffer[..read]);
    }
    Ok((bytes, format!("{:x}", hasher.finalize())))
}

fn build_inventory(root: &Path, previous: Option<&Inventory>) -> Result<Inventory> {
    let previous: BTreeMap<_, _> = previous
        .into_iter()
        .flat_map(|inventory| &inventory.pack)
        .map(|entry| (entry.path.as_str(), entry))
        .collect();
    let mut pack = Vec::new();
    for (path, manifest) in catalog_paths(root)? {
        let (bytes, sha256) = digest(&root.join(&path))?;
        let (_, manifest_sha256) = digest(&root.join(&manifest))?;
        if bytes < 127 {
            return Err(format!("PMTiles archive too small: {path}").into());
        }
        let mut entry = Pack {
            path,
            manifest,
            manifest_sha256,
            bytes,
            sha256,
            download_url: None,
        };
        if let Some(old) = previous.get(entry.path.as_str())
            && old.manifest == entry.manifest
            && old.manifest_sha256 == entry.manifest_sha256
            && old.bytes == entry.bytes
            && old.sha256 == entry.sha256
        {
            entry.download_url = old.download_url.clone();
        }
        pack.push(entry);
    }
    Ok(Inventory { version: 1, pack })
}

fn checked_inventory(root: &Path, file: &Path) -> Result<Inventory> {
    let inventory: Inventory = toml::from_str(&fs::read_to_string(file)?)?;
    if inventory.version != 1 || inventory.pack.is_empty() {
        return Err("unsupported or empty map pack inventory".into());
    }
    let catalogs = catalog_paths(root)?;
    if inventory.pack.len() != catalogs.len() {
        return Err("inventory pack count differs from catalogs".into());
    }
    for (entry, (path, manifest)) in inventory.pack.iter().zip(&catalogs) {
        if &entry.path != path || &entry.manifest != manifest {
            return Err(format!("inventory differs from catalog at {path}").into());
        }
        if entry.bytes < 127 || !valid_sha(&entry.sha256) || !valid_sha(&entry.manifest_sha256) {
            return Err(format!("invalid inventory metadata: {path}").into());
        }
        if digest(&root.join(manifest))?.1 != entry.manifest_sha256 {
            return Err(format!("source manifest changed: {manifest}").into());
        }
        if let Some(url) = &entry.download_url {
            checked_asset_url(url, &entry.sha256)?;
        }
    }
    Ok(inventory)
}

fn valid_sha(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn checked_base_url(raw: &str) -> Result<reqwest::Url> {
    let url = reqwest::Url::parse(raw)?;
    if url.scheme() != "https"
        || !url.path().ends_with('/')
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("pack URL base must be an HTTPS directory without credentials or query".into());
    }
    Ok(url)
}

fn checked_asset_url(raw: &str, sha256: &str) -> Result<reqwest::Url> {
    let url = reqwest::Url::parse(raw)?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path_segments().and_then(|mut parts| parts.next_back())
            != Some(format!("{sha256}.pmtiles").as_str())
    {
        return Err(format!("invalid pinned pack URL: {raw}").into());
    }
    Ok(url)
}

fn write_inventory(path: &Path, inventory: &Inventory) -> Result<()> {
    let parent = path.parent().ok_or("inventory has no parent directory")?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(toml::to_string_pretty(inventory)?.as_bytes())?;
    temporary.as_file_mut().sync_all()?;
    temporary.persist(path)?;
    Ok(())
}

fn pin_release(inventory: &mut Inventory, base: &str, asset_list: &Path) -> Result<usize> {
    let base = checked_base_url(base)?;
    let names = fs::read_to_string(asset_list)?;
    let mut by_sha: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, entry) in inventory.pack.iter().enumerate() {
        by_sha.entry(entry.sha256.clone()).or_default().push(index);
    }
    let mut assigned = 0;
    let mut seen = std::collections::BTreeSet::new();
    for name in names.lines().filter(|line| !line.is_empty()) {
        let Some(sha256) = name.strip_suffix(".pmtiles") else {
            return Err(format!("unexpected release asset name: {name}").into());
        };
        if !valid_sha(sha256) || !seen.insert(sha256.to_owned()) {
            return Err(format!("invalid or duplicate release asset name: {name}").into());
        }
        let indexes = by_sha
            .get(sha256)
            .ok_or(format!("release asset not in inventory: {name}"))?;
        let url = base.join(name)?.to_string();
        checked_asset_url(&url, sha256)?;
        for &index in indexes {
            let entry = &mut inventory.pack[index];
            if entry.download_url.as_ref().is_some_and(|old| old != &url) {
                return Err(format!("pack already pinned to another URL: {}", entry.path).into());
            }
            if entry.download_url.is_none() {
                assigned += 1;
            }
            entry.download_url = Some(url.clone());
        }
    }
    if seen.is_empty() {
        return Err("release asset list is empty".into());
    }
    Ok(assigned)
}

fn verify_file(root: &Path, entry: &Pack) -> Result<()> {
    let (bytes, sha256) = digest(&root.join(&entry.path))?;
    if bytes != entry.bytes || sha256 != entry.sha256 {
        return Err(format!("map pack differs from inventory: {}", entry.path).into());
    }
    Ok(())
}

fn install_from_dir(root: &Path, inventory: &Inventory, source: &Path) -> Result<usize> {
    let mut installed = 0;
    for entry in &inventory.pack {
        let destination = root.join(&entry.path);
        if destination.exists() {
            verify_file(root, entry)?;
            continue;
        }
        let candidate = source.join(format!("{}.pmtiles", entry.sha256));
        let (bytes, sha256) = digest(&candidate)?;
        if bytes != entry.bytes || sha256 != entry.sha256 {
            return Err(format!("source pack differs from inventory: {}", entry.path).into());
        }
        let parent = destination.parent().ok_or("pack has no parent directory")?;
        fs::create_dir_all(parent)?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        std::io::copy(&mut File::open(&candidate)?, &mut temporary)?;
        temporary.as_file_mut().sync_all()?;
        // The source may have changed during copying; check the temporary file too.
        let (copied_bytes, copied_sha) = digest(temporary.path())?;
        if copied_bytes != entry.bytes || copied_sha != entry.sha256 {
            return Err(format!("copied pack differs from inventory: {}", entry.path).into());
        }
        temporary.persist(&destination)?;
        installed += 1;
    }
    Ok(installed)
}

fn stage_release(root: &Path, inventory: &Inventory, output: &Path) -> Result<usize> {
    fs::create_dir_all(output)?;
    let mut staged = 0;
    for entry in &inventory.pack {
        verify_file(root, entry)?;
        let destination = output.join(format!("{}.pmtiles", entry.sha256));
        if destination.exists() {
            let (bytes, sha256) = digest(&destination)?;
            if bytes != entry.bytes || sha256 != entry.sha256 {
                return Err(format!("staged asset differs from inventory: {}", entry.path).into());
            }
            continue;
        }
        let source = root.join(&entry.path);
        if fs::hard_link(&source, &destination).is_err() {
            fs::copy(&source, &destination)?;
        }
        let (bytes, sha256) = digest(&destination)?;
        if bytes != entry.bytes || sha256 != entry.sha256 {
            fs::remove_file(&destination)?;
            return Err(format!("staged asset differs from inventory: {}", entry.path).into());
        }
        staged += 1;
    }
    Ok(staged)
}

async fn install_from_urls(
    root: &Path,
    inventory: &Inventory,
    urls: &[reqwest::Url],
) -> Result<usize> {
    if urls.len() != inventory.pack.len() {
        return Err("pack URL count differs from inventory".into());
    }
    let client = reqwest::Client::builder()
        .user_agent("Mappa offline pack installation")
        .build()?;
    let mut installed = 0;
    for (entry, url) in inventory.pack.iter().zip(urls) {
        let destination = root.join(&entry.path);
        if destination.exists() {
            verify_file(root, entry)?;
            continue;
        }
        let mut response = client.get(url.clone()).send().await?.error_for_status()?;
        if response.url().scheme() != "https" {
            return Err(format!("pack download redirected away from HTTPS: {}", entry.path).into());
        }
        if response
            .content_length()
            .is_some_and(|size| size != entry.bytes)
        {
            return Err(format!("publisher size differs from inventory: {}", entry.path).into());
        }
        let parent = destination.parent().ok_or("pack has no parent directory")?;
        fs::create_dir_all(parent)?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        let mut hasher = Sha256::new();
        let mut bytes = 0u64;
        while let Some(chunk) = response.chunk().await? {
            bytes = bytes
                .checked_add(chunk.len() as u64)
                .ok_or("download size overflow")?;
            if bytes > entry.bytes {
                return Err(format!("download exceeds inventory size: {}", entry.path).into());
            }
            temporary.write_all(&chunk)?;
            hasher.update(&chunk);
        }
        if bytes != entry.bytes || format!("{:x}", hasher.finalize()) != entry.sha256 {
            return Err(format!("download differs from inventory: {}", entry.path).into());
        }
        temporary.as_file_mut().sync_all()?;
        temporary.persist(destination)?;
        installed += 1;
    }
    Ok(installed)
}

async fn install_from_url(root: &Path, inventory: &Inventory, base: &str) -> Result<usize> {
    let base = checked_base_url(base)?;
    let urls = inventory
        .pack
        .iter()
        .map(|entry| base.join(&format!("{}.pmtiles", entry.sha256)))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    install_from_urls(root, inventory, &urls).await
}

async fn install_pinned(root: &Path, inventory: &Inventory) -> Result<usize> {
    let urls = inventory
        .pack
        .iter()
        .map(|entry| {
            let raw = entry
                .download_url
                .as_deref()
                .ok_or(format!("pack has no download URL: {}", entry.path))?;
            checked_asset_url(raw, &entry.sha256)
        })
        .collect::<Result<Vec<_>>>()?;
    install_from_urls(root, inventory, &urls).await
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let [_, command, root, inventory, rest @ ..] = args.as_slice() else {
        return Err("usage: map_pack_bundle inventory|verify|install-dir|install-url|install-pinned|pin-release|stage REPO_ROOT INVENTORY.toml [SOURCE_ROOT|HTTPS_BASE|OUTPUT_DIR] [ASSET_LIST]".into());
    };
    let root = Path::new(root);
    let inventory_path = Path::new(inventory);
    match (command.as_str(), rest) {
        ("inventory", []) => {
            let previous = if inventory_path.exists() {
                let previous: Inventory = toml::from_str(&fs::read_to_string(inventory_path)?)?;
                if previous.version != 1 {
                    return Err("unsupported previous map pack inventory".into());
                }
                Some(previous)
            } else {
                None
            };
            let inventory = build_inventory(root, previous.as_ref())?;
            write_inventory(inventory_path, &inventory)?;
            println!("inventory_packs={} output={}", inventory.pack.len(), inventory_path.display());
        }
        ("verify", []) => {
            let inventory = checked_inventory(root, inventory_path)?;
            for entry in &inventory.pack {
                verify_file(root, entry)?;
            }
            let bytes: u64 = inventory.pack.iter().map(|entry| entry.bytes).sum();
            println!("verified_packs={} bytes={bytes}", inventory.pack.len());
        }
        ("install-dir", [source]) => {
            let inventory = checked_inventory(root, inventory_path)?;
            let installed = install_from_dir(root, &inventory, Path::new(source))?;
            println!("installed_packs={installed} verified_packs={}", inventory.pack.len());
        }
        ("install-url", [base]) => {
            let inventory = checked_inventory(root, inventory_path)?;
            let installed = install_from_url(root, &inventory, base).await?;
            println!("installed_packs={installed} verified_packs={}", inventory.pack.len());
        }
        ("install-pinned", []) => {
            let inventory = checked_inventory(root, inventory_path)?;
            let installed = install_pinned(root, &inventory).await?;
            println!("installed_packs={installed} verified_packs={}", inventory.pack.len());
        }
        ("pin-release", [base, asset_list]) => {
            let mut inventory = checked_inventory(root, inventory_path)?;
            let pinned = pin_release(&mut inventory, base, Path::new(asset_list))?;
            write_inventory(inventory_path, &inventory)?;
            println!("pinned_assets={pinned} inventory_packs={}", inventory.pack.len());
        }
        ("stage", [output]) => {
            let inventory = checked_inventory(root, inventory_path)?;
            let staged = stage_release(root, &inventory, Path::new(output))?;
            println!("staged_assets={staged} inventory_packs={}", inventory.pack.len());
        }
        _ => return Err("usage: map_pack_bundle inventory|verify|install-dir|install-url|install-pinned|pin-release|stage REPO_ROOT INVENTORY.toml [SOURCE_ROOT|HTTPS_BASE|OUTPUT_DIR] [ASSET_LIST]".into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_paths_cannot_escape_repository() {
        assert_eq!(
            relative_path(
                Path::new("assets/map"),
                Path::new("../../artifacts/a.pmtiles")
            )
            .unwrap(),
            "artifacts/a.pmtiles"
        );
        assert!(relative_path(Path::new("assets/map"), Path::new("../../../outside")).is_err());
        assert!(relative_path(Path::new("assets/map"), Path::new("/outside")).is_err());
    }

    #[test]
    fn catalog_discovery_includes_new_region_without_code_change() {
        let temporary = tempfile::tempdir().unwrap();
        for name in [
            "regional_packs.toml",
            "gb_regional_packs.toml",
            "fr_regional_packs.toml",
            "unrelated.toml",
        ] {
            fs::write(temporary.path().join(name), "pack = []\n").unwrap();
        }
        let files = regional_catalog_files(temporary.path()).unwrap();
        let names: Vec<_> = files
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            [
                "fr_regional_packs.toml",
                "gb_regional_packs.toml",
                "regional_packs.toml"
            ]
        );
    }

    #[test]
    fn installer_rejects_corrupted_source_without_writing_destination() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("repo");
        let source = temporary.path().join("source");
        fs::create_dir_all(&source).unwrap();
        fs::write(
            source.join(format!("{}.pmtiles", "0".repeat(64))),
            vec![1; 128],
        )
        .unwrap();
        let inventory = Inventory {
            version: 1,
            pack: vec![Pack {
                path: "artifacts/a.pmtiles".into(),
                manifest: "data/a.toml".into(),
                manifest_sha256: "0".repeat(64),
                bytes: 128,
                sha256: "0".repeat(64),
                download_url: None,
            }],
        };
        assert!(install_from_dir(&root, &inventory, &source).is_err());
        assert!(!root.join("artifacts/a.pmtiles").exists());
    }

    #[test]
    fn installer_copies_verified_source_and_is_idempotent() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("repo");
        let source = temporary.path().join("source");
        let bytes = vec![7; 128];
        let inventory = Inventory {
            version: 1,
            pack: vec![Pack {
                path: "artifacts/a.pmtiles".into(),
                manifest: "data/a.toml".into(),
                manifest_sha256: "0".repeat(64),
                bytes: bytes.len() as u64,
                sha256: format!("{:x}", Sha256::digest(&bytes)),
                download_url: None,
            }],
        };
        fs::create_dir_all(source.join("artifacts")).unwrap();
        fs::write(source.join("artifacts/a.pmtiles"), &bytes).unwrap();
        let staged = temporary.path().join("staged");
        assert_eq!(stage_release(&source, &inventory, &staged).unwrap(), 1);
        assert_eq!(stage_release(&source, &inventory, &staged).unwrap(), 0);
        assert_eq!(
            fs::read(staged.join(format!("{}.pmtiles", inventory.pack[0].sha256))).unwrap(),
            bytes
        );
        assert_eq!(install_from_dir(&root, &inventory, &staged).unwrap(), 1);
        assert_eq!(install_from_dir(&root, &inventory, &staged).unwrap(), 0);
        assert_eq!(fs::read(root.join("artifacts/a.pmtiles")).unwrap(), bytes);
    }

    #[test]
    fn release_pin_assigns_only_listed_hashes_and_rejects_changes() {
        let temporary = tempfile::tempdir().unwrap();
        let names = temporary.path().join("assets.txt");
        let first = "a".repeat(64);
        let second = "b".repeat(64);
        let mut inventory = Inventory {
            version: 1,
            pack: [(&first, "a"), (&second, "b")]
                .into_iter()
                .map(|(sha256, name)| Pack {
                    path: format!("artifacts/{name}.pmtiles"),
                    manifest: format!("data/{name}.toml"),
                    manifest_sha256: "0".repeat(64),
                    bytes: 128,
                    sha256: sha256.clone(),
                    download_url: None,
                })
                .collect(),
        };
        fs::write(&names, format!("{first}.pmtiles\n")).unwrap();
        let base = "https://github.com/seoyd/mappa/releases/download/test/";
        assert_eq!(pin_release(&mut inventory, base, &names).unwrap(), 1);
        assert_eq!(pin_release(&mut inventory, base, &names).unwrap(), 0);
        assert_eq!(
            inventory.pack[0].download_url.as_deref(),
            Some(format!("{base}{first}.pmtiles").as_str())
        );
        assert!(inventory.pack[1].download_url.is_none());
        assert!(pin_release(&mut inventory, "http://example.com/", &names).is_err());
        assert!(pin_release(&mut inventory, "https://example.com/", &names).is_err());
        fs::write(&names, format!("{second}.pmtiles\n")).unwrap();
        assert_eq!(pin_release(&mut inventory, base, &names).unwrap(), 1);
    }

    #[test]
    fn pinned_url_requires_https_and_expected_asset_name() {
        let sha = "a".repeat(64);
        assert!(checked_asset_url(&format!("https://example.com/{sha}.pmtiles"), &sha).is_ok());
        assert!(checked_asset_url(&format!("http://example.com/{sha}.pmtiles"), &sha).is_err());
        assert!(checked_asset_url("https://example.com/other.pmtiles", &sha).is_err());
        assert!(
            checked_asset_url(
                &format!("https://example.com/{sha}.pmtiles?token=secret"),
                &sha
            )
            .is_err()
        );
    }

    #[test]
    fn inventory_rebuild_keeps_url_only_for_unchanged_pack() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        fs::create_dir_all(root.join("assets/map")).unwrap();
        fs::create_dir_all(root.join("artifacts")).unwrap();
        fs::create_dir_all(root.join("data")).unwrap();
        fs::write(
            root.join("assets/map/regional_packs.toml"),
            "[[pack]]\npath = '../../artifacts/a.pmtiles'\nmanifest = '../../data/a.toml'\n",
        )
        .unwrap();
        fs::write(
            root.join("assets/map/gb_regional_packs.toml"),
            "pack = []\n",
        )
        .unwrap();
        fs::write(root.join("data/a.toml"), "source = []\n").unwrap();
        fs::write(root.join("artifacts/a.pmtiles"), vec![7; 128]).unwrap();
        let mut first = build_inventory(root, None).unwrap();
        let url = format!("https://example.com/{}.pmtiles", first.pack[0].sha256);
        first.pack[0].download_url = Some(url.clone());
        assert_eq!(
            build_inventory(root, Some(&first)).unwrap().pack[0].download_url,
            Some(url)
        );
        fs::write(root.join("artifacts/a.pmtiles"), vec![8; 128]).unwrap();
        assert!(
            build_inventory(root, Some(&first)).unwrap().pack[0]
                .download_url
                .is_none()
        );
    }
}
