//! Register every audited Queensland LGA road pack in the offline catalog.

use mappa_map_data::canonical::SourceManifest;
use std::{error::Error, fmt::Write, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: make_au_qld_qrt_catalog MANIFEST_DIR PACK_DIR OUTPUT.toml".into());
    }
    let manifest_dir = Path::new(&args[1]);
    let pack_dir = Path::new(&args[2]);
    let output_path = Path::new(&args[3]);
    let mut paths = std::fs::read_dir(manifest_dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    let mut catalog = String::from(
        "# Queensland Roads and Tracks source LGAs; built from audited GeoJSON pages.\n",
    );
    let mut count = 0;
    for manifest_path in paths {
        let Some(stem) = manifest_path.file_stem().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(code) = stem.strip_prefix("au_qld_qrt_roads_") else {
            continue;
        };
        if manifest_path.extension().and_then(|value| value.to_str()) != Some("toml") {
            continue;
        }
        let manifest = SourceManifest::open(&manifest_path)?;
        if manifest.source.len() != 1
            || manifest.source[0].adapter != "au-qld-qrt-roads"
            || manifest.proof_region != format!("au-qld-qrt-{code}")
        {
            return Err(format!("invalid Queensland manifest: {}", manifest_path.display()).into());
        }
        let pack_path = pack_dir.join(format!("{code}.pmtiles"));
        if !pack_path.is_file() {
            return Err(format!("missing Queensland pack: {}", pack_path.display()).into());
        }
        let region = manifest.source[0]
            .coverage
            .strip_prefix("Queensland Roads and Tracks; source LGA assignment=")
            .ok_or("Queensland manifest omits its source LGA")?;
        let log = std::fs::read_to_string(pack_dir.join(format!("{code}-tile-audit.log")))?;
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
            .ok_or("Queensland tile audit has no visible tiles")?;
        let reported_bytes = log
            .lines()
            .find_map(|line| line.strip_prefix("archive_bytes="))
            .ok_or("Queensland tile audit omits archive bytes")?
            .parse::<u64>()?;
        if reported_bytes != std::fs::metadata(&pack_path)?.len() {
            return Err("Queensland archive changed after tile audit".into());
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
            .ok_or("Queensland source lacks attribution")?;
        let label = format!("QRT · {region} · {credit}");
        if label.chars().count() > 180 {
            return Err(format!("Queensland label exceeds UI limit: {region}").into());
        }
        let label = toml::Value::String(label);
        write!(
            catalog,
            "\n[[pack]]\npath = \"../../{pack}\"\nmanifest = \"../../{source}\"\nmin_visible_zoom = {min_visible_zoom}\narchive_min_zoom = 10\narchive_max_zoom = 15\nlabel = {label}\n"
        )?;
        count += 1;
    }
    if count == 0 {
        return Err("no Queensland road packs found".into());
    }
    std::fs::write(output_path, catalog)?;
    println!("qld_packs={count} catalog={}", output_path.display());
    Ok(())
}
