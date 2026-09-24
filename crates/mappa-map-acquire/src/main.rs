//! Build-time official source downloader. The map runtime never calls this.

use std::{collections::BTreeSet, error::Error, path::PathBuf, sync::Arc, time::Duration};
use tokio::{fs, sync::Semaphore, task::JoinSet};

const MAX_SOURCE_BYTES: usize = 64 * 1024 * 1024;
const CONCURRENT_DOWNLOADS: usize = 8;

fn county_name(name: &str) -> bool {
    name.strip_prefix("tl_2025_")
        .and_then(|rest| rest.strip_suffix("_roads.zip"))
        .is_some_and(|county| county.len() == 5 && county.bytes().all(|b| b.is_ascii_digit()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: mappa-map-acquire INVENTORY.txt OUTPUT_DIR".into());
    }
    let output = PathBuf::from(&args[2]);
    fs::create_dir_all(&output).await?;
    let mut names = BTreeSet::new();
    for line in fs::read_to_string(&args[1]).await?.lines() {
        let name = line.trim();
        if name.is_empty() || name.starts_with('#') {
            continue;
        }
        if !county_name(name) || !names.insert(name.to_owned()) {
            return Err(format!("invalid or duplicate county source name: {name}").into());
        }
    }
    if names.is_empty() {
        return Err("empty source inventory".into());
    }
    let client = reqwest::Client::builder()
        .user_agent("Mappa build-time source acquisition")
        .timeout(Duration::from_secs(120))
        .build()?;
    let permits = Arc::new(Semaphore::new(CONCURRENT_DOWNLOADS));
    let mut tasks = JoinSet::new();
    for name in names {
        let permit = Arc::clone(&permits).acquire_owned().await?;
        let client = client.clone();
        let output = output.clone();
        tasks.spawn(async move {
            let _permit = permit;
            let url = format!("https://www2.census.gov/geo/tiger/TIGER2025/ROADS/{name}");
            let bytes = client
                .get(url)
                .send()
                .await?
                .error_for_status()?
                .bytes()
                .await?;
            if bytes.is_empty() || bytes.len() > MAX_SOURCE_BYTES {
                return Err::<(String, usize), Box<dyn Error + Send + Sync>>(
                    format!("source size outside allowed range: {name}").into(),
                );
            }
            let temporary = output.join(format!("{name}.part"));
            fs::write(&temporary, &bytes).await?;
            fs::rename(temporary, output.join(&name)).await?;
            Ok::<_, Box<dyn Error + Send + Sync>>((name, bytes.len()))
        });
    }
    let mut files = 0usize;
    let mut total_bytes = 0usize;
    while let Some(result) = tasks.join_next().await {
        let (name, bytes) = result??;
        files += 1;
        total_bytes += bytes;
        println!("downloaded={name} bytes={bytes}");
    }
    eprintln!("downloaded_files={files} compressed_bytes={total_bytes}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::county_name;

    #[test]
    fn only_publisher_county_filenames_are_accepted() {
        assert!(county_name("tl_2025_36061_roads.zip"));
        assert!(!county_name("../tl_2025_36061_roads.zip"));
        assert!(!county_name("tl_2025_36061_roads.zip/../../other"));
        assert!(!county_name("tl_2024_36061_roads.zip"));
    }
}
