//! Build-time official source downloader. The map runtime never calls this.

use md5::{Digest, Md5};
use reqwest::header::{CONTENT_RANGE, RANGE};
use std::{
    collections::BTreeSet,
    error::Error,
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{fs, io::AsyncWriteExt, sync::Semaphore, task::JoinSet};
use zip::ZipArchive;

const MAX_SOURCE_BYTES: usize = 64 * 1024 * 1024;
const CONCURRENT_DOWNLOADS: usize = 8;

fn county_name(name: &str, class: &str) -> bool {
    name.strip_prefix("tl_2025_")
        .and_then(|rest| rest.strip_suffix(&format!("_{class}.zip")))
        .is_some_and(|county| county.len() == 5 && county.bytes().all(|b| b.is_ascii_digit()))
}

async fn download(
    client: &reqwest::Client,
    url: &str,
    output: &Path,
    max_bytes: u64,
    expected_md5: Option<&str>,
) -> Result<u64, Box<dyn Error + Send + Sync>> {
    let temporary = output.with_extension("part");
    let mut hasher = Md5::new();
    let mut total = if expected_md5.is_some() {
        match fs::metadata(&temporary).await {
            Ok(metadata) => metadata.len(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
            Err(error) => return Err(error.into()),
        }
    } else {
        0
    };
    if total > max_bytes {
        return Err("partial source exceeds publisher size".into());
    }
    if total > 0 {
        let mut existing = std::fs::File::open(&temporary)?;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let read = existing.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
    }
    if total < max_bytes {
        let mut request = client.get(url);
        if total > 0 {
            request = request.header(RANGE, format!("bytes={total}-"));
        }
        let mut response = request.send().await?.error_for_status()?;
        if total > 0 {
            let content_range = response
                .headers()
                .get(CONTENT_RANGE)
                .ok_or("resumed source has no Content-Range")?
                .to_str()?;
            if response.status() != reqwest::StatusCode::PARTIAL_CONTENT
                || !content_range.starts_with(&format!("bytes {total}-"))
                || !content_range.ends_with(&format!("/{max_bytes}"))
            {
                return Err("server did not honor the requested source range".into());
            }
        }
        if response
            .content_length()
            .is_some_and(|size| size > max_bytes - total)
        {
            return Err("source exceeds expected size".into());
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(total > 0)
            .truncate(total == 0)
            .open(&temporary)
            .await?;
        while let Some(chunk) = response.chunk().await? {
            total = total
                .checked_add(chunk.len() as u64)
                .ok_or("source size overflow")?;
            if total > max_bytes {
                return Err("source exceeds expected size".into());
            }
            file.write_all(&chunk).await?;
            if expected_md5.is_some() {
                hasher.update(&chunk);
            }
        }
        file.flush().await?;
    }
    if total == 0 || (expected_md5.is_some() && total != max_bytes) {
        return Err("source byte count differs from publisher metadata".into());
    }
    if let Some(expected) = expected_md5 {
        let actual = hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if actual != expected {
            return Err("publisher MD5 mismatch".into());
        }
    }
    if output
        .extension()
        .is_some_and(|extension| extension == "zip")
    {
        let archive = ZipArchive::new(std::fs::File::open(&temporary)?)?;
        if archive.is_empty() {
            return Err("publisher returned an empty ZIP archive".into());
        }
    }
    fs::rename(temporary, output).await?;
    Ok(total)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|arg| arg == "--file") {
        if args.len() != 6 {
            return Err(
                "usage: mappa-map-acquire --file HTTPS_URL OUTPUT EXPECTED_BYTES EXPECTED_MD5"
                    .into(),
            );
        }
        if !args[2].starts_with("https://")
            || args[5].len() != 32
            || !args[5].bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("expected HTTPS source URL and 32-digit MD5".into());
        }
        let expected_bytes: u64 = args[4].parse()?;
        let output = PathBuf::from(&args[3]);
        fs::create_dir_all(output.parent().ok_or("output needs a parent directory")?).await?;
        let client = reqwest::Client::builder()
            .user_agent("Mappa build-time source acquisition")
            .timeout(Duration::from_secs(1200))
            .build()?;
        let actual = download(
            &client,
            &args[2],
            &output,
            expected_bytes,
            Some(&args[5].to_ascii_lowercase()),
        )
        .await?;
        println!(
            "downloaded={} bytes={actual} publisher_md5={}",
            output.display(),
            args[5]
        );
        return Ok(());
    }
    let (inventory, output, class, directory) = match args.as_slice() {
        [_, inventory, output] => (inventory, output, "roads", "ROADS"),
        [_, mode, inventory, output] if mode == "--areawater" => {
            (inventory, output, "areawater", "AREAWATER")
        }
        _ => return Err("usage: mappa-map-acquire [--areawater] INVENTORY.txt OUTPUT_DIR".into()),
    };
    let output = PathBuf::from(output);
    fs::create_dir_all(&output).await?;
    let mut names = BTreeSet::new();
    for line in fs::read_to_string(inventory).await?.lines() {
        let name = line.trim();
        if name.is_empty() || name.starts_with('#') {
            continue;
        }
        if !county_name(name, class) || !names.insert(name.to_owned()) {
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
            let url = format!("https://www2.census.gov/geo/tiger/TIGER2025/{directory}/{name}");
            let path = output.join(&name);
            let bytes = match download(&client, &url, &path, MAX_SOURCE_BYTES as u64, None).await {
                Ok(bytes) => bytes,
                Err(first_error) => {
                    eprintln!(
                        "retrying invalid or unavailable publisher ZIP {name}: {first_error}"
                    );
                    download(
                        &client,
                        &format!("{url}?download=1"),
                        &path,
                        MAX_SOURCE_BYTES as u64,
                        None,
                    )
                    .await?
                }
            };
            Ok::<_, Box<dyn Error + Send + Sync>>((name, bytes))
        });
    }
    let mut files = 0usize;
    let mut total_bytes = 0u64;
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
        assert!(county_name("tl_2025_36061_roads.zip", "roads"));
        assert!(county_name("tl_2025_36061_areawater.zip", "areawater"));
        assert!(!county_name("../tl_2025_36061_roads.zip", "roads"));
        assert!(!county_name("tl_2025_36061_roads.zip/../../other", "roads"));
        assert!(!county_name("tl_2024_36061_roads.zip", "roads"));
    }
}
