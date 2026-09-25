//! Capture a paged, build-time snapshot of Norway's official NVDB V4 road links.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, error::Error, path::Path, time::Duration};
use tokio::{fs, time::sleep};

const ENDPOINT: &str = "https://nvdbapiles.atlas.vegvesen.no/vegnett/api/v4/veglenkesekvenser";
const SEGMENTED_ENDPOINT: &str =
    "https://nvdbapiles.atlas.vegvesen.no/vegnett/api/v4/veglenkesekvenser/segmentert";
const PAGE_SIZE: usize = 1_000;

#[derive(Serialize)]
struct Page {
    file: String,
    sha256: String,
    first_id: String,
    last_id: String,
    objects: usize,
}

#[derive(Serialize)]
struct Snapshot {
    source_url: &'static str,
    license_url: &'static str,
    municipality: u32,
    expected_objects: usize,
    pages: Vec<Page>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
struct CaptureRequest {
    source_url: String,
    municipality: u32,
}

async fn bind_capture_directory(
    directory: &Path,
    endpoint: &str,
    municipality: u32,
) -> Result<(), Box<dyn Error>> {
    let requested = CaptureRequest {
        source_url: endpoint.to_owned(),
        municipality,
    };
    let binding = directory.join("request.toml");
    let existing = match fs::read_to_string(&binding).await {
        Ok(text) => Some(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            match fs::read_to_string(directory.join("snapshot.toml")).await {
                Ok(text) => Some(text),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            }
        }
        Err(error) => return Err(error.into()),
    };
    if let Some(existing) = existing {
        let existing: CaptureRequest = toml::from_str(&existing)?;
        if existing != requested {
            return Err(
                "NVDB capture directory belongs to another municipality or endpoint".into(),
            );
        }
    } else if fs::try_exists(directory.join("pages/00000.json")).await? {
        return Err("NVDB pages have no municipality and endpoint binding".into());
    }
    fs::write(binding, toml::to_string_pretty(&requested)?).await?;
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn examine_page(
    bytes: &[u8],
    expected_total: Option<usize>,
    seen: &mut BTreeSet<String>,
    segmented: bool,
) -> Result<(usize, Option<String>, Page), Box<dyn Error>> {
    let value: Value = serde_json::from_slice(bytes)?;
    let metadata = value.get("metadata").ok_or("NVDB page has no metadata")?;
    let total = metadata
        .get("antall")
        .and_then(Value::as_u64)
        .ok_or("NVDB page has no total count")? as usize;
    if total == 0 || expected_total.is_some_and(|expected| expected != total) {
        return Err("NVDB municipality total changed during acquisition".into());
    }
    let objects = value
        .get("objekter")
        .and_then(Value::as_array)
        .ok_or("NVDB page has no road-link sequences")?;
    if objects.is_empty()
        || metadata.get("returnert").and_then(Value::as_u64) != Some(objects.len() as u64)
        || objects.len() > PAGE_SIZE
    {
        return Err("NVDB page object count mismatch".into());
    }
    let mut ids = Vec::with_capacity(objects.len());
    for object in objects {
        let id = if segmented {
            object
                .get("referanse")
                .and_then(Value::as_str)
                .ok_or("NVDB segment has no reference ID")?
                .to_owned()
        } else {
            object
                .get("veglenkesekvensid")
                .and_then(Value::as_u64)
                .ok_or("NVDB object has no road-link sequence ID")?
                .to_string()
        };
        if id.is_empty() || !seen.insert(id.clone()) {
            return Err(format!("duplicate or empty NVDB road-link ID: {id}").into());
        }
        if segmented {
            if !object.get("geometri").is_some_and(Value::is_object) {
                return Err(format!("NVDB segment {id} has no geometry").into());
            }
        } else if !object.get("veglenker").is_some_and(Value::is_array) {
            return Err(format!("NVDB sequence {id} has no road links").into());
        }
        ids.push(id);
    }
    let reported_next = metadata
        .get("neste")
        .and_then(|next| next.get("start"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    if seen.len() > total || (reported_next.is_none() && seen.len() != total) {
        return Err("NVDB ended before expected object count".into());
    }
    // NVDB V4 reports a next cursor even on a short final page. The audited
    // count and unique IDs, rather than cursor presence, determine completion.
    let next = if seen.len() == total {
        None
    } else {
        reported_next
    };
    Ok((
        total,
        next,
        Page {
            file: String::new(),
            sha256: sha256(bytes),
            first_id: ids[0].clone(),
            last_id: ids.last().expect("nonempty page").clone(),
            objects: ids.len(),
        },
    ))
}

async fn request(
    client: &Client,
    endpoint: &str,
    municipality: u32,
    start: Option<&str>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut error = String::new();
    for attempt in 0..4 {
        let mut query = vec![
            ("kommune", municipality.to_string()),
            ("antall", PAGE_SIZE.to_string()),
            ("inkluderAntall", "true".to_owned()),
        ];
        if let Some(start) = start {
            query.push(("start", start.to_owned()));
        }
        match client.get(endpoint).query(&query).send().await {
            Ok(response) => match response.error_for_status() {
                Ok(response) => match response.bytes().await {
                    Ok(bytes) => return Ok(bytes.to_vec()),
                    Err(failure) => error = failure.to_string(),
                },
                Err(failure) => error = failure.to_string(),
            },
            Err(failure) => error = failure.to_string(),
        }
        sleep(Duration::from_secs(1 << attempt)).await;
    }
    Err(format!("NVDB page request failed: {error}").into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if !(args.len() == 3 || (args.len() == 4 && args[3] == "segmentert")) {
        return Err(
            "usage: acquire_no_nvdb_roads MUNICIPALITY_CODE OUTPUT_DIRECTORY [segmentert]".into(),
        );
    }
    let municipality: u32 = args[1].parse()?;
    if municipality == 0 {
        return Err("municipality code must be positive".into());
    }
    let directory = Path::new(&args[2]);
    let segmented = args.len() == 4;
    let endpoint = if segmented {
        SEGMENTED_ENDPOINT
    } else {
        ENDPOINT
    };
    fs::create_dir_all(directory.join("pages")).await?;
    bind_capture_directory(directory, endpoint, municipality).await?;
    let client = Client::builder()
        .user_agent("Mappa official-source offline map builder")
        .timeout(Duration::from_secs(120))
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert("X-Client", "MappaOfflineMapBuilder".parse()?);
            headers
        })
        .build()?;
    let mut seen = BTreeSet::new();
    let mut expected_total = None;
    let mut start: Option<String> = None;
    let mut pages = Vec::new();
    loop {
        let file = format!("pages/{:05}.json", pages.len());
        let path = directory.join(&file);
        let bytes = match fs::read(&path).await {
            Ok(bytes) => bytes,
            Err(failure) if failure.kind() == std::io::ErrorKind::NotFound => {
                let bytes = request(&client, endpoint, municipality, start.as_deref()).await?;
                let temporary = path.with_extension("part");
                fs::write(&temporary, &bytes).await?;
                fs::rename(temporary, &path).await?;
                bytes
            }
            Err(failure) => return Err(failure.into()),
        };
        let (total, next, mut page) = examine_page(&bytes, expected_total, &mut seen, segmented)?;
        expected_total = Some(total);
        page.file = file;
        println!(
            "page={} objects={} captured={}/{} sha256={}",
            pages.len(),
            page.objects,
            seen.len(),
            total,
            page.sha256
        );
        pages.push(page);
        if next.is_none() {
            break;
        }
        start = next;
    }
    let snapshot = Snapshot {
        source_url: endpoint,
        license_url: "https://data.norge.no/nlod/no/1.0",
        municipality,
        expected_objects: expected_total.expect("at least one page"),
        pages,
    };
    fs::write(
        directory.join("snapshot.toml"),
        toml::to_string_pretty(&snapshot)?,
    )
    .await?;
    println!(
        "complete municipality={} objects={} pages={}",
        municipality,
        seen.len(),
        snapshot.pages.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ENDPOINT, SEGMENTED_ENDPOINT, bind_capture_directory, examine_page};
    use std::collections::BTreeSet;

    #[test]
    fn rejects_duplicate_sequence_across_pages() {
        let page = br#"{"objekter":[{"veglenkesekvensid":1,"veglenker":[]}],"metadata":{"antall":2,"returnert":1,"neste":{"start":"1"}}}"#;
        let mut seen = BTreeSet::new();
        examine_page(page, None, &mut seen, false).unwrap();
        assert!(examine_page(page, Some(2), &mut seen, false).is_err());
    }

    #[test]
    fn completes_when_final_page_still_reports_cursor() {
        let page = br#"{"objekter":[{"veglenkesekvensid":1,"veglenker":[]}],"metadata":{"antall":1,"returnert":1,"neste":{"start":"1"}}}"#;
        let (_, next, _) = examine_page(page, None, &mut BTreeSet::new(), false).unwrap();
        assert!(next.is_none());
    }

    #[test]
    fn segmented_ids_include_each_distinct_segment() {
        let page = br#"{"objekter":[{"referanse":"10-1-1","geometri":{}},{"referanse":"10-1-2","geometri":{}}],"metadata":{"antall":2,"returnert":2,"neste":{"start":"10:1:2"}}}"#;
        let (_, next, result) = examine_page(page, None, &mut BTreeSet::new(), true).unwrap();
        assert_eq!(result.objects, 2);
        assert!(next.is_none());
    }

    #[tokio::test]
    async fn resume_rejects_different_municipality_or_endpoint() {
        let directory = tempfile::tempdir().unwrap();
        bind_capture_directory(directory.path(), ENDPOINT, 5001)
            .await
            .unwrap();
        assert!(
            bind_capture_directory(directory.path(), ENDPOINT, 5002)
                .await
                .is_err()
        );
        assert!(
            bind_capture_directory(directory.path(), SEGMENTED_ENDPOINT, 5001)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn resume_rejects_legacy_pages_without_source_binding() {
        let directory = tempfile::tempdir().unwrap();
        tokio::fs::create_dir_all(directory.path().join("pages"))
            .await
            .unwrap();
        tokio::fs::write(directory.path().join("pages/00000.json"), b"{}")
            .await
            .unwrap();
        assert!(
            bind_capture_directory(directory.path(), ENDPOINT, 5001)
                .await
                .is_err()
        );
    }
}
