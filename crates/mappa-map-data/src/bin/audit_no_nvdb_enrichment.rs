//! Confirm that class/name enrichment preserves every original NVDB road line.

use mappa_map_data::canonical::{BBox, CanonicalFeature, GeoDb};
use std::{collections::BTreeMap, error::Error, path::Path};

fn features(path: &Path) -> Result<BTreeMap<String, CanonicalFeature>, Box<dyn Error>> {
    let mut database = GeoDb::open(path)?;
    let provenance: BTreeMap<_, _> = database
        .provenance
        .iter()
        .map(|record| (record.feature_id, record.source_feature_id.clone()))
        .collect();
    let features = database.query(BBox {
        west: -180.0,
        south: -85.0,
        east: 180.0,
        north: 85.0,
    })?;
    let mut result = BTreeMap::new();
    for feature in features {
        let source_id = provenance
            .get(&feature.id)
            .ok_or("GeoDB feature has no source record")?;
        if result.insert(source_id.clone(), feature).is_some() {
            return Err("duplicate source feature ID".into());
        }
    }
    if result.len() != database.feature_count() {
        return Err("GeoDB query omitted features".into());
    }
    Ok(result)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: audit_no_nvdb_enrichment BASE.mgeodb ENRICHED.mgeodb".into());
    }
    let baseline = features(Path::new(&args[1]))?;
    let enriched = features(Path::new(&args[2]))?;
    if baseline.len() != enriched.len() || baseline.keys().ne(enriched.keys()) {
        return Err("enrichment changed the source road ID set".into());
    }
    let mut reclassified = 0;
    let mut named = 0;
    for (source_id, before) in baseline {
        let after = &enriched[&source_id];
        if before.geometry != after.geometry || before.bbox != after.bbox {
            return Err(format!("enrichment changed road geometry: {source_id}").into());
        }
        reclassified += usize::from(before.kind != after.kind);
        named += usize::from(after.name.is_some());
    }
    println!(
        "preserved_geometry={} reclassified={} named={}",
        enriched.len(),
        reclassified,
        named
    );
    Ok(())
}
