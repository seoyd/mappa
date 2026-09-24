use futures_util::StreamExt;
use mappa_map_core::TileKey;
use mappa_map_data::{LocalPmTiles, TileSource, decode_mvt};
use pmtiles::{AsyncPmTilesReader, TileCoord};
use std::{path::Path, sync::Arc};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: audit_fixture FILE.pmtiles")?;
    let source = LocalPmTiles::open(Path::new(&path)).await?;
    let directory = Arc::new(AsyncPmTilesReader::new_with_path(Path::new(&path)).await?);
    let expected = directory.get_header().n_addressed_tiles().map(|n| n.get());
    let mut entries = directory.entries();
    let mut checked = 0;
    let mut absent = 0;
    let mut failed = 0;
    let mut layer_counts = [0usize; 11];
    while let Some(entry) = entries.next().await {
        for id in entry?.iter_coords() {
            let coord = TileCoord::from(id);
            let (z, x, y) = (coord.z(), coord.x(), coord.y());
            if z < source.min_zoom || z > source.max_zoom {
                return Err(format!("directory tile outside zoom bounds: z{z}/{x}/{y}").into());
            }
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
    println!(
        "{path}: decoded={checked}, absent={absent}, failures={failed}, layers (land, green, water, road_surface, building, boundary, waterway, major, collector, local, place)={layer_counts:?}"
    );
    if failed != 0 || absent != 0 || expected.is_some_and(|n| n != (checked + absent) as u64) {
        return Err("invalid vector tiles found".into());
    }
    Ok(())
}
