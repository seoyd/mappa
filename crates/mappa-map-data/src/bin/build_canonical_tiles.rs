use mappa_map_data::{
    builder::canonical_tiles::build_canonical_tiles,
    canonical::{BBox, GeoDb, SourceManifest},
};
use std::{error::Error, path::Path};

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        return Err(
            "usage: build_canonical_tiles data/sources.toml INPUT.mgeodb OUTPUT.pmtiles".into(),
        );
    }
    let manifest = SourceManifest::open(Path::new(&args[1]))?;
    let database = GeoDb::open(Path::new(&args[2]))?;
    let mut expected_sources: Vec<_> = manifest
        .source
        .iter()
        .map(|source| source.id.clone())
        .collect();
    expected_sources.sort();
    if database.sources != expected_sources {
        return Err("GeoDB source list does not match the approved manifest".into());
    }
    let providers = manifest
        .source
        .iter()
        .map(|source| source.provider.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(", ");
    let worldcover_present = manifest
        .source
        .iter()
        .any(|source| source.adapter.starts_with("esa-worldcover-"));
    let worldcover_credit = if worldcover_present {
        " · © ESA WorldCover project 2021 / Contains modified Copernicus Sentinel data (2021) processed by ESA WorldCover consortium"
    } else {
        ""
    };
    let attribution = format!(
        "원본: {}{} · Mappa GeoDB · 미구축 지역 공백",
        providers, worldcover_credit
    );
    let [west, south, east, north] = manifest.proof_bbox_wgs84;
    let (tiles, features) = build_canonical_tiles(
        Path::new(&args[2]),
        Path::new(&args[3]),
        BBox {
            west,
            south,
            east,
            north,
        },
        10,
        15,
        &attribution,
    )?;
    println!(
        "region={} tiles={} encoded_features={} pmtiles_bytes={}",
        manifest.proof_region,
        tiles,
        features,
        std::fs::metadata(&args[3])?.len()
    );
    Ok(())
}
