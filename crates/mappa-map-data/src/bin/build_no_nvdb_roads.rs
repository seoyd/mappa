//! Build a local GeoDB from a pinned Norway NVDB V4 municipality snapshot.

use flate2::{Compression, write::GzEncoder};
use mappa_map_data::canonical::{
    GeoDb, SourceManifest, SourceRecord, adapt_no_nvdb_roads, adapt_no_nvdb_segmented_roads,
    write_geodb,
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
    let args: Vec<String> = std::env::args().collect();
    if !(args.len() == 5 || args.len() == 6) {
        return Err(
            "usage: build_no_nvdb_roads SNAPSHOT.toml DOWNLOAD_DATE OUTPUT.mgeodb MANIFEST.toml [ENRICHMENT_SNAPSHOT.toml]"
                .into(),
        );
    }
    let snapshot = Path::new(&args[1]).canonicalize()?;
    let date = &args[2];
    if date.len() != 10
        || !date.bytes().enumerate().all(|(index, byte)| {
            if index == 4 || index == 7 {
                byte == b'-'
            } else {
                byte.is_ascii_digit()
            }
        })
    {
        return Err("invalid snapshot download date".into());
    }
    let geodb = Path::new(&args[3]);
    let manifest_path = Path::new(&args[4]);
    let snapshot_value: toml::Value = toml::from_str(&fs::read_to_string(&snapshot)?)?;
    let municipality = snapshot_value
        .get("municipality")
        .and_then(toml::Value::as_integer)
        .ok_or("snapshot has no municipality")?;
    let segmented = snapshot_value
        .get("source_url")
        .and_then(toml::Value::as_str)
        .ok_or("snapshot has no source URL")?
        .ends_with("/segmentert");
    let enrichment = args
        .get(5)
        .map(|path| Path::new(path).canonicalize())
        .transpose()?;
    if segmented && enrichment.is_some() {
        return Err("segmented source cannot also take an enrichment source".into());
    }
    let digest = sha256(&snapshot)?;
    let enrichment_digest = enrichment.as_deref().map(sha256).transpose()?;
    let source_suffix = if let Some(other) = &enrichment_digest {
        format!("{}-{}", &digest[..12], &other[..12])
    } else {
        digest[..16].to_owned()
    };
    let source = SourceRecord {
        id: format!("no-nvdb-v4-{municipality}-{source_suffix}"),
        name: if segmented {
            "NVDB API Les V4 segmented road links"
        } else {
            "NVDB API Les V4 road-link sequences"
        }
        .into(),
        provider: "Statens vegvesen, Norwegian Public Roads Administration".into(),
        source_version: format!("NVDB V4 municipality {municipality} snapshot {date}"),
        download_date: date.clone(),
        official_url: if segmented {
            "https://nvdbapiles.atlas.vegvesen.no/vegnett/api/v4/veglenkesekvenser/segmentert"
        } else {
            "https://nvdbapiles.atlas.vegvesen.no/vegnett/api/v4/veglenkesekvenser"
        }.into(),
        download_page_url: "https://nvdb-docs.atlas.vegvesen.no/nvdbapil/v4/Vegnett/".into(),
        file: snapshot.to_string_lossy().into_owned(),
        sha256: digest,
        upstream_file: enrichment.as_ref().map(|path| path.to_string_lossy().into_owned()),
        upstream_sha256: enrichment_digest,
        dedup_file: None,
        dedup_sha256: None,
        crs: "EPSG:5973 EUREF89/UTM33N with NN2000 elevation; pure Rust proj4rs horizontal transform to WGS84; source vertex checked with independent PROJ".into(),
        format: "NVDB V4 paged JSON; page byte SHA-256 validated against local snapshot.toml".into(),
        coverage: format!("NVDB {} road links intersecting municipality {municipality}; links may extend over its boundary", if segmented { "segmented" } else { "unsegmented" }),
        resolution: if enrichment.is_some() {
            "Official unsegmented vector road geometry enriched only where matching segmented source IDs provide unambiguous road class/name; field accuracy and completeness not independently verified"
        } else if segmented {
            "Official vector road segments with source street names and administrative road categories; field accuracy and completeness not independently verified"
        } else {
            "Official vector road links; source-date, completeness, names, major-road hierarchy and on-ground accuracy not independently verified"
        }.into(),
        update_frequency: "Live API; pinned local page-byte snapshot".into(),
        license_id: "NLOD-1.0".into(),
        license_url: "https://data.norge.no/nlod/no/1.0".into(),
        license_status: "APPROVED_WITH_ATTRIBUTION".into(),
        commercial_use: true,
        modification: true,
        redistribution: true,
        attribution_required: true,
        attribution_text: Some("Inneholder data under norsk lisens for offentlige data (NLOD) tilgjengeliggjort av Statens vegvesen. Bearbeidet av Mappa.".into()),
        share_alike: false,
        adapter: if enrichment.is_some() {
            "no-nvdb-v4-road-links-enriched"
        } else if segmented {
            "no-nvdb-v4-segmented-road-links"
        } else {
            "no-nvdb-v4-road-links"
        }.into(),
        adapter_version: 1,
    };
    let (records, rejected) = if segmented {
        adapt_no_nvdb_segmented_roads(&source)?
    } else {
        adapt_no_nvdb_roads(&source)?
    };
    if records.is_empty() {
        return Err("Norway NVDB snapshot yielded no physical vehicle roads".into());
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
    let manifest_root = manifest_path
        .parent()
        .ok_or("manifest has no parent")?
        .canonicalize()?;
    manifest_source.file = snapshot
        .strip_prefix(&manifest_root)?
        .to_string_lossy()
        .into_owned();
    if let Some(enrichment_path) = &enrichment {
        manifest_source.upstream_file = Some(
            enrichment_path
                .strip_prefix(&manifest_root)?
                .to_string_lossy()
                .into_owned(),
        );
    }
    let manifest = SourceManifest {
        schema_version: 1,
        proof_region: format!(
            "no-nvdb-v4-{}-{municipality}",
            if enrichment.is_some() {
                "enriched"
            } else if segmented {
                "segmented"
            } else {
                "unsegmented"
            }
        ),
        proof_bbox_wgs84: bounds,
        source: vec![manifest_source],
    };
    fs::write(manifest_path, toml::to_string_pretty(&manifest)?)?;
    SourceManifest::open(manifest_path)?;
    let opened = GeoDb::open(geodb)?;
    if opened.feature_count() != records.len() || opened.provenance.len() != records.len() {
        return Err("Norway NVDB GeoDB feature/provenance mismatch".into());
    }
    let rejection_path = geodb.with_extension("rejected.json.gz");
    let mut output = GzEncoder::new(File::create(rejection_path)?, Compression::best());
    serde_json::to_writer(&mut output, &rejected)?;
    output.finish()?;
    println!(
        "municipality={municipality} accepted={} rejected={} bbox={bounds:?} geodb_bytes={}",
        records.len(),
        rejected.len(),
        fs::metadata(geodb)?.len()
    );
    Ok(())
}
