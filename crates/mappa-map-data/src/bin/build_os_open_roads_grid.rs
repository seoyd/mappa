//! Convert one official OS Open Roads 100 km square into Mappa's local GeoDB.

use flate2::{Compression, write::GzEncoder};
use mappa_map_data::canonical::{
    GeoDb, SourceManifest, SourceRecord, adapt_os_open_roads_grid, write_geodb,
};
use md5::{Digest as Md5Digest, Md5};
use serde_json::Value;
use sha2::{Digest as ShaDigest, Sha256};
use std::{error::Error, fs::File, io::Read, path::Path};
use zip::ZipArchive;

fn required<'a>(value: &'a Value, key: &str) -> Result<&'a str, Box<dyn Error>> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("missing official metadata field: {key}").into())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 8 {
        return Err("usage: build_os_open_roads_grid SOURCE.zip GRID PRODUCT.json DOWNLOADS.json DOWNLOAD_DATE OUTPUT.mgeodb OUTPUT.toml".into());
    }
    let source_path = Path::new(&args[1]);
    let grid = &args[2];
    let product: Value = serde_json::from_reader(File::open(&args[3])?)?;
    let downloads: Value = serde_json::from_reader(File::open(&args[4])?)?;
    if required(&product, "id")? != "OpenRoads"
        || !product["areas"]
            .as_array()
            .is_some_and(|areas| areas.iter().any(|area| area == "GB"))
    {
        return Err("unexpected OS product metadata".into());
    }
    let version = required(&product, "version")?;
    let entry = downloads
        .as_array()
        .ok_or("downloads metadata is not an array")?
        .iter()
        .find(|item| {
            item["format"] == "ESRI® Shapefile"
                && item["area"] == "GB"
                && item["fileName"] == "oproad_essh_gb.zip"
        })
        .ok_or("OS Shapefile download absent")?;
    let published_bytes = entry["size"].as_u64().ok_or("missing published size")?;
    let published_md5 = required(entry, "md5")?;
    if std::fs::metadata(source_path)?.len() != published_bytes {
        return Err("source ZIP size differs from official listing".into());
    }
    let mut sha = Sha256::new();
    let mut md5 = Md5::new();
    let mut file = File::open(source_path)?;
    let mut block = [0u8; 256 * 1024];
    loop {
        let size = file.read(&mut block)?;
        if size == 0 {
            break;
        }
        ShaDigest::update(&mut sha, &block[..size]);
        Md5Digest::update(&mut md5, &block[..size]);
    }
    let actual_md5: String = Md5Digest::finalize(md5)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if actual_md5 != published_md5 {
        return Err("source ZIP MD5 differs from official listing".into());
    }
    let mut source_archive = ZipArchive::new(File::open(source_path)?)?;
    let mut licence_member = source_archive.by_name("/doc/licence.txt")?;
    if licence_member.size() > 16 * 1024 {
        return Err("OS embedded licence notice exceeds 16 KiB".into());
    }
    let mut licence_text = String::new();
    licence_member.read_to_string(&mut licence_text)?;
    let credit = licence_text
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("Contains Ordnance Survey data ©"))
        .ok_or("OS embedded copyright acknowledgement absent")?;
    if !credit.ends_with(version.get(..4).ok_or("invalid OS release year")?) {
        return Err("OS embedded copyright year differs from product release".into());
    }
    let output_manifest = Path::new(&args[7]);
    let manifest_dir = output_manifest.parent().unwrap_or_else(|| Path::new("."));
    let canonical_source = source_path.canonicalize()?;
    let source_file = canonical_source
        .strip_prefix(manifest_dir.canonicalize()?)
        .map_or_else(
            |_| canonical_source.to_string_lossy().into_owned(),
            |relative| relative.to_string_lossy().into_owned(),
        );
    let source = SourceRecord {
        id: format!("os-open-roads-{version}-gb"),
        name: "OS Open Roads RoadLink".into(),
        provider: "Ordnance Survey".into(),
        source_version: version.into(),
        download_date: args[5].clone(),
        official_url: required(entry, "url")?.into(),
        download_page_url: required(&product, "url")?.into(),
        file: canonical_source.to_string_lossy().into_owned(),
        sha256: format!("{:x}", ShaDigest::finalize(sha)),
        upstream_file: None,
        upstream_sha256: None,
        crs: "EPSG:27700 (OSGB36); OSTN15 to ETRS89 geographic".into(),
        format: "ESRI Shapefile ZIP / RoadLink PolylineZ".into(),
        coverage: format!("Great Britain source; {grid} 100 km square selection"),
        resolution: "OS Open Roads generalized 1:25,000".into(),
        update_frequency: "Official product releases".into(),
        license_id: "OGL-UK-3.0".into(),
        license_url: "https://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/"
            .into(),
        license_status: "APPROVED_WITH_ATTRIBUTION".into(),
        commercial_use: true,
        modification: true,
        redistribution: true,
        attribution_required: true,
        attribution_text: Some(credit.to_owned()),
        share_alike: false,
        adapter: "os-open-roads".into(),
        adapter_version: 1,
    };
    let (records, rejected) = adapt_os_open_roads_grid(&source, grid)?;
    if records.is_empty() {
        return Err("no OS roads accepted in selected grid".into());
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
    let mut source_for_manifest = source;
    source_for_manifest.file = source_file;
    let manifest = SourceManifest {
        schema_version: 1,
        proof_region: format!("os-open-roads-{grid}"),
        proof_bbox_wgs84: bounds,
        source: vec![source_for_manifest],
    };
    write_geodb(Path::new(&args[6]), &records)?;
    std::fs::write(output_manifest, toml::to_string_pretty(&manifest)?)?;
    let writer = GzEncoder::new(
        File::create(Path::new(&args[6]).with_extension("rejected.json.gz"))?,
        Compression::default(),
    );
    let mut writer = writer;
    serde_json::to_writer(&mut writer, &rejected)?;
    writer.finish()?;
    let approved = SourceManifest::open(output_manifest)?;
    let database = GeoDb::open(Path::new(&args[6]))?;
    if approved.source.len() != database.sources.len() || database.feature_count() != records.len()
    {
        return Err("OS GeoDB source or feature count mismatch".into());
    }
    println!(
        "grid={grid} source_sha256={} accepted={} rejected={} bounds={bounds:?} geodb_bytes={}",
        manifest.source[0].sha256,
        records.len(),
        rejected.len(),
        std::fs::metadata(&args[6])?.len(),
    );
    Ok(())
}
