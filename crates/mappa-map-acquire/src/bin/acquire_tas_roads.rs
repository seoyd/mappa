//! Capture an audited, build-time snapshot of Tasmania's LIST Transport Segments.

use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    error::Error,
    path::Path,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{fs, task::JoinSet, time::sleep};

const LAYER_URL: &str =
    "https://services.thelist.tas.gov.au/arcgis/rest/services/Public/OpenDataWFS/MapServer/42";
const DATASET_URL: &str = "https://www.thelist.tas.gov.au/app/content/data/geo-meta-data-record?detailRecordUID=1ab7e34f-811c-4521-a549-212f295acc97";
const PAGE_SIZE: usize = 1_000;
const CONCURRENT_REQUESTS: usize = 4;

type TaskResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Serialize)]
struct PageRecord {
    first_objectid: u64,
    last_objectid: u64,
    features: usize,
    bytes: usize,
    sha256: String,
    file: String,
}

#[derive(Serialize)]
struct Snapshot {
    source_layer_url: &'static str,
    source_dataset_url: &'static str,
    license_url: &'static str,
    acquired_unix_seconds: u64,
    source_sr: u32,
    output_sr: u32,
    expected_objectids: usize,
    ids_sha256: String,
    page: Vec<PageRecord>,
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn parse_ids(bytes: &[u8]) -> TaskResult<Vec<u64>> {
    let value: Value = serde_json::from_slice(bytes)?;
    let raw = value
        .get("objectIds")
        .and_then(Value::as_array)
        .ok_or("LIST returned no objectIds array")?;
    let mut ids = Vec::with_capacity(raw.len());
    for id in raw {
        ids.push(id.as_u64().ok_or("invalid LIST objectId")?);
    }
    ids.sort_unstable();
    if ids.is_empty() || ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("empty or duplicate LIST objectIds".into());
    }
    Ok(ids)
}

fn validate_page(bytes: &[u8], expected: &[u64]) -> TaskResult<()> {
    let value: Value = serde_json::from_slice(bytes)?;
    if value.get("type").and_then(Value::as_str) != Some("FeatureCollection") {
        return Err("LIST response is not a GeoJSON FeatureCollection".into());
    }
    let features = value
        .get("features")
        .and_then(Value::as_array)
        .ok_or("LIST GeoJSON has no features array")?;
    if features.len() != expected.len() {
        return Err(format!(
            "LIST page has {} features, expected {}",
            features.len(),
            expected.len()
        )
        .into());
    }
    let mut actual = BTreeSet::new();
    for feature in features {
        let id = feature
            .get("properties")
            .and_then(|properties| properties.get("OBJECTID"))
            .and_then(Value::as_u64)
            .ok_or("LIST feature has no numeric OBJECTID")?;
        if !actual.insert(id) {
            return Err(format!("duplicate LIST OBJECTID {id}").into());
        }
        let geometry_type = feature
            .get("geometry")
            .and_then(|geometry| geometry.get("type"))
            .and_then(Value::as_str);
        if !matches!(geometry_type, Some("LineString" | "MultiLineString")) {
            return Err(
                format!("LIST OBJECTID {id} has unsupported geometry {geometry_type:?}").into(),
            );
        }
    }
    if actual.into_iter().ne(expected.iter().copied()) {
        return Err("LIST page objectIds differ from initial ID snapshot".into());
    }
    Ok(())
}

async fn query(client: &Client, params: &[(&str, String)]) -> TaskResult<Vec<u8>> {
    let mut last_error = None;
    for attempt in 0..4 {
        match client
            .get(format!("{LAYER_URL}/query"))
            .query(params)
            .send()
            .await
        {
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
        "LIST query failed after retries: {}",
        last_error.unwrap_or_default()
    )
    .into())
}

async fn get_ids(client: &Client) -> TaskResult<Vec<u8>> {
    query(
        client,
        &[
            ("where", "1=1".into()),
            ("returnIdsOnly", "true".into()),
            ("f", "json".into()),
        ],
    )
    .await
}

