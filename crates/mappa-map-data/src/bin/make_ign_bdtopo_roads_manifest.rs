//! Pin an official IGN departmental class and derive its accepted geometry bounds.

use mappa_map_data::canonical::{
    BBox, SourceManifest, SourceRecord, adapt_ign_bdtopo_roads, adapt_ign_bdtopo_stations,
    adapt_ign_bdtopo_water,
};
use sha2::{Digest, Sha256};
use std::{error::Error, fs, path::Path};

fn sha256(path: &Path) -> Result<String, Box<dyn Error>> {
    let mut input = fs::File::open(path)?;
    let mut hash = Sha256::new();
    std::io::copy(&mut input, &mut hash)?;
    Ok(format!("{:x}", hash.finalize()))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    let class = match args.get(6).map(String::as_str) {
        None if args.len() == 6 => "road",
        Some("--water") if args.len() == 7 => "water",
        Some("--transport") if args.len() == 7 => "transport",
        _ => return Err("usage: make_ign_bdtopo_roads_manifest OFFICIAL_7Z_URL ARCHIVE.7z SELECTED.zip OUTPUT.toml DOWNLOAD_DATE [--water|--transport]".into()),
    };
    if !args[1].starts_with("https://data.geopf.fr/telechargement/download/BDTOPO/") {
        return Err("unexpected IGN download URL".into());
    }
    let archive = Path::new(&args[2]);
    let road_zip = Path::new(&args[3]);
    let output = Path::new(&args[4]);
    let date = &args[5];
    if date.len() != 10
        || !date.bytes().enumerate().all(|(index, byte)| {
            if index == 4 || index == 7 {
                byte == b'-'
            } else {
                byte.is_ascii_digit()
            }
        })
    {
        return Err("invalid download date".into());
    }
    let stem = archive
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or("invalid archive name")?;
    if !stem.starts_with("BDTOPO_3-5_TOUSTHEMES_SHP_LAMB93_D")
        || !args[1].ends_with(&format!("/{stem}.7z"))
    {
        return Err("archive name and official URL disagree".into());
    }
    let parent = output.parent().ok_or("manifest needs a parent directory")?;
    let relative_zip = road_zip
        .strip_prefix(parent)?
        .to_string_lossy()
        .into_owned();
    let relative_archive = archive.strip_prefix(parent)?.to_string_lossy().into_owned();
    let source_id = match class {
        "water" => format!("ign-bdtopo-water-{}", stem.to_lowercase()),
        "transport" => format!("ign-bdtopo-stations-{}", stem.to_lowercase()),
        _ => format!("ign-bdtopo-{}", stem.to_lowercase()),
    };
    let mut source = SourceRecord {
        id: source_id,
        name: format!(
            "IGN BD TOPO 3.5 {}, {stem}",
            match class {
                "water" => "Surface hydrographique",
                "transport" => "Equipement de transport: passenger stations",
                _ => "Tronçon de route",
            }
        ),
        provider: "Institut national de l'information géographique et forestière (IGN)".into(),
        source_version: stem.into(),
        download_date: date.into(),
        official_url: args[1].clone(),
        download_page_url: "https://www.data.gouv.fr/datasets/bd-topo-r".into(),
        file: road_zip.to_string_lossy().into_owned(),
        sha256: sha256(road_zip)?,
        upstream_file: Some(archive.to_string_lossy().into_owned()),
        upstream_sha256: Some(sha256(archive)?),
        dedup_file: None,
        dedup_sha256: None,
        crs: "EPSG:2154 RGF93 / Lambert-93; inverted to RGF93 geographic degrees; independent WGS84 datum and positional accuracy pending".into(),
        format: format!(
            "official 7z Shapefile; {}-only ZIP selected and repacked in Rust",
            class
        ),
        coverage: "departmental source; observed selected geometry bounds below".into(),
        resolution: match class {
            "water" => "IGN BD TOPO 3.5 surface hydrography polygons; source precision varies by acquisition method",
            "transport" => "IGN BD TOPO 3.5 real passenger station footprints; source precision varies by acquisition method",
            _ => "IGN BD TOPO 3.5 road centerlines; source precision varies by acquisition method",
        }.into(),
        update_frequency: "published quarterly snapshot; pinned edition".into(),
        license_id: "ETALAB-LICENCE-OUVERTE-2.0".into(),
        license_url: "https://www.data.gouv.fr/pages/legal/licences/etalab-2.0".into(),
        license_status: "APPROVED_WITH_ATTRIBUTION".into(),
        commercial_use: true,
        modification: true,
        redistribution: true,
        attribution_required: true,
        attribution_text: Some(format!("IGN, BD TOPO 3.5, {stem}; modifications Mappa")),
        share_alike: false,
        adapter: match class {
            "water" => "ign-bdtopo-surface-water",
            "transport" => "ign-bdtopo-passenger-station",
            _ => "ign-bdtopo-road-segment",
        }.into(),
        adapter_version: 1,
    };
    let world = BBox {
        west: -180.0,
        south: -85.051_128_78,
        east: 180.0,
        north: 85.051_128_78,
    };
    let (features, rejected) = match class {
        "water" => adapt_ign_bdtopo_water(&source, world)?,
        "transport" => adapt_ign_bdtopo_stations(&source, world)?,
        _ => adapt_ign_bdtopo_roads(&source, world)?,
    };
    if features.is_empty() {
        for rejected in rejected.iter().take(8) {
            eprintln!(
                "excluded={} reason={}",
                rejected.source_feature_id, rejected.reason
            );
        }
        return Err("source yielded no eligible geometry".into());
    }
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for (feature, _) in &features {
        bounds[0] = bounds[0].min(feature.bbox.west);
        bounds[1] = bounds[1].min(feature.bbox.south);
        bounds[2] = bounds[2].max(feature.bbox.east);
        bounds[3] = bounds[3].max(feature.bbox.north);
    }
    source.file = relative_zip;
    source.upstream_file = Some(relative_archive);
    source.coverage = format!(
        "selected in-service {} geometry bounds {bounds:?}",
        match class {
            "water" => "permanent surface-water",
            "transport" => "real named passenger-station",
            _ => "vehicle-road",
        }
    );
    let manifest = SourceManifest {
        schema_version: 1,
        proof_region: stem.to_lowercase(),
        proof_bbox_wgs84: bounds,
        source: vec![source],
    };
    fs::write(output, toml::to_string_pretty(&manifest)?)?;
    SourceManifest::open(output)?;
    println!(
        "manifest={} accepted_parts={} rejected_records={} selected_geometry_bounds={bounds:?}",
        output.display(),
        features.len(),
        rejected.len()
    );
    Ok(())
}
