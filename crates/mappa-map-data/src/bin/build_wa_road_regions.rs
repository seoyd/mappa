//! Convert one official WA archive into source-region MappaGeoDB proofs.

use mappa_map_data::canonical::{
    GeoDb, SourceManifest, SourceRecord, adapt_wa_road_network, write_geodb,
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
            "usage: build_wa_road_regions SOURCE.zip DOWNLOAD_DATE OUTPUT_DIR MANIFEST_DIR".into(),
        );
    }
    let source_path = Path::new(&args[1]).canonicalize()?;
    let output_dir = Path::new(&args[3]);
    let manifest_dir = Path::new(&args[4]);
    std::fs::create_dir_all(output_dir)?;
    std::fs::create_dir_all(manifest_dir)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut File::open(&source_path)?, &mut hasher)?;
    let source_sha256 = format!("{:x}", hasher.finalize());
    let source_file = source_path
        .strip_prefix(manifest_dir.canonicalize()?)
        .map_or_else(
            |_| source_path.to_string_lossy().into_owned(),
            |relative| relative.to_string_lossy().into_owned(),
        );
    let source = SourceRecord {
        id: format!("au-wa-main-roads-network-{}", &source_sha256[..16]),
        name: "Road Network".into(),
        provider: "Main Roads Western Australia".into(),
        source_version: format!("sha256:{source_sha256}"),
        download_date: args[2].clone(),
        official_url: "https://portal-mainroads.opendata.arcgis.com/api/download/v1/items/7febe68ed1764e0d8402264ad62f0357/shapefile?layers=17".into(),
        download_page_url: "https://catalogue.data.wa.gov.au/en/dataset/mrwa-road-network".into(),
        file: source_path.to_string_lossy().into_owned(),
        sha256: source_sha256,
        upstream_file: None,
        upstream_sha256: None,
        dedup_file: None,
        dedup_sha256: None,
        crs: "GDA94 geographic degrees (source .prj); numeric degrees retained; WGS84 position unverified".into(),
        format: "Shapefile ZIP, Polyline".into(),
        coverage: "WA Main Roads Road Network; source RA_NAME regions".into(),
        resolution: "Official road centreline geometry; positional accuracy not independently tested".into(),
        update_frequency: "Unspecified by WA dataset catalogue".into(),
        license_id: "CC-BY-4.0".into(),
        license_url: "https://creativecommons.org/licenses/by/4.0/".into(),
        license_status: "APPROVED_WITH_ATTRIBUTION".into(),
        commercial_use: true,
        modification: true,
        redistribution: true,
        attribution_required: true,
        attribution_text: Some("The Commissioner of Main Roads is the creator and owner of the data and Licenced Material, which is accessed pursuant to a Creative Commons (Attribution) Licence, which has a disclaimer of warranties and limitation of liability.".into()),
        share_alike: false,
        adapter: "au-wa-road-network".into(),
        adapter_version: 1,
    };
    let (regions, rejected) = adapt_wa_road_network(&source)?;
    if regions.is_empty() {
        return Err("WA source yielded no supported road segments".into());
    }
    let mut slugs = BTreeSet::new();
    let mut total = 0;
    for (region, records) in &regions {
        let code = slug(region);
        if code.is_empty() || !slugs.insert(code.clone()) {
            return Err("empty or colliding WA source region slug".into());
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
        let manifest_path = manifest_dir.join(format!("au_wa_roads_{code}.toml"));
        write_geodb(&geodb_path, records)?;
        let mut manifest_source = source.clone();
        manifest_source.file = source_file.clone();
        manifest_source.coverage = format!("WA Road Network source RA_NAME={region}");
        let manifest = SourceManifest {
            schema_version: 1,
            proof_region: format!("au-wa-{code}"),
            proof_bbox_wgs84: bounds,
            source: vec![manifest_source],
        };
        std::fs::write(&manifest_path, toml::to_string_pretty(&manifest)?)?;
        SourceManifest::open(&manifest_path)?;
        let db = GeoDb::open(&geodb_path)?;
        if db.feature_count() != records.len() || db.provenance.len() != records.len() {
            return Err(format!("WA GeoDB feature/provenance mismatch: {code}").into());
        }
        total += records.len();
        println!(
            "region={region:?} code={code} accepted={} bounds={bounds:?} geodb_bytes={}",
            records.len(),
            std::fs::metadata(&geodb_path)?.len()
        );
    }
    std::fs::write(
        output_dir.join("rejected.json"),
        serde_json::to_vec_pretty(&rejected)?,
    )?;
    println!(
        "source_sha256={} regions={} accepted={} rejected={}",
        source.sha256,
        regions.len(),
        total,
        rejected.len()
    );
    Ok(())
}