async fn acquire_page(
    client: Client,
    directory: Arc<std::path::PathBuf>,
    ids: Vec<u64>,
) -> TaskResult<PageRecord> {
    let first = *ids.first().ok_or("empty LIST ID page")?;
    let last = *ids.last().ok_or("empty LIST ID page")?;
    let file = format!("pages/{first:010}-{last:010}.geojson");
    let path = directory.join(&file);
    let bytes = match fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let bytes = query(
                &client,
                &[
                    (
                        "where",
                        format!("OBJECTID >= {first} AND OBJECTID <= {last}"),
                    ),
                    ("outFields", "*".into()),
                    ("returnGeometry", "true".into()),
                    ("outSR", "4326".into()),
                    ("f", "geojson".into()),
                ],
            )
            .await?;
            validate_page(&bytes, &ids)?;
            let temporary = path.with_extension("part");
            fs::write(&temporary, &bytes).await?;
            fs::rename(&temporary, &path).await?;
            bytes
        }
        Err(error) => return Err(error.into()),
    };
    validate_page(&bytes, &ids)?;
    Ok(PageRecord {
        first_objectid: first,
        last_objectid: last,
        features: ids.len(),
        bytes: bytes.len(),
        sha256: sha256(&bytes),
        file,
    })
}

#[tokio::main]
async fn main() -> TaskResult<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: acquire_tas_roads OUTPUT_DIRECTORY".into());
    }
    let directory = Arc::new(Path::new(&args[1]).to_path_buf());
    fs::create_dir_all(directory.join("pages")).await?;
    let client = Client::builder()
        .user_agent("Mappa offline map build-time official-source acquisition")
        .timeout(Duration::from_secs(120))
        .build()?;
    let fresh_ids = get_ids(&client).await?;
    let ids = parse_ids(&fresh_ids)?;
    let ids_path = directory.join("ids.json");
    let ids_bytes = match fs::read(&ids_path).await {
        Ok(previous) => {
            if parse_ids(&previous)? != ids {
                return Err(
                    "LIST objectIds changed since prior acquisition; use a new snapshot directory"
                        .into(),
                );
            }
            previous
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::write(&ids_path, &fresh_ids).await?;
            fresh_ids
        }
        Err(error) => return Err(error.into()),
    };
    println!(
        "source_objectids={} ids_sha256={}",
        ids.len(),
        sha256(&ids_bytes)
    );
    let mut tasks = JoinSet::new();
    let mut pages = Vec::new();
    for chunk in ids.chunks(PAGE_SIZE) {
        tasks.spawn(acquire_page(
            client.clone(),
            Arc::clone(&directory),
            chunk.to_vec(),
        ));
        if tasks.len() == CONCURRENT_REQUESTS {
            pages.push(
                tasks
                    .join_next()
                    .await
                    .ok_or("missing LIST page task")???,
            );
        }
    }
    while let Some(result) = tasks.join_next().await {
        pages.push(result??);
    }
    pages.sort_by_key(|page| page.first_objectid);
    if pages.iter().map(|page| page.features).sum::<usize>() != ids.len() {
        return Err("LIST page count differs from source ID snapshot".into());
    }
    if parse_ids(&get_ids(&client).await?)? != ids {
        return Err("LIST objectIds changed during acquisition".into());
    }
    let snapshot = Snapshot {
        source_layer_url: LAYER_URL,
        source_dataset_url: DATASET_URL,
        license_url: "https://creativecommons.org/licenses/by/3.0/au/",
        acquired_unix_seconds: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        source_sr: 3857,
        output_sr: 4326,
        expected_objectids: ids.len(),
        ids_sha256: sha256(&ids_bytes),
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
    use super::validate_page;

    #[test]
    fn rejects_page_with_wrong_objectid() {
        let bytes = br#"{"type":"FeatureCollection","features":[{"properties":{"OBJECTID":2},"geometry":{"type":"LineString","coordinates":[[147,-42],[147,-43]]}}]}"#;
        assert!(validate_page(bytes, &[1]).is_err());
    }
}
