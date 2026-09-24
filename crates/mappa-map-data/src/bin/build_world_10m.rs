//! Verify pinned public-domain inputs before building the offline world overview.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

type DynError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Deserialize)]
struct Manifest {
    schema_version: u32,
    name: String,
    min_zoom: u8,
    max_zoom: u8,
    max_road_rank: u8,
    max_place_rank: u8,
    river_mid_max_rank: u8,
    river_detail_max_rank: u8,
    source: Vec<Source>,
}

#[derive(Deserialize)]
struct Source {
    file: String,
    sha256: String,
    url: String,
    license: String,
    license_url: String,
    commercial_use: bool,
    modification: bool,
    redistribution: bool,
    coverage: String,
}

fn sha256_file(path: &Path) -> Result<String, DynError> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let length = file.read(&mut buffer)?;
        if length == 0 {
            break;
        }
        hasher.update(&buffer[..length]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn verify(manifest: &Manifest, source_dir: &Path) -> Result<(), DynError> {
    if manifest.schema_version != 1
        || manifest.name != "world-10m-overview"
        || manifest.min_zoom != 5
        || manifest.max_zoom < manifest.min_zoom
        || manifest.max_zoom > 8
        || manifest.river_mid_max_rank > manifest.river_detail_max_rank
    {
        return Err("invalid world build settings".into());
    }
    let expected = BTreeSet::from([
        "ne_10m_land.geojson",
        "ne_10m_lakes.geojson",
        "ne_10m_admin_0_boundary_lines_land.geojson",
        "ne_10m_populated_places.geojson",
        "ne_10m_roads.geojson",
        "ne_10m_rivers_lake_centerlines.geojson",
    ]);
    let observed: BTreeSet<_> = manifest
        .source
        .iter()
        .map(|source| source.file.as_str())
        .collect();
    if observed != expected || manifest.source.len() != expected.len() {
        return Err("world source list must include each required layer exactly once".into());
    }
    for source in &manifest.source {
        if source.license != "Natural Earth public domain"
            || source.license_url != "https://www.naturalearthdata.com/about/terms-of-use/"
            || !source.commercial_use
            || !source.modification
            || !source.redistribution
            || source.coverage.is_empty()
            || !source
                .url
                .starts_with("https://raw.githubusercontent.com/nvkelso/natural-earth-vector/")
            || source.sha256.len() != 64
        {
            return Err(format!("unreviewed source metadata: {}", source.file).into());
        }
        let file = source_dir.join(&source.file);
        if sha256_file(&file)? != source.sha256 {
            return Err(format!("source SHA-256 mismatch: {}", source.file).into());
        }
    }
    Ok(())
}

fn main() -> Result<(), DynError> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: build_world_10m MANIFEST SOURCE_DIR OUTPUT".into());
    }
    let manifest_path = PathBuf::from(&args[0]);
    let source_dir = PathBuf::from(&args[1]);
    let output = PathBuf::from(&args[2]);
    let manifest: Manifest = toml::from_str(&std::fs::read_to_string(manifest_path)?)?;
    verify(&manifest, &source_dir)?;
    let (tiles, features) = mappa_map_data::builder::build_world_detail_with_rivers(
        &source_dir,
        &output,
        manifest.min_zoom,
        manifest.max_zoom,
        manifest.max_road_rank,
        manifest.max_place_rank,
        [manifest.river_mid_max_rank, manifest.river_detail_max_rank],
    )?;
    println!(
        "verified {} sources; {tiles} tiles, {features} tile features, {} bytes",
        manifest.source.len(),
        std::fs::metadata(output)?.len()
    );
    Ok(())
}
