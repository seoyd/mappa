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
const CATALOGS: [&str; 3] = [
    "assets/map/regional_packs.toml",
    "assets/map/gb_regional_packs.toml",
    "assets/map/ca_regional_packs.toml",
];

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

fn catalog_paths(root: &Path) -> Result<BTreeMap<String, String>> {
    let mut entries = BTreeMap::new();
    for catalog in CATALOGS {
        let value: Catalog = toml::from_str(&fs::read_to_string(root.join(catalog))?)?;
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

fn build_inventory(root: &Path) -> Result<Inventory> {
    let mut pack = Vec::new();
    for (path, manifest) in catalog_paths(root)? {
        let (bytes, sha256) = digest(&root.join(&path))?;
        let (_, manifest_sha256) = digest(&root.join(&manifest))?;
        if bytes < 127 {
            return Err(format!("PMTiles archive too small: {path}").into());
        }
        pack.push(Pack {
            path,
            manifest,
            manifest_sha256,
            bytes,
            sha256,
        });
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
    }
    Ok(inventory)
}

fn valid_sha(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
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

async fn install_from_url(root: &Path, inventory: &Inventory, base: &str) -> Result<usize> {
    let base = reqwest::Url::parse(base)?;
    if base.scheme() != "https"
        || !base.path().ends_with('/')
        || !base.username().is_empty()
        || base.password().is_some()
        || base.query().is_some()
        || base.fragment().is_some()
    {
        return Err("pack URL base must be an HTTPS directory without credentials or query".into());
    }
    let client = reqwest::Client::builder()
        .user_agent("Mappa offline pack installation")
        .build()?;
    let mut installed = 0;
    for entry in &inventory.pack {
        let destination = root.join(&entry.path);
        if destination.exists() {
            verify_file(root, entry)?;
            continue;
        }
        let url = base.join(&format!("{}.pmtiles", entry.sha256))?;
        let mut response = client.get(url).send().await?.error_for_status()?;
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

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let [_, command, root, inventory, rest @ ..] = args.as_slice() else {
        return Err("usage: map_pack_bundle inventory|verify|install-dir|install-url|stage REPO_ROOT INVENTORY.toml [SOURCE_ROOT|HTTPS_BASE|OUTPUT_DIR]".into());
    };
    let root = Path::new(root);
    let inventory_path = Path::new(inventory);
    match (command.as_str(), rest) {
        ("inventory", []) => {
            let inventory = build_inventory(root)?;
            fs::write(inventory_path, toml::to_string_pretty(&inventory)?)?;
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
        ("stage", [output]) => {
            let inventory = checked_inventory(root, inventory_path)?;
            let staged = stage_release(root, &inventory, Path::new(output))?;
            println!("staged_assets={staged} inventory_packs={}", inventory.pack.len());
        }
        _ => return Err("usage: map_pack_bundle inventory|verify|install-dir|install-url|stage REPO_ROOT INVENTORY.toml [SOURCE_ROOT|HTTPS_BASE|OUTPUT_DIR]".into()),
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
}
