//! Save and audit a build-time snapshot of Queensland Roads and Tracks.

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
use tokio::{fs, task::JoinSet};

const LAYER_URL: &str = "https://spatial-gis.information.qld.gov.au/arcgis/rest/services/Transportation/RoadsAndTracks/MapServer/10";
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
    source_data_dictionary_url: &'static str,
    acquired_unix_seconds: u64,
    out_sr: u32,
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
        .ok_or("source returned no objectIds array")?;
    let mut ids = Vec::with_capacity(raw.len());
    for id in raw {
        ids.push(id.as_u64().ok_or("invalid source objectId")?);
    }
    ids.sort_unstable();
    if ids.is_empty() || ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("empty or duplicate source objectIds".into());
    }
    Ok(ids)
}

fn validate_page(bytes: &[u8], expected: &[u64]) -> TaskResult<()> {
    let value: Value = serde_json::from_slice(bytes)?;
    if value.get("type").and_then(Value::as_str) != Some("FeatureCollection") {
        return Err("source response is not GeoJSON FeatureCollection".into());
    }
    if value.get("exceededTransferLimit").and_then(Value::as_bool) == Some(true) {
        return Err("source truncated a GeoJSON page".into());
    }
    let features = value
        .get("features")
        .and_then(Value::as_array)
        .ok_or("source GeoJSON has no features array")?;
    if features.len() != expected.len() {
        return Err(format!(
            "source page has {} features, expected {}",
            features.len(),
            expected.len()
        )
        .into());
    }
    let mut actual = BTreeSet::new();
    for feature in features {
        let id = feature
            .get("properties")
            .and_then(|properties| properties.get("objectid"))
            .and_then(Value::as_u64)
            .ok_or("source feature has no numeric objectid")?;
        if !actual.insert(id) {
            return Err(format!("duplicate objectid {id} in source page").into());
        }
        if feature.get("geometry").is_none_or(Value::is_null) {
            return Err(format!("objectid {id} has no geometry").into());
        }
    }
    if actual.into_iter().ne(expected.iter().copied()) {
        return Err("source page objectIds differ from the initial ID snapshot".into());
    }
    Ok(())
}

async fn get_ids(client: &Client) -> TaskResult<Vec<u8>> {
    let response = client
        .get(format!("{LAYER_URL}/query"))
        .query(&[("where", "1=1"), ("returnIdsOnly", "true"), ("f", "json")])
        .send()
        .await?
        .error_for_status()?;
    Ok(response.bytes().await?.to_vec())
}

async fn acquire_page(
    client: Client,
    directory: Arc<std::path::PathBuf>,
    ids: Vec<u64>,
) -> TaskResult<PageRecord> {
    let first = *ids.first().ok_or("empty ID page")?;
    let last = *ids.last().ok_or("empty ID page")?;
    let file = format!("pages/{first:010}-{last:010}.geojson");
    let path = directory.join(&file);
    let bytes = match fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let query = [
                (
                    "where",
                    format!("objectid >= {first} AND objectid <= {last}"),
                ),
                ("outFields", "*".into()),
                ("returnGeometry", "true".into()),
                ("outSR", "4326".into()),
                ("f", "geojson".into()),
            ];
            let response = client
                .get(format!("{LAYER_URL}/query"))
                .query(&query)
                .send()
                .await?
                .error_for_status()?;
            let bytes = response.bytes().await?.to_vec();
            validate_page(&bytes, &ids)?;
            let temporary = path.with_extension("part");
            fs::write(&temporary, &bytes).await?;
            fs::rename(temporary, &path).await?;
            bytes
        }
        Err(error) => return Err(error.into()),
    };
    validate_page(&bytes, &ids)?;
    let record = PageRecord {
        first_objectid: first,
        last_objectid: last,
        features: ids.len(),
        bytes: bytes.len(),
        sha256: sha256(&bytes),
        file,
    };
    println!(
        "page={}..{} features={} bytes={}",
        first,
        last,
        ids.len(),
        bytes.len()
    );
    Ok(record)
}

#[tokio::main]
async fn main() -> TaskResult<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: acquire_qld_roads OUTPUT_DIRECTORY".into());
    }
    let directory = Arc::new(Path::new(&args[1]).to_path_buf());
    fs::create_dir_all(directory.join("pages")).await?;
    let client = Client::builder()
        .user_agent("Mappa build-time official-source acquisition")
        .timeout(Duration::from_secs(120))
        .build()?;
    let ids_bytes = get_ids(&client).await?;
    let ids = parse_ids(&ids_bytes)?;
    let ids_path = directory.join("ids.json");
    let ids_bytes = match fs::read(&ids_path).await {
        Ok(previous) => {
            if parse_ids(&previous)? != ids {
                return Err("source objectIds changed since the previous acquisition; start a new snapshot directory".into());
            }
            previous
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::write(&ids_path, &ids_bytes).await?;
            ids_bytes
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
            pages.push(tasks.join_next().await.ok_or("missing page task")???);
        }
    }
    while let Some(result) = tasks.join_next().await {
        pages.push(result??);
    }
    pages.sort_by_key(|page| page.first_objectid);
    if pages.iter().map(|page| page.features).sum::<usize>() != ids.len() {
        return Err("acquired page count differs from source ID snapshot".into());
    }
    if parse_ids(&get_ids(&client).await?)? != ids {
        return Err("source objectIds changed during acquisition; snapshot is not complete".into());
    }
    let snapshot = Snapshot {
        source_layer_url: LAYER_URL,
        source_dataset_url: "https://www.data.qld.gov.au/dataset/queensland-roads-and-tracks",
        source_data_dictionary_url: "https://www.qld.gov.au/__data/assets/pdf_file/0030/563457/roads-tracks-data-dictionary.pdf",
        acquired_unix_seconds: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        out_sr: 4326,
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
    fn rejects_missing_source_id_even_with_expected_feature_count() {
        let bytes = br#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"objectid":2},"geometry":{"type":"LineString","coordinates":[[153,-27],[154,-27]]}}]}"#;
        assert!(validate_page(bytes, &[1]).is_err());
    }
}
