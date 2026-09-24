//! Count identical road lines where two regional archives contain the same tile.

use geo::LineString;
use mappa_map_core::{TileKey, project};
use mappa_map_data::{LocalPmTiles, TileSource, decode_mvt};
use std::{collections::HashSet, error::Error, path::Path};

fn key(line: &LineString<f32>) -> Vec<(u32, u32)> {
    let mut points: Vec<_> = line
        .points()
        .map(|point| (point.x().to_bits(), point.y().to_bits()))
        .collect();
    if points.iter().cmp(points.iter().rev()).is_gt() {
        points.reverse();
    }
    points
}

fn matching_lines(first: &[LineString<f32>], second: &[LineString<f32>]) -> usize {
    let lines: HashSet<_> = first.iter().map(key).collect();
    second
        .iter()
        .filter(|line| lines.contains(&key(line)))
        .count()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: audit_pmtiles_road_overlap FIRST.pmtiles SECOND.pmtiles".into());
    }
    let first = LocalPmTiles::open(Path::new(&args[1])).await?;
    let second = LocalPmTiles::open(Path::new(&args[2])).await?;
    let west = first.bounds[0].max(second.bounds[0]);
    let south = first.bounds[1].max(second.bounds[1]);
    let east = first.bounds[2].min(second.bounds[2]);
    let north = first.bounds[3].min(second.bounds[3]);
    if east <= west || north <= south {
        println!("shared_nonempty_tiles=0 exact_duplicate_lines=0");
        return Ok(());
    }
    let mut shared_tiles = 0usize;
    let mut first_lines = 0usize;
    let mut second_lines = 0usize;
    let mut duplicates = 0usize;
    for z in first.min_zoom.max(second.min_zoom)..=first.max_zoom.min(second.max_zoom) {
        let n = (1u32 << z) as f64;
        let x0 = (project(west, 0.0)?.x * n).floor() as u32;
        let x1 = (project(east, 0.0)?.x * n).ceil() as u32;
        let y0 = (project(0.0, north)?.y * n).floor() as u32;
        let y1 = (project(0.0, south)?.y * n).ceil() as u32;
        if (x1 - x0) as u64 * (y1 - y0) as u64 > 1_000_000 {
            return Err("intersection too large for one audit".into());
        }
        for y in y0..y1 {
            for x in x0..x1 {
                let tile = TileKey::new(z, x, y)?;
                let Some(a) = first.tile_bytes(tile).await? else {
                    continue;
                };
                let Some(b) = second.tile_bytes(tile).await? else {
                    continue;
                };
                let a = decode_mvt(a)?;
                let b = decode_mvt(b)?;
                shared_tiles += 1;
                first_lines += a.road_major.len() + a.road_collector.len() + a.road_local.len();
                second_lines += b.road_major.len() + b.road_collector.len() + b.road_local.len();
                duplicates += matching_lines(&a.road_major, &b.road_major);
                duplicates += matching_lines(&a.road_collector, &b.road_collector);
                duplicates += matching_lines(&a.road_local, &b.road_local);
            }
        }
    }
    println!(
        "shared_nonempty_tiles={shared_tiles} first_road_lines={first_lines} second_road_lines={second_lines} exact_duplicate_lines={duplicates}"
    );
    Ok(())
}
