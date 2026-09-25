//! Build spatial GeoDB packs from the pinned monthly Dutch NWB-Wegen release.

use flate2::{Compression, write::GzEncoder};
use mappa_map_data::canonical::{
    GeoDb, SourceManifest, SourceRecord, adapt_nl_nwb_roads, nl_nwb_regions, write_geodb,
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
        return Err("usage: build_nl_nwb_roads 01-09-2026.zip Wegvakken.gpkg DOWNLOAD_DATE OUTPUT_DIR MANIFEST_DIR".into());
    }
    let archive = Path::new(&args[1]).canonicalize()?;
    let gpkg = Path::new(&args[2]).canonicalize()?;
    let date = &args[3];
    let output_dir = Path::new(&args[4]);
    let manifest_dir = Path::new(&args[5]);
    if archive.file_name().and_then(|name| name.to_str()) != Some("01-09-2026.zip")
        || gpkg.file_name().and_then(|name| name.to_str()) != Some("Wegvakken.gpkg")
        || date.len() != 10
        || !date.bytes().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                c == b'-'
            } else {
                c.is_ascii_digit()
            }
        })
    {
        return Err("unexpected NWB release file or download date".into());
    }
    fs::create_dir_all(output_dir)?;
    fs::create_dir_all(manifest_dir)?;
    let source_sha256 = sha256(&gpkg)?;
    let archive_sha256 = sha256(&archive)?;
    let source = SourceRecord {
        id: format!("nl-nwb-wegen-{}", &source_sha256[..16]),
        name: "NWB-Wegen Wegvakken".into(),
        provider: "Rijkswaterstaat, Netherlands".into(),
        source_version: "2026-09-01 monthly NWB-Wegen".into(),
        download_date: date.clone(),
        official_url: "https://downloads.rijkswaterstaatdata.nl/nwb-wegen/geogegevens/geopackage/Nederland_totaal/01-09-2026.zip".into(),
        download_page_url: "https://api.pdok.nl/rws/nationaal-wegenbestand-wegen/ogc/v1?f=html&lang=nl".into(),
        file: gpkg.to_string_lossy().into_owned(),
        sha256: source_sha256,
        upstream_file: Some(archive.to_string_lossy().into_owned()),
        upstream_sha256: Some(archive_sha256),
        dedup_file: None,
        dedup_sha256: None,
        crs: "EPSG:28992 Amersfoort/RD New; pure Rust proj4rs seven-parameter transform to WGS84; one source vertex checked against independent PROJ".into(),
        format: "Official GeoPackage 2D LINESTRING with SQLite RTree; Rust reader".into(),
        coverage: "NWB-Wegen national road sections with a street name or road number; 100-km RD grid ownership by source bbox midpoint".into(),
        resolution: "Official vector road sections; completeness and on-ground accuracy not independently measured".into(),
        update_frequency: "Monthly; pinned 2026-09-01 release".into(),
        license_id: "CC0-1.0".into(),
        license_url: "https://creativecommons.org/publicdomain/zero/1.0/".into(),
        license_status: "APPROVED".into(),
        commercial_use: true,
        modification: true,
        redistribution: true,
        attribution_required: false,
        attribution_text: Some("Rijkswaterstaat NWB-Wegen · CC0 · adapted by Mappa".into()),
        share_alike: false,
        adapter: "nl-nwb-wegen".into(),
        adapter_version: 1,
    };
    let mut total_accepted = 0;
    let mut total_rejected = 0;
    let regions = nl_nwb_regions(&source)?;
    for region in &regions {
        let region_id = format!("rd-{}-{}", region.0, region.1);
        let (records, rejected) = adapt_nl_nwb_roads(&source, *region)?;
        if records.is_empty() {
            return Err(format!("empty NWB region: {region_id}").into());
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
        let geodb = output_dir.join(format!("{region_id}.mgeodb"));
        write_geodb(&geodb, &records)?;
        let mut manifest_source = source.clone();
        let manifest_root = manifest_dir.canonicalize()?;
        manifest_source.file = gpkg
            .strip_prefix(&manifest_root)?
            .to_string_lossy()
            .into_owned();
        manifest_source.upstream_file = Some(
            archive
                .strip_prefix(&manifest_root)?
                .to_string_lossy()
                .into_owned(),
        );
        manifest_source.coverage =
            format!("NWB-Wegen region {region_id}; accepted geometry bounds {bounds:?}");
        let manifest = SourceManifest {
            schema_version: 1,
            proof_region: format!("nl-nwb-wegen-{region_id}"),
            proof_bbox_wgs84: bounds,
            source: vec![manifest_source],
        };
        let manifest_path = manifest_dir.join(format!("nl_nwb_roads_{region_id}.toml"));
        fs::write(&manifest_path, toml::to_string_pretty(&manifest)?)?;
        SourceManifest::open(&manifest_path)?;
        let database = GeoDb::open(&geodb)?;
        if database.feature_count() != records.len() || database.provenance.len() != records.len() {
            return Err(format!("NWB GeoDB count/provenance mismatch: {region_id}").into());
        }
        let mut rejected_output = GzEncoder::new(
            File::create(output_dir.join(format!("{region_id}.rejected.json.gz")))?,
            Compression::best(),
        );
        serde_json::to_writer(&mut rejected_output, &rejected)?;
        rejected_output.finish()?;
        total_accepted += records.len();
        total_rejected += rejected.len();
        println!(
            "region={region_id} accepted={} rejected={} bounds={bounds:?} geodb_bytes={}",
            records.len(),
            rejected.len(),
            fs::metadata(&geodb)?.len()
        );
    }
    println!(
        "regions={} accepted={} rejected={}",
        regions.len(),
        total_accepted,
        total_rejected
    );
    Ok(())
}
