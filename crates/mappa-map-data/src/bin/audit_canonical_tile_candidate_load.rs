//! Diagnose the number of tile bounding boxes visited for each canonical feature.

use mappa_map_core::project;
use mappa_map_data::canonical::{BBox, FeatureKind, GeoDb};
use std::{collections::HashMap, error::Error, path::Path};

type DynError = Box<dyn Error + Send + Sync>;

fn main() -> Result<(), DynError> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: audit_canonical_tile_candidate_load INPUT.mgeodb ZOOM".into());
    }
    let zoom: u8 = args[2].parse()?;
    if zoom > 15 {
        return Err("zoom exceeds canonical tile limit".into());
    }
    let mut database = GeoDb::open(Path::new(&args[1]))?;
    let lineage = database
        .provenance
        .iter()
        .map(|item| {
            (
                item.feature_id,
                (item.source_id.clone(), item.source_feature_id.clone()),
            )
        })
        .collect::<HashMap<_, _>>();
    let features = database.query(BBox {
        west: -180.0,
        south: -85.051_128_78,
        east: 180.0,
        north: 85.051_128_78,
    })?;
    let n = f64::from(1u32 << zoom);
    let mut candidates = Vec::new();
    let mut total = 0u64;
    for feature in features {
        if feature.kind != FeatureKind::Water
            || !(feature.min_zoom..=feature.max_zoom).contains(&zoom)
        {
            continue;
        }
        let northwest = project(feature.bbox.west, feature.bbox.north)?;
        let southeast = project(feature.bbox.east, feature.bbox.south)?;
        let x0 = (northwest.x * n).floor() as u64;
        let x1 = (southeast.x * n).ceil() as u64;
        let y0 = (northwest.y * n).floor() as u64;
        let y1 = (southeast.y * n).ceil() as u64;
        let count = (x1 - x0) * (y1 - y0);
        total += count;
        candidates.push((count, feature.id, feature.bbox));
    }
    candidates.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    println!(
        "zoom={zoom} water_features={} bounding_box_tile_candidates={total}",
        candidates.len()
    );
    for (count, id, bbox) in candidates.iter().take(10) {
        println!(
            "candidate_tiles={count} source_id={} source_feature_id={} bounds=[{},{},{},{}]",
            lineage
                .get(id)
                .map(|item| item.0.as_str())
                .unwrap_or("unknown"),
            lineage
                .get(id)
                .map(|item| item.1.as_str())
                .unwrap_or("unknown"),
            bbox.west,
            bbox.south,
            bbox.east,
            bbox.north
        );
    }
    Ok(())
}
