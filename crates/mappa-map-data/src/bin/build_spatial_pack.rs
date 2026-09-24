//! Compile the approved Naju GeoDB to a separate experimental MSP artifact.
use mappa_map_data::{
    canonical::{BBox, GeoDb, SourceManifest},
    spatial_pack,
};
use std::{error::Error, path::Path};

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: build_spatial_pack MANIFEST INPUT.mgeodb OUTPUT.msp".into());
    }
    let manifest = SourceManifest::open(Path::new(&args[0]))?;
    let database = GeoDb::open(Path::new(&args[1]))?;
    let mut sources: Vec<_> = manifest.source.iter().map(|s| s.id.clone()).collect();
    sources.sort();
    if database.sources != sources {
        return Err("GeoDB source list does not match approved manifest".into());
    }
    let [west, south, east, north] = manifest.proof_bbox_wgs84;
    let providers = manifest
        .source
        .iter()
        .map(|source| source.provider.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(", ");
    let credits = manifest
        .source
        .iter()
        .filter_map(|source| source.attribution_text.as_deref())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(" · ");
    let attribution = if credits.is_empty() {
        format!("원본: {providers} · Mappa GeoDB · 미구축 지역 공백")
    } else {
        format!("원본: {providers} · {credits} · Mappa GeoDB · 미구축 지역 공백")
    };
    let (features, cells) = spatial_pack::build(
        Path::new(&args[1]),
        Path::new(&args[2]),
        BBox {
            west,
            south,
            east,
            north,
        },
        &attribution,
    )?;
    println!(
        "region={} features={} cells={} bytes={}",
        manifest.proof_region,
        features,
        cells,
        std::fs::metadata(&args[2])?.len()
    );
    Ok(())
}
