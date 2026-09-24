//! Build-time acquisition for public archives whose server rejects whole-file GETs.

use reqwest::header::{CONTENT_RANGE, RANGE};
use sha2::{Digest, Sha256};
use std::{error::Error, fs::File, io::Read, path::PathBuf, time::Duration};
use tokio::{fs, io::AsyncWriteExt, sync::Semaphore, task::JoinSet};

const CHUNK_BYTES: u64 = 2 * 1024 * 1024;
const MAX_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const CONCURRENT_REQUESTS: usize = 2;

async fn fetch_chunk(
    client: &reqwest::Client,
    url: &str,
    directory: &std::path::Path,
    start: u64,
    end: u64,
    total: u64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let path = directory.join(format!("{start:012}-{end:012}.chunk"));
    let expected = end - start + 1;
    if fs::metadata(&path)
        .await
        .is_ok_and(|metadata| metadata.len() == expected)
    {
        return Ok(());
    }
    let temporary = path.with_extension("part");
    for attempt in 0..5 {
        let result: Result<(), Box<dyn Error + Send + Sync>> = async {
            let mut response = client
                .get(url)
                .header(RANGE, format!("bytes={start}-{end}"))
                .send()
                .await?
                .error_for_status()?;
            let actual_range = response
                .headers()
                .get(CONTENT_RANGE)
                .ok_or("range response has no Content-Range")?
                .to_str()?;
            let wanted_range = format!("bytes {start}-{end}/{total}");
            if response.status() != reqwest::StatusCode::PARTIAL_CONTENT
                || actual_range != wanted_range
            {
                return Err(format!("unexpected range: {actual_range}").into());
            }
            let mut output = fs::File::create(&temporary).await?;
            let mut written = 0u64;
            while let Some(bytes) = response.chunk().await? {
                written += bytes.len() as u64;
                if written > expected {
                    return Err("range response exceeds requested bytes".into());
                }
                output.write_all(&bytes).await?;
            }
            output.flush().await?;
            if written != expected {
                return Err(format!("short range: {written} != {expected}").into());
            }
            fs::rename(&temporary, &path).await?;
            Ok(())
        }
        .await;
        match result {
            Ok(()) => return Ok(()),
            Err(error) if attempt == 4 => return Err(error),
            Err(error) => {
                eprintln!("range={start}-{end} attempt={} error={error}", attempt + 1);
                tokio::time::sleep(Duration::from_secs(2 << attempt)).await;
            }
        }
    }
    unreachable!()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 || !args[1].starts_with("https://") {
        return Err("usage: ranged HTTPS_URL OUTPUT EXPECTED_BYTES".into());
    }
    let total: u64 = args[3].parse()?;
    if total == 0 || total > MAX_BYTES {
        return Err("expected bytes outside build-time archive limit".into());
    }
    let output = PathBuf::from(&args[2]);
    if fs::metadata(&output)
        .await
        .is_ok_and(|metadata| metadata.len() == total)
    {
        return Err("output already exists; remove it after verifying its checksum".into());
    }
    let parent = output.parent().ok_or("output needs a parent directory")?;
    fs::create_dir_all(parent).await?;
    let directory = output.with_extension("parts");
    fs::create_dir_all(&directory).await?;
    let client = reqwest::Client::builder()
        .user_agent("Mappa build-time official archive acquisition")
        .timeout(Duration::from_secs(90))
        .build()?;
    let permits = std::sync::Arc::new(Semaphore::new(CONCURRENT_REQUESTS));
    let mut tasks = JoinSet::new();
    for start in (0..total).step_by(CHUNK_BYTES as usize) {
        let permit = permits.clone().acquire_owned().await?;
        let client = client.clone();
        let url = args[1].clone();
        let directory = directory.clone();
        let end = (start + CHUNK_BYTES).min(total) - 1;
        tasks.spawn(async move {
            let _permit = permit;
            fetch_chunk(&client, &url, &directory, start, end, total).await?;
            Ok::<_, Box<dyn Error + Send + Sync>>(start)
        });
        // The publisher documents a one-request-per-second rate on this endpoint.
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    let mut completed = 0usize;
    while let Some(result) = tasks.join_next().await {
        result??;
        completed += 1;
        if completed.is_multiple_of(10) || completed as u64 == total.div_ceil(CHUNK_BYTES) {
            eprintln!(
                "downloaded_chunks={completed}/{}",
                total.div_ceil(CHUNK_BYTES)
            );
        }
    }
    let temporary = output.with_extension("assembling");
    let mut assembled = File::create(&temporary)?;
    let mut hash = Sha256::new();
    let mut bytes = 0u64;
    for start in (0..total).step_by(CHUNK_BYTES as usize) {
        let end = (start + CHUNK_BYTES).min(total) - 1;
        let path = directory.join(format!("{start:012}-{end:012}.chunk"));
        let mut input = File::open(&path)?;
        if input.metadata()?.len() != end - start + 1 {
            return Err(format!("incomplete chunk: {}", path.display()).into());
        }
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let read = input.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hash.update(&buffer[..read]);
            std::io::Write::write_all(&mut assembled, &buffer[..read])?;
            bytes += read as u64;
        }
    }
    assembled.sync_all()?;
    if bytes != total {
        return Err("assembled archive differs from publisher byte count".into());
    }
    std::fs::rename(&temporary, &output)?;
    println!(
        "downloaded={} bytes={bytes} sha256={:x}",
        output.display(),
        hash.finalize()
    );
    Ok(())
}
