//! Count accepted and rejected source features without inferring geographic completeness.

use flate2::read::GzDecoder;
use mappa_map_data::canonical::{GeoDb, RejectedFeature, SourceManifest};
use std::{collections::BTreeMap, error::Error, io::Read, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err(
            "usage: audit_canonical_sources MANIFEST.toml FILE.mgeodb REJECTED.json".into(),
        );
    }
    let manifest = SourceManifest::open(Path::new(&args[1]))?;
    let db = GeoDb::open(Path::new(&args[2]))?;
    let mut reject_bytes = Vec::new();
    if Path::new(&args[3])
        .extension()
        .is_some_and(|extension| extension == "gz")
    {
        GzDecoder::new(std::fs::File::open(&args[3])?).read_to_end(&mut reject_bytes)?;
    } else {
        reject_bytes = std::fs::read(&args[3])?;
    }
    let rejected: Vec<RejectedFeature> = serde_json::from_slice(&reject_bytes)?;
    let mut counts = manifest
        .source
        .iter()
        .map(|source| (source.id.clone(), (0usize, 0usize)))
        .collect::<BTreeMap<_, _>>();
    for feature in &db.provenance {
        counts
            .get_mut(&feature.source_id)
            .ok_or("GeoDB contains an unapproved source")?
            .0 += 1;
    }
    for feature in &rejected {
        counts
            .get_mut(&feature.source_id)
            .ok_or("rejection log contains an unapproved source")?
            .1 += 1;
    }
    if db.feature_count() != db.provenance.len() {
        return Err("GeoDB feature and provenance counts differ".into());
    }
    let expected: Vec<_> = counts
        .iter()
        .filter(|(_, (accepted, _))| *accepted > 0)
        .map(|(id, _)| id.clone())
        .collect();
    if db.sources != expected {
        return Err("GeoDB source index and observed provenance differ".into());
    }
    println!("source_id,accepted_features,rejected_records");
    for (id, (accepted, invalid)) in &counts {
        println!("{id},{accepted},{invalid}");
    }
    eprintln!(
        "sources={} contributing={} accepted_features={} rejected_records={}",
        counts.len(),
        counts
            .values()
            .filter(|(accepted, _)| *accepted > 0)
            .count(),
        db.feature_count(),
        rejected.len()
    );
    Ok(())
}
