use mappa_map_core::{TileKey, project};
use mappa_map_data::{LocalPmTiles, TileSource, decode_mvt};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: audit_fixture FILE.pmtiles")?;
    let source = LocalPmTiles::open(Path::new(&path)).await?;
    let [west, south, east, north] = source.bounds;
    let mut checked = 0;
    let mut absent = 0;
    let mut failed = 0;
    let mut layer_counts = [0usize; 11];
    for z in source.min_zoom..=source.max_zoom {
        let n = (1u32 << z) as f64;
        let x0 = (project(west, 0.0)?.x * n).floor() as u32;
        let x1 = if east >= 180.0 {
            n as u32
        } else {
            (project(east, 0.0)?.x * n).ceil() as u32
        };
        let y0 = (project(0.0, north)?.y * n).floor() as u32;
        let y1 = (project(0.0, south)?.y * n).ceil() as u32;
        if x1 <= x0 || y1 <= y0 || (x1 - x0) as u64 * (y1 - y0) as u64 > 1_000_000 {
            return Err("archive bounds cover too many tiles for this audit".into());
        }
        for y in y0..y1 {
            for x in x0..x1 {
                let key = TileKey::new(z, x, y)?;
                match source.tile_bytes(key).await? {
                    Some(bytes) => {
                        checked += 1;
                        match decode_mvt(bytes) {
                            Ok(tile) => {
                                for (total, count) in layer_counts.iter_mut().zip([
                                    tile.land.len(),
                                    tile.green.len(),
                                    tile.water.len(),
                                    tile.road_surface.len(),
                                    tile.building.len(),
                                    tile.boundary.len(),
                                    tile.waterway.len(),
                                    tile.road.len() + tile.road_major.len(),
                                    tile.road_collector.len(),
                                    tile.road_local.len(),
                                    tile.place.len(),
                                ]) {
                                    *total += count;
                                }
                            }
                            Err(error) => {
                                failed += 1;
                                if failed <= 10 {
                                    eprintln!("z{z}/{x}/{y}: {error}");
                                }
                            }
                        }
                    }
                    None => absent += 1,
                }
            }
        }
    }
    println!(
        "{path}: decoded={checked}, absent={absent}, failures={failed}, layers (land, green, water, road_surface, building, boundary, waterway, major, collector, local, place)={layer_counts:?}"
    );
    if failed != 0 {
        return Err("invalid vector tiles found".into());
    }
    Ok(())
}
