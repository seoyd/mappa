//! Check that a wider replacement archive retains every old nonempty tile key.

use geo::LineString;
use mappa_map_core::{TileKey, project};
use mappa_map_data::{LocalPmTiles, TileSource, decode_mvt};
use std::{error::Error, path::Path};

fn contains_all_lines(old: &[LineString<f32>], new: &[LineString<f32>]) -> bool {
    let mut remaining = new.to_vec();
    for line in old {
        let Some(position) = remaining.iter().position(|candidate| candidate == line) else {
            return false;
        };
        remaining.swap_remove(position);
    }
    true
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: compare_pmtiles_coverage OLD.pmtiles NEW.pmtiles".into());
    }
    let old = LocalPmTiles::open(Path::new(&args[1])).await?;
    let new = LocalPmTiles::open(Path::new(&args[2])).await?;
    if new.min_zoom > old.min_zoom || new.max_zoom < old.max_zoom {
        return Err("replacement archive has a narrower zoom range".into());
    }
    let [west, south, east, north] = old.bounds;
    let mut old_tiles = 0usize;
    let mut equal = 0usize;
    let mut changed = 0usize;
    let mut missing = 0usize;
    let mut changed_without_old_roads = 0usize;
    for z in old.min_zoom..=old.max_zoom {
        let n = (1u32 << z) as f64;
        let x0 = (project(west, 0.0)?.x * n).floor() as u32;
        let x1 = (project(east, 0.0)?.x * n).ceil() as u32;
        let y0 = (project(0.0, north)?.y * n).floor() as u32;
        let y1 = (project(0.0, south)?.y * n).ceil() as u32;
        if x1 <= x0 || y1 <= y0 || (x1 - x0) as u64 * (y1 - y0) as u64 > 1_000_000 {
            return Err("comparison region too large".into());
        }
        for y in y0..y1 {
            for x in x0..x1 {
                let key = TileKey::new(z, x, y)?;
                let Some(before) = old.tile_bytes(key).await? else {
                    continue;
                };
                old_tiles += 1;
                let Some(after) = new.tile_bytes(key).await? else {
                    missing += 1;
                    continue;
                };
                let unchanged = before == after;
                let before = decode_mvt(before)?;
                let after = decode_mvt(after)?;
                if unchanged {
                    equal += 1;
                } else {
                    changed += 1;
                    if !contains_all_lines(&before.road_major, &after.road_major)
                        || !contains_all_lines(&before.road_collector, &after.road_collector)
                        || !contains_all_lines(&before.road_local, &after.road_local)
                    {
                        changed_without_old_roads += 1;
                    }
                }
            }
        }
    }
    println!(
        "old_nonempty_tiles={old_tiles} equal_bytes={equal} changed_bytes={changed} missing_in_replacement={missing} changed_without_old_roads={changed_without_old_roads}"
    );
    if missing > 0 || changed_without_old_roads > 0 {
        return Err("replacement omits old road geometry".into());
    }
    Ok(())
}
