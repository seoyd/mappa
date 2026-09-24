//! Snapshot the official Vicmap Transport road WFS for offline map building.
//! The WFS is used only by this build-time acquisition tool, never by the map client.

use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    path::Path,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{fs, task::JoinSet, time::sleep};

const WFS_URL: &str = "https://opendata.maps.vic.gov.au/geoserver/wfs";
const DATASET_URL: &str = "https://discover.data.vic.gov.au/dataset/vicmap-transport-road-line";
const TYPE_NAME: &str = "open-data-platform:tr_road";
const PAGE_SIZE: usize = 5_000;
const CONCURRENT_REQUESTS: usize = 4;

type TaskResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Serialize)]
struct PageRecord {
    first_ufi: u64,
    last_ufi: u64,
    features: usize,
    bytes: usize,
    sha256: String,
    file: String,
}

#[derive(Serialize)]
struct Snapshot {
    source_wfs_url: &'static str,
    source_dataset_url: &'static str,
    source_type_name: &'static str,
    license: &'static str,
    acquired_unix_seconds: u64,
    output_crs: &'static str,
    expected_ufi_count: usize,
    ufi_sha256: String,
    page: Vec<PageRecord>,
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn parse_collection(bytes: &[u8]) -> TaskResult<Value> {
    let value: Value = serde_json::from_slice(bytes)?;
    if value.get("type").and_then(Value::as_str) != Some("FeatureCollection") {
        return Err("WFS returned something other than a GeoJSON FeatureCollection".into());
    }
    Ok(value)
}

fn count(value: &Value, key: &str) -> TaskResult<usize> {
    Ok(value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("WFS response has no numeric {key}"))? as usize)
}

fn features(value: &Value) -> TaskResult<&Vec<Value>> {
    value
        .get("features")
        .and_then(Value::as_array)
        .ok_or_else(|| "WFS response has no features array".into())
}

fn ufi(feature: &Value) -> TaskResult<u64> {
    feature
        .get("properties")
        .and_then(|properties| properties.get("ufi"))
        .and_then(Value::as_u64)
        .ok_or_else(|| "WFS feature has no positive numeric ufi".into())
}

fn parse_id_page(bytes: &[u8], expected_total: usize, expected_len: usize) -> TaskResult<Vec<u64>> {
    let value = parse_collection(bytes)?;
    if count(&value, "numberMatched")? != expected_total {
        return Err("source feature count changed during ID acquisition".into());
    }
    let records = features(&value)?;
    if records.len() != expected_len || count(&value, "numberReturned")? != expected_len {
        return Err(format!(
            "ID page returned {}, expected {expected_len}",
            records.len()
        )
        .into());
    }
    let ids: Vec<_> = records.iter().map(ufi).collect::<TaskResult<_>>()?;
    if ids.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("ID page has duplicate or unordered UFI values".into());
    }
    if records
        .iter()
        .any(|feature| !feature.get("geometry").is_some_and(Value::is_null))
    {
        return Err("ID-only WFS query unexpectedly returned geometry".into());
    }
    Ok(ids)
}

fn validate_geometry_page(bytes: &[u8], expected: &[u64]) -> TaskResult<()> {
    let value = parse_collection(bytes)?;
    let records = features(&value)?;
    if records.len() != expected.len()
        || count(&value, "numberReturned")? != expected.len()
        || count(&value, "numberMatched")? != expected.len()
    {
        return Err(format!(
            "geometry page returned {}, expected {}",
            records.len(),
            expected.len()
        )
        .into());
    }
    for (feature, expected_ufi) in records.iter().zip(expected) {
        let actual = ufi(feature)?;
        if actual != *expected_ufi {
            return Err(
                format!("geometry UFI {actual} differs from ID snapshot {expected_ufi}").into(),
            );
        }
        let geometry_type = feature
            .get("geometry")
            .and_then(|geometry| geometry.get("type"))
            .and_then(Value::as_str);
        if !matches!(geometry_type, Some("LineString" | "MultiLineString")) {
            return Err(format!(
                "UFI {actual} has unsupported or missing geometry {geometry_type:?}"
            )
            .into());
        }
    }
    Ok(())
}

