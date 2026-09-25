//! Build a nationwide Irish local-road GeoDB from the pinned RMO Shapefile ZIP.

use flate2::{Compression, write::GzEncoder};
use mappa_map_data::canonical::{
    GeoDb, SourceManifest, SourceRecord, adapt_ie_rmo_local_roads, write_geodb,
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
    if args.len() != 5 {
        return Err(
            "usage: build_ie_rmo_local_roads SOURCE.zip DOWNLOAD_DATE OUTPUT.mgeodb MANIFEST.toml"
                .into(),
        );
    }
    let archive = Path::new(&args[1]).canonicalize()?;
    let date = &args[2];
    let geodb = Path::new(&args[3]);
    let manifest_path = Path::new(&args[4]);
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
    if archive.file_name().and_then(|name| name.to_str())
        != Some("Local_Road_Schedule_Open_data_Shapefile.zip")
    {
        return Err("unexpected Irish RMO source archive name".into());
    }
    let digest = sha256(&archive)?;
    let source = SourceRecord {
        id: format!("ie-rmo-local-roads-{}", &digest[..16]),
        name: "Local Road Network Open data Shapefile".into(),
        provider: "Road Management Office and Irish Local Authorities".into(),
        source_version: "ArcGIS item modified 2024-12-12; ZIP members dated 2024-12-12".into(),
        download_date: date.clone(),
        official_url: "https://www.arcgis.com/sharing/rest/content/items/a049f91847034767b00896e48871cb91/data".into(),
        download_page_url: "https://data.gov.ie/dataset/local-roads".into(),
        file: archive.to_string_lossy().into_owned(),
        sha256: digest,
        upstream_file: None,
        upstream_sha256: None,
        dedup_file: None,
        dedup_sha256: None,
        crs: "EPSG:2157 IRENET95/Irish TM; EPSG transformation to WGS84 via pure Rust proj4rs; first source vertex matched independent GDAL/PROJ within 0.000001 degree".into(),
        format: "Official ZIP Shapefile with .shp/.dbf/.prj, Rust ZIP and Shapefile reader".into(),
        coverage: "Ireland 31 local authorities; local-road schedule, excluding national road network".into(),
        resolution: "Official local road schedule; completeness and on-ground accuracy not independently measured".into(),
        update_frequency: "Official portal says irregular; this is the pinned source ZIP".into(),
        license_id: "CC-BY-4.0".into(),
        license_url: "https://creativecommons.org/licenses/by/4.0/".into(),
        license_status: "APPROVED_WITH_ATTRIBUTION".into(),
        commercial_use: true,
        modification: true,
        redistribution: true,
        attribution_required: true,
        attribution_text: Some("Road Management Office and Irish Local Authorities · CC BY 4.0 · adapted by Mappa".into()),
        share_alike: false,
        adapter: "ie-rmo-local-roads".into(),
        adapter_version: 1,
    };
    let (records, rejected) = adapt_ie_rmo_local_roads(&source)?;
    if records.is_empty() {
        return Err("Irish RMO source yielded no supported local roads".into());
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
    manifest_source.file = archive
        .strip_prefix(&manifest_dir)?
        .to_string_lossy()
        .into_owned();
    manifest_source.coverage = format!("accepted local road geometry bounds {bounds:?}");
    let manifest = SourceManifest {
        schema_version: 1,
        proof_region: "ie-rmo-local-roads-2024-12-12".into(),
        proof_bbox_wgs84: bounds,
        source: vec![manifest_source],
    };
    fs::write(manifest_path, toml::to_string_pretty(&manifest)?)?;
    SourceManifest::open(manifest_path)?;
    let database = GeoDb::open(geodb)?;
    if database.feature_count() != records.len() || database.provenance.len() != records.len() {
        return Err("Irish RMO GeoDB count/provenance mismatch".into());
    }
    let rejected_path = geodb.with_extension("rejected.json.gz");
    let mut writer = GzEncoder::new(File::create(&rejected_path)?, Compression::best());
    serde_json::to_writer(&mut writer, &rejected)?;
    writer.finish()?;
    println!(
        "accepted_parts={} rejected_records={} bounds={bounds:?} geodb_bytes={} rejected_file={}",
        records.len(),
        rejected.len(),
        fs::metadata(geodb)?.len(),
        rejected_path.display()
    );
    Ok(())
}
