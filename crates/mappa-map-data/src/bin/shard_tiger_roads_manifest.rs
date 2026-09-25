//! Partition a large, approved TIGER road manifest by observed county longitude and DBF rows.

use mappa_map_data::canonical::{SourceManifest, SourceRecord};
use std::{
    collections::BTreeSet,
    error::Error,
    fs::{self, File},
    io::Read,
    path::Path,
};
use zip::ZipArchive;

struct County {
    id: String,
    center_lon: f64,
    bounds: [f64; 4],
    rows: u64,
}

fn county(manifest_path: &Path, source: &SourceRecord) -> Result<County, Box<dyn Error>> {
    if source.adapter != "us-census-tiger-roads" {
        return Err("expected only Census TIGER road sources".into());
    }
    let path = manifest_path
        .parent()
        .ok_or("manifest has no parent directory")?
        .join(&source.file);
    let mut archive = ZipArchive::new(File::open(path)?)?;
    let shp_name = archive
        .file_names()
        .find(|name| name.ends_with(".shp"))
        .ok_or("source ZIP has no .shp")?
        .to_owned();
    let dbf_name = archive
        .file_names()
        .find(|name| name.ends_with(".dbf"))
        .ok_or("source ZIP has no .dbf")?
        .to_owned();
    let mut shp = [0u8; 100];
    archive.by_name(&shp_name)?.read_exact(&mut shp)?;
    let mut dbf = [0u8; 32];
    archive.by_name(&dbf_name)?.read_exact(&mut dbf)?;
    let mut bounds = [0.0; 4];
    for (index, value) in bounds.iter_mut().enumerate() {
        let start = 36 + index * 8;
        *value = f64::from_le_bytes(shp[start..start + 8].try_into()?);
    }
    if !bounds.iter().all(|value| value.is_finite()) || bounds[0] >= bounds[2] {
        return Err("invalid county source longitude bounds".into());
    }
    Ok(County {
        id: source.id.clone(),
        center_lon: (bounds[0] + bounds[2]) / 2.0,
        bounds,
        rows: u64::from(u32::from_le_bytes(dbf[4..8].try_into()?)),
    })
}

fn union_bounds(counties: &[County], ids: &BTreeSet<String>) -> [f64; 4] {
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for county in counties.iter().filter(|county| ids.contains(&county.id)) {
        bounds[0] = bounds[0].min(county.bounds[0]);
        bounds[1] = bounds[1].min(county.bounds[1]);
        bounds[2] = bounds[2].max(county.bounds[2]);
        bounds[3] = bounds[3].max(county.bounds[3]);
    }
    bounds
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: shard_tiger_roads_manifest FULL_ROADS.toml".into());
    }
    let path = Path::new(&args[1]);
    SourceManifest::open(path)?;
    let full: SourceManifest = toml::from_str(&fs::read_to_string(path)?)?;
    if full.source.len() < 2 {
        return Err("at least two county sources are required".into());
    }
    let mut counties = full
        .source
        .iter()
        .map(|source| county(path, source))
        .collect::<Result<Vec<_>, _>>()?;
    counties.sort_by(|a, b| a.center_lon.total_cmp(&b.center_lon).then(a.id.cmp(&b.id)));
    let total: u64 = counties.iter().map(|county| county.rows).sum();
    let mut west_rows = 0u64;
    let mut best = None;
    for split in 1..counties.len() {
        west_rows += counties[split - 1].rows;
        let east_rows = total - west_rows;
        if west_rows > 1_000_000 || east_rows > 1_000_000 {
            continue;
        }
        let difference = west_rows.abs_diff(east_rows);
        if best.is_none_or(|(_, best_difference)| difference < best_difference) {
            best = Some((split, difference));
        }
    }
    let (split, _) = best.ok_or("two county shards cannot fit the GeoDB feature limit")?;
    let west_ids = counties[..split]
        .iter()
        .map(|county| county.id.clone())
        .collect::<BTreeSet<_>>();
    let east_ids = counties[split..]
        .iter()
        .map(|county| county.id.clone())
        .collect::<BTreeSet<_>>();
    let stem = path
        .file_stem()
        .ok_or("manifest has no file stem")?
        .to_string_lossy();
    let parent = path.parent().ok_or("manifest has no parent directory")?;
    for (side, ids) in [("west", &west_ids), ("east", &east_ids)] {
        let output = parent.join(format!("{stem}_{side}.toml"));
        let mut shard = full.clone();
        shard.proof_region = format!("{}-{side}", full.proof_region);
        shard.proof_bbox_wgs84 = union_bounds(&counties, ids);
        shard.source.retain(|source| ids.contains(&source.id));
        fs::write(&output, toml::to_string_pretty(&shard)?)?;
        SourceManifest::open(&output)?;
        println!(
            "manifest={} counties={} dbf_rows={} bbox={:?}",
            output.display(),
            ids.len(),
            counties
                .iter()
                .filter(|county| ids.contains(&county.id))
                .map(|county| county.rows)
                .sum::<u64>(),
            shard.proof_bbox_wgs84
        );
    }
    if west_ids.len() + east_ids.len() != full.source.len() {
        return Err("partition did not account for every source".into());
    }
    Ok(())
}