async fn query(client: &Client, extra: &[(&str, String)]) -> TaskResult<Vec<u8>> {
    let base = [
        ("service", "WFS".to_owned()),
        ("version", "2.0.0".to_owned()),
        ("request", "GetFeature".to_owned()),
        ("typeNames", TYPE_NAME.to_owned()),
        ("outputFormat", "application/json".to_owned()),
        ("srsName", "EPSG:4326".to_owned()),
        ("sortBy", "ufi".to_owned()),
    ];
    let params: Vec<_> = base.iter().chain(extra.iter()).cloned().collect();
    let mut last_error = None;
    for attempt in 0..4 {
        match client.get(WFS_URL).query(&params).send().await {
            Ok(response) => match response.error_for_status() {
                Ok(response) => match response.bytes().await {
                    Ok(bytes) => return Ok(bytes.to_vec()),
                    Err(error) => last_error = Some(error.to_string()),
                },
                Err(error) => last_error = Some(error.to_string()),
            },
            Err(error) => last_error = Some(error.to_string()),
        }
        sleep(Duration::from_secs(2_u64.pow(attempt))).await;
    }
    Err(format!(
        "WFS query failed after retries: {}",
        last_error.unwrap_or_default()
    )
    .into())
}

async fn id_page(
    client: Client,
    directory: Arc<std::path::PathBuf>,
    page_index: usize,
    expected_total: usize,
    refresh: bool,
) -> TaskResult<Vec<u64>> {
    let start = page_index * PAGE_SIZE;
    let expected_len = PAGE_SIZE.min(expected_total - start);
    let path = directory.join(format!("ids/page-{page_index:04}.json"));
    let bytes = if refresh {
        query(
            &client,
            &[
                ("propertyName", "ufi".into()),
                ("startIndex", start.to_string()),
                ("count", expected_len.to_string()),
            ],
        )
        .await?
    } else {
        match fs::read(&path).await {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let bytes = query(
                    &client,
                    &[
                        ("propertyName", "ufi".into()),
                        ("startIndex", start.to_string()),
                        ("count", expected_len.to_string()),
                    ],
                )
                .await?;
                parse_id_page(&bytes, expected_total, expected_len)?;
                let temporary = path.with_extension("part");
                fs::write(&temporary, &bytes).await?;
                fs::rename(&temporary, &path).await?;
                bytes
            }
            Err(error) => return Err(error.into()),
        }
    };
    parse_id_page(&bytes, expected_total, expected_len)
}

async fn all_ids(
    client: Client,
    directory: Arc<std::path::PathBuf>,
    expected_total: usize,
    refresh: bool,
) -> TaskResult<Vec<u64>> {
    let page_count = expected_total.div_ceil(PAGE_SIZE);
    let mut tasks = JoinSet::new();
    let mut pages = Vec::with_capacity(page_count);
    for index in 0..page_count {
        tasks.spawn(id_page(
            client.clone(),
            Arc::clone(&directory),
            index,
            expected_total,
            refresh,
        ));
        if tasks.len() == CONCURRENT_REQUESTS {
            pages.push(tasks.join_next().await.ok_or("missing ID page task")???);
        }
    }
    while let Some(result) = tasks.join_next().await {
        pages.push(result??);
    }
    // JoinSet returns completion order. Sort by the first UFI because the source sorted each page.
    pages.sort_by_key(|page| page[0]);
    let ids: Vec<_> = pages.into_iter().flatten().collect();
    if ids.len() != expected_total || ids.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("source UFI snapshot has gaps, duplicates, or reordered pages".into());
    }
    Ok(ids)
}

async fn geometry_page(
    client: Client,
    directory: Arc<std::path::PathBuf>,
    expected: Vec<u64>,
) -> TaskResult<PageRecord> {
    let first = *expected.first().ok_or("empty geometry page")?;
    let last = *expected.last().ok_or("empty geometry page")?;
    let file = format!("pages/{first:010}-{last:010}.geojson");
    let path = directory.join(&file);
    let bytes = match fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let bytes = query(
                &client,
                &[
                    ("CQL_FILTER", format!("ufi >= {first} AND ufi <= {last}")),
                    ("count", expected.len().to_string()),
                ],
            )
            .await?;
            validate_geometry_page(&bytes, &expected)?;
            let temporary = path.with_extension("part");
            fs::write(&temporary, &bytes).await?;
            fs::rename(&temporary, &path).await?;
            bytes
        }
        Err(error) => return Err(error.into()),
    };
    validate_geometry_page(&bytes, &expected)?;
    Ok(PageRecord {
        first_ufi: first,
        last_ufi: last,
        features: expected.len(),
        bytes: bytes.len(),
        sha256: sha256(&bytes),
        file,
    })
}

