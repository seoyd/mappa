//! Register audited LIST road packs in the offline world catalog.

use mappa_map_data::canonical::SourceManifest;
use std::{error::Error, fmt::Write, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: make_au_tas_catalog MANIFEST_DIR PACK_DIR OUTPUT.toml".into());
    }
    let manifest_dir = Path::new(&args[1]);
    let pack_dir = Path::new(&args[2]);
    let output_path = Path::new(&args[3]);
    let mut paths = std::fs::read_dir(manifest_dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    let mut catalog = String::from(
        "# Tasmania LIST Transport Segments; open public vehicular roads from audited REST pages.\n",
    );
    let mut count = 0;
    for manifest_path in paths {
        let Some(stem) = manifest_path.file_stem().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(region) = stem.strip_prefix("au_tas_roads_") else {
            continue;
        };
        if manifest_path.extension().and_then(|value| value.to_str()) != Some("toml") {
            continue;
        }
        let manifest = SourceManifest::open(&manifest_path)?;
        if manifest.source.len() != 1
            || manifest.source[0].adapter != "au-tas-list-transport-segments"
            || manifest.proof_region != format!("au-tas-list-{region}")
        {
            return Err(format!("invalid LIST manifest: {}", manifest_path.display()).into());
        }
        let pack_path = pack_dir.join(format!("{region}.pmtiles"));
        if !pack_path.is_file() {
            return Err(format!("missing LIST pack: {}", pack_path.display()).into());
        }
        let log = std::fs::read_to_string(pack_dir.join(format!("{region}-tile-audit.log")))?;
        let min_visible_zoom = (10..=15)
            .find(|zoom| {
                log.lines()
                    .find(|line| line.starts_with(&format!("z={zoom} ")))
                    .and_then(|line| {
                        line.split_whitespace()
                            .find_map(|field| field.strip_prefix("tiles="))
                    })
                    .and_then(|count| count.parse::<usize>().ok())
                    .is_some_and(|tiles| tiles > 0)
            })
            .ok_or("LIST tile audit has no visible tiles")?;
        let reported_bytes = log
            .lines()
            .find_map(|line| line.strip_prefix("archive_bytes="))
            .ok_or("LIST tile audit omits archive bytes")?
            .parse::<u64>()?;
        if reported_bytes != std::fs::metadata(&pack_path)?.len() {
            return Err("LIST archive changed after tile audit".into());
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
        let credit = manifest.source[0]
            .attribution_text
            .as_deref()
            .ok_or("LIST source lacks attribution")?;
        let label = format!("LIST · Tasmania · {credit}");
        if label.chars().count() > 180 {
            return Err("LIST label exceeds UI limit".into());
        }
        let label = toml::Value::String(label);
        write!(
            catalog,
            "\n[[pack]]\npath = \"../../{pack}\"\nmanifest = \"../../{source}\"\nmin_visible_zoom = {min_visible_zoom}\narchive_min_zoom = 10\narchive_max_zoom = 15\nlabel = {label}\n"
        )?;
        count += 1;
    }
    if count == 0 {
        return Err("no LIST road packs found".into());
    }
    std::fs::write(output_path, catalog)?;
    println!("tas_packs={count} catalog={}", output_path.display());
    Ok(())
}
