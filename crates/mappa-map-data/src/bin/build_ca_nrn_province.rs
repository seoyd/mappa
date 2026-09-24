//! Build one official Canadian NRN province's road segments into MappaGeoDB.

use flate2::{Compression, write::GzEncoder};
use mappa_map_data::canonical::{
    GeoDb, SourceManifest, SourceRecord, adapt_ca_nrn_roads, write_geodb,
};
use sha2::{Digest, Sha256};
use std::{error::Error, fs::File, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 7 {
        return Err("usage: build_ca_nrn_province SOURCE.zip PROVINCE VERSION DOWNLOAD_DATE OUTPUT.mgeodb OUTPUT.toml".into());
    }
    let source_path = Path::new(&args[1]);
    let province = &args[2];
    if province.len() != 2 || !province.bytes().all(|byte| byte.is_ascii_uppercase()) {
        return Err("province code must be two uppercase letters".into());
    }
    let lower = province.to_ascii_lowercase();
    let version = &args[3];
    if !version
        .chars()
        .all(|character| character.is_ascii_digit() || character == '.')
        || !version.contains('.')
    {
        return Err("NRN source version must be numeric dotted notation".into());
    }
    let canonical_source = source_path.canonicalize()?;
    let mut source_file_reader = File::open(&canonical_source)?;
    let mut source_hasher = Sha256::new();
    std::io::copy(&mut source_file_reader, &mut source_hasher)?;
    let source_sha256 = format!("{:x}", source_hasher.finalize());
    let manifest_path = Path::new(&args[6]);
    let manifest_dir = manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .canonicalize()?;
    let source_file = canonical_source.strip_prefix(&manifest_dir).map_or_else(
        |_| canonical_source.to_string_lossy().into_owned(),
        |relative| relative.to_string_lossy().into_owned(),
    );
    let source = SourceRecord {
        id: format!("ca-nrn-{province}-{version}"),
        name: "National Road Network Road Segment".into(),
        provider: "Statistics Canada / National Road Network".into(),
        source_version: version.into(),
        download_date: args[4].clone(),
        official_url: format!("https://geo.statcan.gc.ca/nrn_rrn/{lower}/nrn_rrn_{lower}_SHAPE.zip"),
        download_page_url: "https://open.canada.ca/data/en/dataset/3d282116-e556-400c-9306-ca1a3cada77f".into(),
        file: canonical_source.to_string_lossy().into_owned(),
        sha256: source_sha256,
        upstream_file: None,
        upstream_sha256: None,
        dedup_file: None,
        dedup_sha256: None,
        crs: "NAD83(CSRS) geographic degrees; numeric degrees retained; WGS84 shift not independently verified".into(),
        format: "Province ZIP / English ROADSEG Shapefile Polyline".into(),
        coverage: format!("NRN {province} provincial Road Segment input"),
        resolution: "Official NRN source geometry; no independent field accuracy test".into(),
        update_frequency: "Official dataset catalogue: annually".into(),
        license_id: "OGL-CA".into(),
        license_url: "https://open.canada.ca/en/open-government-licence-canada".into(),
        license_status: "APPROVED_WITH_ATTRIBUTION".into(),
        commercial_use: true,
        modification: true,
        redistribution: true,
        attribution_required: true,
        attribution_text: Some("Contains information licensed under the Open Government Licence – Canada.".into()),
        share_alike: false,
        adapter: "ca-nrn-roadseg".into(),
        adapter_version: 1,
    };
    let (records, rejected) = adapt_ca_nrn_roads(&source, province)?;
    if records.is_empty() {
        return Err("no NRN roads accepted".into());
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
    let mut manifest_source = source;
    manifest_source.file = source_file;
    let manifest = SourceManifest {
        schema_version: 1,
        proof_region: format!("ca-nrn-{province}"),
        proof_bbox_wgs84: bounds,
        source: vec![manifest_source],
    };
    let geodb_path = Path::new(&args[5]);
    write_geodb(geodb_path, &records)?;
    std::fs::write(manifest_path, toml::to_string_pretty(&manifest)?)?;
    let mut reject_writer = GzEncoder::new(
        File::create(geodb_path.with_extension("rejected.json.gz"))?,
        Compression::default(),
    );
    serde_json::to_writer(&mut reject_writer, &rejected)?;
    reject_writer.finish()?;
    let approved = SourceManifest::open(manifest_path)?;
    let database = GeoDb::open(geodb_path)?;
    if approved.source.len() != database.sources.len() || database.feature_count() != records.len()
    {
        return Err("NRN GeoDB source or feature count mismatch".into());
    }
    println!(
        "province={province} source_sha256={} accepted={} rejected={} bounds={bounds:?} geodb_bytes={}",
        manifest.source[0].sha256,
        records.len(),
        rejected.len(),
        std::fs::metadata(geodb_path)?.len(),
    );
    Ok(())
}