#[tokio::main]
async fn main() -> TaskResult<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: acquire_vicmap_roads OUTPUT_DIRECTORY".into());
    }
    let directory = Arc::new(Path::new(&args[1]).to_path_buf());
    fs::create_dir_all(directory.join("ids")).await?;
    fs::create_dir_all(directory.join("pages")).await?;
    let client = Client::builder()
        .user_agent("Mappa offline map build-time official-source acquisition")
        .timeout(Duration::from_secs(180))
        .build()?;
    let first = query(
        &client,
        &[("propertyName", "ufi".into()), ("count", "1".into())],
    )
    .await?;
    let expected_total = count(&parse_collection(&first)?, "numberMatched")?;
    if expected_total == 0 {
        return Err("Vicmap WFS returned zero roads".into());
    }
    println!("source_matched={expected_total}");
    let ids = all_ids(
        client.clone(),
        Arc::clone(&directory),
        expected_total,
        false,
    )
    .await?;
    let mut id_bytes = Vec::with_capacity(ids.len() * 10);
    for id in &ids {
        id_bytes.extend_from_slice(&id.to_le_bytes());
    }
    let id_hash = sha256(&id_bytes);
    println!("source_ufi_count={} ufi_sha256={id_hash}", ids.len());

    let mut tasks = JoinSet::new();
    let mut pages = Vec::with_capacity(ids.len().div_ceil(PAGE_SIZE));
    for chunk in ids.chunks(PAGE_SIZE) {
        tasks.spawn(geometry_page(
            client.clone(),
            Arc::clone(&directory),
            chunk.to_vec(),
        ));
        if tasks.len() == CONCURRENT_REQUESTS {
            pages.push(tasks.join_next().await.ok_or("missing geometry task")???);
            if pages.len() % 20 == 0 {
                println!(
                    "geometry_pages={}/{}",
                    pages.len(),
                    ids.len().div_ceil(PAGE_SIZE)
                );
            }
        }
    }
    while let Some(result) = tasks.join_next().await {
        pages.push(result??);
    }
    pages.sort_by_key(|page| page.first_ufi);
    if pages.iter().map(|page| page.features).sum::<usize>() != ids.len() {
        return Err("geometry page total differs from ID snapshot".into());
    }
    println!("rechecking_source_ids={}", ids.len());
    if all_ids(client.clone(), Arc::clone(&directory), expected_total, true).await? != ids {
        return Err("source UFI set changed during acquisition; start a new snapshot".into());
    }
    let snapshot = Snapshot {
        source_wfs_url: WFS_URL,
        source_dataset_url: DATASET_URL,
        source_type_name: TYPE_NAME,
        license: "CC BY 4.0",
        acquired_unix_seconds: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        output_crs: "EPSG:4326",
        expected_ufi_count: ids.len(),
        ufi_sha256: id_hash,
        page: pages,
    };
    fs::write(
        directory.join("snapshot.toml"),
        toml::to_string_pretty(&snapshot)?,
    )
    .await?;
    println!(
        "complete_pages={} features={}",
        snapshot.page.len(),
        ids.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_id_page, validate_geometry_page};

    #[test]
    fn rejects_duplicate_ufi_in_id_page() {
        let bytes = br#"{"type":"FeatureCollection","numberMatched":2,"numberReturned":2,"features":[{"properties":{"ufi":4},"geometry":null},{"properties":{"ufi":4},"geometry":null}]}"#;
        assert!(parse_id_page(bytes, 2, 2).is_err());
    }

    #[test]
    fn rejects_geometry_with_wrong_ufi() {
        let bytes = br#"{"type":"FeatureCollection","numberMatched":1,"numberReturned":1,"features":[{"properties":{"ufi":5},"geometry":{"type":"LineString","coordinates":[[1,2],[2,3]]}}]}"#;
        assert!(validate_geometry_page(bytes, &[4]).is_err());
    }
}
