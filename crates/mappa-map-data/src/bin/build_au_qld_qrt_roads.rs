//! Convert an audited Queensland Roads and Tracks snapshot to LGA packs.

use flate2::{Compression, write::GzEncoder};
use mappa_map_data::canonical::{
    GeoDb, SourceManifest, SourceRecord, adapt_au_qld_qrt_roads, write_geodb,
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, error::Error, fs::File, path::Path};

fn slug(name: &str) -> String {
    let mut output = String::new();
    for byte in name.bytes() {
        if byte.is_ascii_alphanumeric() {
            output.push(char::from(byte.to_ascii_lowercase()));
        } else if !output.ends_with('-') && !output.is_empty() {
            output.push('-');
        }
    }
    output.trim_end_matches('-').to_owned()
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err(
            "usage: build_au_qld_qrt_roads SNAPSHOT.toml DOWNLOAD_DATE OUTPUT_DIR MANIFEST_DIR"
                .into(),
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
        id: format!("au-qld-qrt-roads-{}", &source_sha256[..16]),
        name: "Queensland Roads and Tracks".into(),
        provider: "State of Queensland, Department of Natural Resources and Mines, Manufacturing and Regional and Rural Development".into(),
        source_version: format!("sha256:{source_sha256}"),
        download_date: args[2].clone(),
        official_url: "https://spatial-gis.information.qld.gov.au/arcgis/rest/services/Transportation/RoadsAndTracks/MapServer/10".into(),
        download_page_url: "https://www.data.qld.gov.au/dataset/queensland-roads-and-tracks".into(),
        file: snapshot_path.to_string_lossy().into_owned(),
        sha256: source_sha256,
        upstream_file: None,
        upstream_sha256: None,
        dedup_file: None,
        dedup_sha256: None,
        crs: "WGS84 geographic degrees returned by official ArcGIS outSR=4326; original service Web Mercator 102100".into(),
        format: "Audited GeoJSON page snapshot with source ID and page SHA-256 manifest".into(),
        coverage: "Queensland Roads and Tracks; LGA assignment from source lga_name_left/lga_name_right".into(),
        resolution: "Approximate road centreline; per-feature pos_accuracy exists but independent position accuracy not measured".into(),
        update_frequency: "Official QRT service is maintained daily; this is one acquired snapshot".into(),
        license_id: "CC-BY-4.0".into(),
        license_url: "https://creativecommons.org/licenses/by/4.0/".into(),
        license_status: "APPROVED_WITH_ATTRIBUTION".into(),
        commercial_use: true,
        modification: true,
        redistribution: true,
        attribution_required: true,
        attribution_text: Some("© State of Queensland (Department of Natural Resources and Mines, Manufacturing and Regional and Rural Development) 2025".into()),
        share_alike: false,
        adapter: "au-qld-qrt-roads".into(),
        adapter_version: 1,
    };
    let (regions, rejected) = adapt_au_qld_qrt_roads(&source)?;
    if regions.is_empty() {
        return Err("Queensland source yielded no supported road segments".into());
    }
    let mut slugs = BTreeSet::new();
    let mut total = 0;
    for (region, records) in &regions {
        let code = slug(region);
        if code.is_empty() || !slugs.insert(code.clone()) {
            return Err(format!("empty or colliding Queensland LGA slug: {region}").into());
        }
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
        let geodb_path = output_dir.join(format!("{code}.mgeodb"));
        let manifest_path = manifest_dir.join(format!("au_qld_qrt_roads_{code}.toml"));
        write_geodb(&geodb_path, records)?;
        let mut manifest_source = source.clone();
        manifest_source.file = source_file.clone();
        manifest_source.coverage =
            format!("Queensland Roads and Tracks; source LGA assignment={region}");
        let manifest = SourceManifest {
            schema_version: 1,
            proof_region: format!("au-qld-qrt-{code}"),
            proof_bbox_wgs84: bounds,
            source: vec![manifest_source],
        };
        std::fs::write(&manifest_path, toml::to_string_pretty(&manifest)?)?;
        SourceManifest::open(&manifest_path)?;
        let db = GeoDb::open(&geodb_path)?;
        if db.feature_count() != records.len() || db.provenance.len() != records.len() {
            return Err(format!("Queensland GeoDB feature/provenance mismatch: {code}").into());
        }
        total += records.len();
        println!(
            "region={region:?} code={code} accepted={} bounds={bounds:?} geodb_bytes={}",
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
