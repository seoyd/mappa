//! Reconcile official TIGER road DBF rows with GeoDB provenance and exclusions.

use flate2::read::GzDecoder;
use mappa_map_data::canonical::{GeoDb, RejectedFeature, SourceManifest};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    error::Error,
    fs::File,
    io::{Read, Seek},
    path::Path,
};
use zip::ZipArchive;

fn base_id(id: &str) -> &str {
    match id.rsplit_once(':') {
        Some((base, part)) if part.parse::<usize>().is_ok() => base,
        _ => id,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err(
            "usage: audit_tiger_road_lineage MANIFEST.toml GEO_DB.mgeodb REJECTED.json.gz".into(),
        );
    }
    let manifest = SourceManifest::open(Path::new(&args[1]))?;
    let mut raw_rows = 0usize;
    let mut source_ids = BTreeSet::new();
    for source in &manifest.source {
        if source.adapter != "us-census-tiger-roads" {
            return Err("manifest contains a non-TIGER road source".into());
        }
        source_ids.insert(source.id.as_str());
        let mut file = File::open(&source.file)?;
        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher)?;
        if format!("{:x}", hasher.finalize()) != source.sha256.to_ascii_lowercase() {
            return Err(format!("source checksum changed: {}", source.id).into());
        }
        file.rewind()?;
        let mut archive = ZipArchive::new(file)?;
        let dbf_names = archive
            .file_names()
            .filter(|name| name.ends_with(".dbf"))
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let [dbf_name] = dbf_names.as_slice() else {
            return Err(format!("expected one DBF: {}", source.id).into());
        };
        let mut dbf = archive.by_name(dbf_name)?;
        let mut header = [0u8; 8];
        dbf.read_exact(&mut header)?;
        raw_rows += u32::from_le_bytes(header[4..8].try_into()?) as usize;
    }

    let db = GeoDb::open(Path::new(&args[2]))?;
    let mut accepted_parts = BTreeSet::new();
    let mut accepted_rows = BTreeSet::new();
    for record in &db.provenance {
        if !source_ids.contains(record.source_id.as_str()) {
            return Err("GeoDB provenance contains an unexpected source".into());
        }
        let Some((raw_id, part)) = record.source_feature_id.rsplit_once(':') else {
            return Err("road provenance has no part index".into());
        };
        if raw_id.is_empty() || part.parse::<usize>().is_err() {
            return Err("road provenance has an invalid part index".into());
        }
        if !accepted_parts.insert((&record.source_id, &record.source_feature_id)) {
            return Err("duplicate accepted source part".into());
        }
        accepted_rows.insert((record.source_id.as_str(), raw_id));
    }
    let rejected: Vec<RejectedFeature> =
        serde_json::from_reader(GzDecoder::new(File::open(&args[3])?))?;
    let mut rejected_rows = BTreeSet::new();
    for record in &rejected {
        if !source_ids.contains(record.source_id.as_str()) {
            return Err("rejections contain an unexpected source".into());
        }
        rejected_rows.insert((
            record.source_id.as_str(),
            base_id(&record.source_feature_id),
        ));
    }
    let covered_rows = accepted_rows.union(&rejected_rows).count();
    if covered_rows != raw_rows {
        return Err(format!("source row accounting differs: {covered_rows} != {raw_rows}").into());
    }
    println!(
        "sources={} dbf_rows={raw_rows} accepted_raw_ids={} accepted_parts={} additional_parts={} rejected_records={} rejected_raw_ids={} covered_rows={covered_rows}",
        source_ids.len(),
        accepted_rows.len(),
        accepted_parts.len(),
        accepted_parts.len() - accepted_rows.len(),
        rejected.len(),
        rejected_rows.len(),
    );
    Ok(())
}
