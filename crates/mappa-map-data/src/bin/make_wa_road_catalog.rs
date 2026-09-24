//! List every built WA source region as an offline map pack.

use mappa_map_data::canonical::SourceManifest;
use std::{error::Error, fmt::Write, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: make_wa_road_catalog MANIFEST_DIR PACK_DIR OUTPUT.toml".into());
    }
    let manifest_dir = Path::new(&args[1]);
    let pack_dir = Path::new(&args[2]);
    let output_path = Path::new(&args[3]);
    let mut paths = std::fs::read_dir(manifest_dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    let mut catalog = String::from(
        "# Main Roads Western Australia source regions; built offline from the approved ZIP.\n",
    );
    let mut count = 0;
    for manifest_path in paths {
        let Some(stem) = manifest_path.file_stem().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(code) = stem.strip_prefix("au_wa_roads_") else {
            continue;
        };
        if manifest_path.extension().and_then(|value| value.to_str()) != Some("toml") {
            continue;
        }
        let manifest = SourceManifest::open(&manifest_path)?;
        if manifest.source.len() != 1
            || manifest.source[0].adapter != "au-wa-road-network"
            || manifest.proof_region != format!("au-wa-{code}")
        {
            return Err(format!("invalid WA manifest: {}", manifest_path.display()).into());
        }
        let pack_path = pack_dir.join(format!("{code}.pmtiles"));
        if !pack_path.is_file() {
            return Err(format!("missing WA pack: {}", pack_path.display()).into());
        }
        let region = manifest.source[0]
            .coverage
            .strip_prefix("WA Road Network source RA_NAME=")
            .ok_or("WA manifest omits its source region name")?;
        let log = std::fs::read_to_string(pack_dir.join(format!("{code}-tile-audit.log")))?;
        let mut visible_zooms = Vec::new();
        for zoom in 10..=15 {
            let line = log
                .lines()
                .find(|line| line.starts_with(&format!("z={zoom} ")))
                .ok_or("WA tile audit is missing a zoom")?;
            let tiles = line
                .split_whitespace()
                .find_map(|field| field.strip_prefix("tiles="))
                .ok_or("WA tile audit omits tile count")?
                .parse::<usize>()?;
            if tiles > 0 {
                visible_zooms.push(zoom);
            }
        }
        let min_visible_zoom = visible_zooms
            .first()
            .ok_or("WA archive has no visible tiles")?;
        let reported_bytes = log
            .lines()
            .find_map(|line| line.strip_prefix("archive_bytes="))
            .ok_or("WA tile audit omits archive bytes")?
            .parse::<u64>()?;
        if reported_bytes != std::fs::metadata(&pack_path)?.len() {
            return Err("WA archive changed after tile audit".into());
        }
        let catalog_dir = output_path.parent().ok_or("catalog output has no parent")?;
        let root = catalog_dir.join("../..").canonicalize()?;
        let pack = pack_path
            .canonicalize()?
            .strip_prefix(&root)?
            .to_string_lossy()
            .into_owned();
        let source = manifest_path
            .canonicalize()?
            .strip_prefix(&root)?
            .to_string_lossy()
            .into_owned();
        let label = toml::Value::String(format!("Main Roads WA · {region} · CC BY 4.0"));
        write!(
            catalog,
            "\n[[pack]]\npath = \"../../{pack}\"\nmanifest = \"../../{source}\"\nmin_visible_zoom = {min_visible_zoom}\narchive_min_zoom = 10\narchive_max_zoom = 15\nlabel = {label}\n"
        )?;
        count += 1;
    }
    if count == 0 {
        return Err("no WA source region packs found".into());
    }
    std::fs::write(output_path, catalog)?;
    println!("wa_packs={count} catalog={}", output_path.display());
    Ok(())
}
