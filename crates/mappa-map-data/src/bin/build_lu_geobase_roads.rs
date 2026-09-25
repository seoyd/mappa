//! Build a Luxembourg GeoDB from the pinned official BD-L-GeoBase TransportNetwork theme.

use flate2::{Compression, write::GzEncoder};
use mappa_map_data::canonical::{
    GeoDb, SourceManifest, SourceRecord, adapt_lu_geobase_roads, write_geodb,
};
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fs::{self, File},
    path::Path,
};

fn sha256(path: &Path) -> Result<String, Box<dyn Error>> {
    let mut hash = Sha256::new();
    std::io::copy(&mut File::open(path)?, &mut hash)?;
    Ok(format!("{:x}", hash.finalize()))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 6 {
        return Err("usage: build_lu_geobase_roads ARCHIVE.zip TRANSPORT.gpkg DOWNLOAD_DATE OUTPUT.mgeodb MANIFEST.toml".into());
    }
    let archive = Path::new(&args[1]).canonicalize()?;
    let gpkg = Path::new(&args[2]).canonicalize()?;
    let date = &args[3];
    let geodb = Path::new(&args[4]);
    let manifest_path = Path::new(&args[5]);
    if date.len() != 10
        || !date.bytes().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                c == b'-'
            } else {
                c.is_ascii_digit()
            }
        })
    {
        return Err("invalid download date".into());
    }
    if archive.file_name().and_then(|name| name.to_str()) != Some("transportnetwork-20260708.zip")
        || gpkg.file_name().and_then(|name| name.to_str()) != Some("TransportNetwork_20260708.gpkg")
    {
        return Err("unexpected BD-L-GeoBase release files".into());
    }
    let source_sha256 = sha256(&gpkg)?;
    let archive_sha256 = sha256(&archive)?;
    let source = SourceRecord {
        id: format!("lu-geobase-road-{}", &source_sha256[..16]),
        name: "BD-L-GeoBase TransportNetwork_TN_Road".into(),
        provider: "Administration du Cadastre et de la Topographie, Luxembourg".into(),
        source_version: "2026-07-08".into(),
        download_date: date.clone(),
        official_url: "https://download.data.public.lu/resources/bd-l-geobase/20260708-140721/transportnetwork-20260708.zip".into(),
        download_page_url: "https://data.public.lu/en/datasets/bd-l-geobase/".into(),
        file: gpkg.to_string_lossy().into_owned(),
        sha256: source_sha256,
        upstream_file: Some(archive.to_string_lossy().into_owned()),
        upstream_sha256: Some(archive_sha256),
        dedup_file: None,
        dedup_sha256: None,
        crs: "EPSG:2169 LUREF/Luxembourg TM; EPSG LUREF→WGS84 (3) seven-parameter transform via pure Rust proj4rs; one point matched independent GDAL/PROJ within 0.000001 degree".into(),
        format: "Official GeoPackage MULTICURVE ZM; Rust SQLite/WKB reader".into(),
        coverage: "Luxembourg official national TransportNetwork road theme".into(),
        resolution: "Official vector roads; positional accuracy and network connectivity not independently verified".into(),
        update_frequency: "Official portal says quarterly; this is the pinned 2026-07-08 snapshot".into(),
        license_id: "CC0-1.0".into(),
        license_url: "https://creativecommons.org/publicdomain/zero/1.0/".into(),
        license_status: "APPROVED".into(),
        commercial_use: true,
        modification: true,
        redistribution: true,
        attribution_required: false,
        attribution_text: Some("Luxembourg Administration du Cadastre et de la Topographie, BD-L-GeoBase · CC0 · adapted by Mappa".into()),
        share_alike: false,
        adapter: "lu-geobase-road".into(),
        adapter_version: 1,
    };
    let (records, rejected) = adapt_lu_geobase_roads(&source)?;
    if records.is_empty() {
        return Err("BD-L-GeoBase yielded no supported vehicle roads".into());
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
    if let Some(parent) = geodb.parent() {
        fs::create_dir_all(parent)?;
    }
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent)?;
    }
    write_geodb(geodb, &records)?;
    let mut manifest_source = source.clone();
    let manifest_dir = manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .canonicalize()?;
    manifest_source.file = gpkg
        .strip_prefix(&manifest_dir)?
        .to_string_lossy()
        .into_owned();
    manifest_source.upstream_file = Some(
        archive
            .strip_prefix(&manifest_dir)?
            .to_string_lossy()
            .into_owned(),
    );
    manifest_source.coverage = format!("accepted functional vehicle road bounds {bounds:?}");
    let manifest = SourceManifest {
        schema_version: 1,
        proof_region: "lu-geobase-transport-roads-20260708".into(),
        proof_bbox_wgs84: bounds,
        source: vec![manifest_source],
    };
    fs::write(manifest_path, toml::to_string_pretty(&manifest)?)?;
    SourceManifest::open(manifest_path)?;
    let database = GeoDb::open(geodb)?;
    if database.feature_count() != records.len() || database.provenance.len() != records.len() {
        return Err("Luxembourg GeoDB count/provenance mismatch".into());
    }
    let rejected_path = geodb.with_extension("rejected.json.gz");
    let mut writer = GzEncoder::new(File::create(&rejected_path)?, Compression::best());
    serde_json::to_writer(&mut writer, &rejected)?;
    writer.finish()?;
    println!(
        "accepted={} rejected={} bounds={bounds:?} geodb_bytes={} rejected_file={}",
        records.len(),
        rejected.len(),
        fs::metadata(geodb)?.len(),
        rejected_path.display()
    );
    Ok(())
}
