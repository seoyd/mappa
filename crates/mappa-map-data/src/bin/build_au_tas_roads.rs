//! Convert an audited statewide LIST Transport Segments snapshot to spatial road packs.

use flate2::{Compression, write::GzEncoder};
use mappa_map_data::canonical::{
    GeoDb, SourceManifest, SourceRecord, adapt_au_tas_list_transport_segments, write_geodb,
};
use sha2::{Digest, Sha256};
use std::{error::Error, fs::File, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err(
            "usage: build_au_tas_roads SNAPSHOT.toml DOWNLOAD_DATE OUTPUT_DIR MANIFEST_DIR".into(),
        );
    }
    let snapshot_path = Path::new(&args[1]).canonicalize()?;
    let output_dir = Path::new(&args[3]);
    let manifest_dir = Path::new(&args[4]);
    std::fs::create_dir_all(output_dir)?;
    std::fs::create_dir_all(manifest_dir)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut File::open(&snapshot_path)?, &mut hasher)?;
    let source_sha256 = format!("{:x}", hasher.finalize());
    let source_file = snapshot_path
        .strip_prefix(manifest_dir.canonicalize()?)
        .map_or_else(
            |_| snapshot_path.to_string_lossy().into_owned(),
            |relative| relative.to_string_lossy().into_owned(),
        );
    let source = SourceRecord {
        id: format!("au-tas-list-transport-segments-{}", &source_sha256[..16]),
        name: "LIST Transport Segments".into(),
        provider: "Land Tasmania, State of Tasmania".into(),
        source_version: format!("sha256:{source_sha256}"),
        download_date: args[2].clone(),
        official_url: "https://services.thelist.tas.gov.au/arcgis/rest/services/Public/OpenDataWFS/MapServer/42".into(),
        download_page_url: "https://www.thelist.tas.gov.au/app/content/data/geo-meta-data-record?detailRecordUID=1ab7e34f-811c-4521-a549-212f295acc97".into(),
        file: snapshot_path.to_string_lossy().into_owned(),
        sha256: source_sha256,
        upstream_file: None,
        upstream_sha256: None,
        dedup_file: None,
        dedup_sha256: None,
        crs: "Official REST layer EPSG:3857; output outSR=4326; service reprojection has not been independently checked".into(),
        format: "Audited ArcGIS GeoJSON pages with OBJECTID index, UFI feature IDs and per-page SHA-256".into(),
        coverage: "Tasmania LIST Transport Segments; public open vehicular roads in z8 midpoint regions".into(),
        resolution: "Official transport centreline; source reports variable position accuracy, independent accuracy not measured".into(),
        update_frequency: "Source says continually revised; this is one acquired REST snapshot".into(),
        license_id: "CC-BY-3.0-AU".into(),
        license_url: "https://creativecommons.org/licenses/by/3.0/au/".into(),
        license_status: "APPROVED_WITH_ATTRIBUTION".into(),
        commercial_use: true,
        modification: true,
        redistribution: true,
        attribution_required: true,
        attribution_text: Some("LIST Transport Segments from theLIST © State of Tasmania · CC BY 3.0 AU · adapted by Mappa".into()),
        share_alike: false,
        adapter: "au-tas-list-transport-segments".into(),
        adapter_version: 1,
    };
    let (regions, rejected) = adapt_au_tas_list_transport_segments(&source)?;
    if regions.is_empty() {
        return Err("LIST source yielded no supported road segments".into());
    }
    let mut total = 0;
    for (region, records) in &regions {
        let mut bounds = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for (feature, _) in records {
            bounds[0] = bounds[0].min(feature.bbox.west);
            bounds[1] = bounds[1].min(feature.bbox.south);
            bounds[2] = bounds[2].max(feature.bbox.east);
            bounds[3] = bounds[3].max(feature.bbox.north);
        }
        let geodb_path = output_dir.join(format!("{region}.mgeodb"));
        let manifest_path = manifest_dir.join(format!("au_tas_roads_{region}.toml"));
        write_geodb(&geodb_path, records)?;
        let mut manifest_source = source.clone();
        manifest_source.file = source_file.clone();
        manifest_source.coverage =
            format!("Tasmania LIST Transport Segments; z8 midpoint region={region}");
        let manifest = SourceManifest {
            schema_version: 1,
            proof_region: format!("au-tas-list-{region}"),
            proof_bbox_wgs84: bounds,
            source: vec![manifest_source],
        };
        std::fs::write(&manifest_path, toml::to_string_pretty(&manifest)?)?;
        SourceManifest::open(&manifest_path)?;
        let db = GeoDb::open(&geodb_path)?;
        if db.feature_count() != records.len() || db.provenance.len() != records.len() {
            return Err(format!("LIST GeoDB feature/provenance mismatch: {region}").into());
        }
        total += records.len();
        println!(
            "region={region} accepted={} bounds={bounds:?} geodb_bytes={}",
            records.len(),
            std::fs::metadata(&geodb_path)?.len()
        );
    }
    let mut rejected_output = GzEncoder::new(
        File::create(output_dir.join("rejected.json.gz"))?,
        Compression::best(),
    );
    serde_json::to_writer(&mut rejected_output, &rejected)?;
    rejected_output.finish()?;
    println!(
        "source_sha256={} regions={} accepted={} rejected={}",
        source.sha256,
        regions.len(),
        total,
        rejected.len()
    );
    Ok(())
}
