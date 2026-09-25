//! Download only county ZIPs named by a pinned official TIGER2025 inventory.
//! Build-time acquisition; the offline map runtime never calls Census servers.

use reqwest::Client;
use std::{collections::BTreeSet, error::Error, fs::File, io, path::Path, time::Duration};
use tokio::{fs, io::AsyncWriteExt, time::sleep};
use zip::ZipArchive;

const MAX_ZIP_BYTES: u64 = 64 * 1024 * 1024;

fn inventory_files(text: &str, class: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let mut files = Vec::new();
    let mut state = None;
    for line in text.lines() {
        let name = line.trim();
        if name.is_empty() || name.starts_with('#') {
            continue;
        }
        let county = name
            .strip_prefix("tl_2025_")
            .and_then(|rest| rest.strip_suffix(&format!("_{class}.zip")))
            .ok_or("inventory contains an unexpected file name")?;
        if county.len() != 5 || !county.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err("inventory contains an invalid county FIPS".into());
        }
        if state.is_some_and(|previous| previous != &county[..2]) {
            return Err("inventory mixes states".into());
        }
        state = Some(&county[..2]);
        if files
            .last()
            .is_some_and(|previous: &String| previous.as_str() >= name)
        {
            return Err("inventory file names are duplicate or not sorted".into());
        }
        files.push(name.to_owned());
    }
    if files.is_empty() || files.len() > 300 {
        return Err("inventory county count is outside the audited range".into());
    }
    Ok(files)
}

fn verify_zip(path: &Path) -> Result<u64, Box<dyn Error>> {
    let file = File::open(path)?;
    let bytes = file.metadata()?.len();
    if bytes == 0 || bytes > MAX_ZIP_BYTES {
        return Err(format!("county ZIP size is invalid: {}", path.display()).into());
    }
    let mut archive = ZipArchive::new(file)?;
    if archive.is_empty() {
        return Err(format!("county ZIP is empty: {}", path.display()).into());
    }
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or("county ZIP has no file stem")?;
    let mut members = BTreeSet::new();
    for index in 0..archive.len() {
        let mut member = archive.by_index(index)?;
        let name = member.name().to_owned();
        if !name.starts_with(&format!("{stem}.")) || name.contains('/') || name.contains('\\') {
            return Err(format!("county ZIP member does not match its source file: {name}").into());
        }
        if !members.insert(name) {
            return Err("duplicate county ZIP member".into());
        }
        io::copy(&mut member, &mut io::sink())?;
    }
    for suffix in ["shp", "shx", "dbf", "prj"] {
        if !members.contains(&format!("{stem}.{suffix}")) {
            return Err(format!("county ZIP is missing {suffix}: {}", path.display()).into());
        }
    }
    Ok(bytes)
}

async fn fetch_one(client: &Client, url: &str, path: &Path) -> Result<u64, Box<dyn Error>> {
    if fs::try_exists(path).await? {
        return verify_zip(path);
    }
    let temporary = path.with_extension("part");
    for attempt in 0..4 {
        let result: Result<u64, Box<dyn Error>> = async {
            let mut response = client.get(url).send().await?.error_for_status()?;
            if response.url().as_str() != url {
                return Err("Census county ZIP redirected".into());
            }
            if response
                .content_length()
                .is_some_and(|length| length > MAX_ZIP_BYTES)
            {
                return Err("Census county ZIP exceeds size limit".into());
            }
            let mut output = fs::File::create(&temporary).await?;
            let mut bytes = 0u64;
            while let Some(chunk) = response.chunk().await? {
                bytes += chunk.len() as u64;
                if bytes > MAX_ZIP_BYTES {
                    return Err("Census county ZIP exceeds size limit".into());
                }
                output.write_all(&chunk).await?;
            }
            output.flush().await?;
            output.sync_all().await?;
            drop(output);
            let checked = verify_zip(&temporary)?;
            fs::rename(&temporary, path).await?;
            Ok(checked)
        }
        .await;
        match result {
            Ok(bytes) => return Ok(bytes),
            Err(error) if attempt == 3 => return Err(error),
            Err(error) => {
                eprintln!(
                    "retry={} file={} error={error}",
                    attempt + 1,
                    path.display()
                );
                sleep(Duration::from_secs(1 << attempt)).await;
            }
        }
    }
    unreachable!()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    let (class, directory) = match args.get(1).map(String::as_str) {
        Some("--roads") if args.len() == 4 => ("roads", "ROADS"),
        Some("--areawater") if args.len() == 4 => ("areawater", "AREAWATER"),
        _ => {
            return Err(
                "usage: acquire_tiger_counties [--roads|--areawater] INVENTORY.txt OUTPUT_DIR"
                    .into(),
            );
        }
    };
    let files = inventory_files(&fs::read_to_string(&args[2]).await?, class)?;
    let destination = Path::new(&args[3]);
    fs::create_dir_all(destination).await?;
    let client = Client::builder()
        .user_agent("Mappa build-time TIGER2025 official source acquisition")
        .timeout(Duration::from_secs(120))
        .build()?;
    let mut total_bytes = 0u64;
    for (index, name) in files.iter().enumerate() {
        let url = format!("https://www2.census.gov/geo/tiger/TIGER2025/{directory}/{name}");
        total_bytes += fetch_one(&client, &url, &destination.join(name)).await?;
        if (index + 1) % 10 == 0 || index + 1 == files.len() {
            println!(
                "validated={}/{} zip_bytes={total_bytes}",
                index + 1,
                files.len()
            );
        }
        sleep(Duration::from_millis(200)).await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{inventory_files, verify_zip};
    use std::io::Write;

    #[test]
    fn accepts_one_state_and_rejects_mixed_or_duplicate_files() {
        let valid = "# official listing\ntl_2025_19001_roads.zip\ntl_2025_19003_roads.zip\n";
        assert_eq!(inventory_files(valid, "roads").unwrap().len(), 2);
        assert!(inventory_files(valid, "areawater").is_err());
        assert!(
            inventory_files("tl_2025_19001_roads.zip\ntl_2025_20001_roads.zip", "roads").is_err()
        );
        assert!(
            inventory_files("tl_2025_19001_roads.zip\ntl_2025_19001_roads.zip", "roads").is_err()
        );
        assert!(inventory_files("../tl_2025_19001_roads.zip", "roads").is_err());
    }

    #[test]
    fn zip_members_must_match_the_county_source_name() {
        let directory = tempfile::tempdir().unwrap();
        let correct = directory.path().join("tl_2025_19001_roads.zip");
        let mut archive = zip::ZipWriter::new(std::fs::File::create(&correct).unwrap());
        for suffix in ["shp", "shx", "dbf", "prj"] {
            archive
                .start_file(
                    format!("tl_2025_19001_roads.{suffix}"),
                    zip::write::SimpleFileOptions::default(),
                )
                .unwrap();
            archive.write_all(b"source bytes").unwrap();
        }
        archive.finish().unwrap();
        assert!(verify_zip(&correct).is_ok());
        let wrong = directory.path().join("tl_2025_19003_roads.zip");
        std::fs::copy(&correct, &wrong).unwrap();
        assert!(verify_zip(&wrong).is_err());
    }
}
