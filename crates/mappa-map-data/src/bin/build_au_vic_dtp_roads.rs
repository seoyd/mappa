//! Convert the official Victoria DTP Managed Roads snapshot to MappaGeoDB.

use mappa_map_data::canonical::{
    GeoDb, SourceManifest, SourceRecord, adapt_au_vic_dtp_roads, write_geodb,
};
use sha2::{Digest, Sha256};
use std::{error::Error, fs::File, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err(
            "usage: build_au_vic_dtp_roads SOURCE.geojson DOWNLOAD_DATE OUTPUT.mgeodb OUTPUT.toml"
                .into(),
        );
    }
    let source_path = Path::new(&args[1]).canonicalize()?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut File::open(&source_path)?, &mut hasher)?;
    let source_sha256 = format!("{:x}", hasher.finalize());
    let manifest_path = Path::new(&args[4]);
    let manifest_dir = manifest_path
        .parent()
        .ok_or("manifest has no parent")?
        .canonicalize()?;
    let source_file = source_path.strip_prefix(&manifest_dir).map_or_else(
        |_| source_path.to_string_lossy().into_owned(),
        |relative| relative.to_string_lossy().into_owned(),
    );
    let source = SourceRecord {
        id: format!("au-vic-dtp-managed-roads-{}", &source_sha256[..16]),
        name: "DTP Managed Roads".into(),
        provider: "Victoria Department of Transport and Planning".into(),
        source_version: format!("sha256:{source_sha256}"),
        download_date: args[2].clone(),
        official_url: "https://opendata.transport.vic.gov.au/dataset/858f87e7-c089-4b00-9d06-ff9c4c682367/resource/7085bd59-9bed-4284-b226-703e3cfe3404/download/dtp_managed_roads.geojson".into(),
        download_page_url: "https://opendata.transport.vic.gov.au/dataset/dtp-managed-roads/resource/7085bd59-9bed-4284-b226-703e3cfe3404".into(),
        file: source_path.to_string_lossy().into_owned(),
        sha256: source_sha256,
        upstream_file: None,
        upstream_sha256: None,
        dedup_file: None,
        dedup_sha256: None,
        crs: "WGS84 geographic degrees (provider resource metadata; GeoJSON has no explicit CRS member)".into(),
        format: "GeoJSON FeatureCollection of LineString features".into(),
        coverage: "Victoria DTP-managed roads only; not the statewide complete Vicmap road network".into(),
        resolution: "Official road centreline geometry; independent position accuracy not measured".into(),
        update_frequency: "Resource snapshot metadata last updated 2026-03-10; frequency not verified".into(),
        license_id: "CC-BY-4.0".into(),
        license_url: "https://creativecommons.org/licenses/by/4.0/".into(),
        license_status: "APPROVED_WITH_ATTRIBUTION".into(),
        commercial_use: true,
        modification: true,
        redistribution: true,
        attribution_required: true,
        attribution_text: Some("Copyright (c) The State of Victoria, Department of Energy, Environment and Climate Action".into()),
        share_alike: false,
        adapter: "au-vic-dtp-managed-roads".into(),
        adapter_version: 1,
    };
    let (records, rejected) = adapt_au_vic_dtp_roads(&source)?;
    if records.is_empty() {
        return Err("Victoria source yielded no supported road segments".into());
    }
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for (feature, _) in &records {
        bounds[0] = bounds[0].min(feature.bbox.west);
        bounds[1] = bounds[1].min(feature.bbox.south);
        bounds[2] = bounds[2].max(feature.bbox.east);
        bounds[3] = bounds[3].max(feature.bbox.north);
    }
    let output_path = Path::new(&args[3]);
    write_geodb(output_path, &records)?;
    std::fs::write(
        output_path.with_extension("rejected.json"),
        serde_json::to_vec_pretty(&rejected)?,
    )?;
    let mut manifest_source = source;
    manifest_source.file = source_file;
    let manifest = SourceManifest {
        schema_version: 1,
        proof_region: "au-vic-dtp-managed-roads".into(),
        proof_bbox_wgs84: bounds,
        source: vec![manifest_source],
    };
    std::fs::write(manifest_path, toml::to_string_pretty(&manifest)?)?;
    SourceManifest::open(manifest_path)?;
    let database = GeoDb::open(output_path)?;
    if database.feature_count() != records.len() || database.provenance.len() != records.len() {
        return Err("Victoria GeoDB feature/provenance mismatch".into());
    }
    println!(
        "source_sha256={} accepted={} rejected={} bounds={bounds:?} geodb_bytes={}",
        manifest.source[0].sha256,
        records.len(),
        rejected.len(),
        std::fs::metadata(output_path)?.len()
    );
    Ok(())
}
