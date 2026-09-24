//! Generate catalog entries only for built, manifest-matched GB road archives.

use mappa_map_data::{LocalPmTiles, canonical::SourceManifest};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, error::Error, fs::File, path::Path};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err(
            "usage: make_os_open_roads_catalog ROADLINK_INDEX.tsv REPO_ROOT OUTPUT.toml PACK_INDEX.tsv".into(),
        );
    }
    let root = Path::new(&args[2]);
    let index = std::fs::read_to_string(&args[1])?;
    let mut lines = index.lines();
    if lines.next()
        != Some("member\tdbf_records\teasting_min\tnorthing_min\teasting_max\tnorthing_max")
    {
        return Err("unexpected OS RoadLink index header".into());
    }
    let mut grids = BTreeSet::new();
    for line in lines {
        let member = line.split('\t').next().ok_or("invalid RoadLink index")?;
        let grid = member
            .strip_prefix("data/")
            .and_then(|name| name.strip_suffix("_RoadLink.shp"))
            .ok_or("invalid RoadLink member")?;
        if grid.len() != 2 || !grid.bytes().all(|byte| byte.is_ascii_uppercase()) {
            return Err("invalid OS grid code".into());
        }
        if !grids.insert(grid.to_owned()) {
            return Err("duplicate OS grid in source index".into());
        }
    }
    if grids.is_empty() {
        return Err("no audited RoadLink grids".into());
    }
    let mut output =
        String::from("# Generated from audited OS Open Roads manifests and local PMTiles.\n");
    let mut inventory = String::from("grid\tpmtiles_bytes\tsha256\twest\tsouth\teast\tnorth\n");
    let mut total_bytes = 0u64;
    for grid in &grids {
        let lower = grid.to_ascii_lowercase();
        let manifest_rel = format!("data/os_open_roads_{lower}.toml");
        let archive_rel = format!("artifacts/world-roads/gb/{lower}.pmtiles");
        let manifest = SourceManifest::open(&root.join(&manifest_rel))?;
        let [source] = manifest.source.as_slice() else {
            return Err(format!("{grid} manifest must have one source").into());
        };
        if source.adapter != "os-open-roads"
            || source.adapter_version != 2
            || manifest.proof_region != format!("os-open-roads-{grid}")
        {
            return Err(format!("{grid} manifest adapter or region mismatch").into());
        }
        let credit = source
            .attribution_text
            .as_deref()
            .ok_or("OS credit missing")?;
        let archive_path = root.join(&archive_rel);
        let archive = LocalPmTiles::open(&archive_path).await?;
        if archive.min_zoom != 10
            || archive.max_zoom != 15
            || archive
                .bounds
                .iter()
                .zip(manifest.proof_bbox_wgs84)
                .any(|(actual, expected)| (actual - expected).abs() > 1e-6)
            || !archive
                .attribution
                .as_deref()
                .is_some_and(|text| text.contains(credit))
        {
            return Err(format!("{grid} PMTiles header differs from manifest").into());
        }
        let label = format!("GB {grid} · {credit}");
        if label.chars().count() > 180
            || label
                .chars()
                .any(|character| matches!(character, '\n' | '\r' | '"' | '\\'))
        {
            return Err(format!("{grid} credit cannot fit the catalog label").into());
        }
        output.push_str(&format!(
            "\n[[pack]]\npath = \"../../{archive_rel}\"\nmanifest = \"../../{manifest_rel}\"\nmin_visible_zoom = 10\narchive_min_zoom = 10\narchive_max_zoom = 15\nlabel = \"{label}\"\n"
        ));
        let bytes = std::fs::metadata(&archive_path)?.len();
        let mut hash = Sha256::new();
        std::io::copy(&mut File::open(&archive_path)?, &mut hash)?;
        let [west, south, east, north] = archive.bounds;
        inventory.push_str(&format!(
            "{grid}\t{bytes}\t{:x}\t{west}\t{south}\t{east}\t{north}\n",
            hash.finalize()
        ));
        total_bytes += bytes;
    }
    std::fs::write(&args[3], output)?;
    std::fs::write(&args[4], inventory)?;
    println!(
        "gb_grid_packs={} total_pmtiles_bytes={total_bytes}",
        grids.len()
    );
    Ok(())
}
