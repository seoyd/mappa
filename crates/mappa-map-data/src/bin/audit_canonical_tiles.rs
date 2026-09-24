use mappa_map_core::{TileKey, project};
use mappa_map_data::{LocalPmTiles, TileSource, canonical::SourceManifest, decode_mvt};
use std::{error::Error, path::Path};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: audit_canonical_tiles data/sources.toml INPUT.pmtiles".into());
    }
    let manifest = SourceManifest::open(Path::new(&args[1]))?;
    let archive = LocalPmTiles::open(Path::new(&args[2])).await?;
    let [west, south, east, north] = manifest.proof_bbox_wgs84;
    let nw = project(west, north)?;
    let se = project(east, south)?;
    for z in archive.min_zoom..=archive.max_zoom {
        let n = (1u32 << z) as f64;
        let mut tiles = 0;
        let mut bytes = 0;
        let mut roads = 0;
        let mut surfaces = 0;
        for y in (nw.y * n).floor() as u32..=(se.y * n).floor() as u32 {
            for x in (nw.x * n).floor() as u32..=(se.x * n).floor() as u32 {
                let Some(payload) = archive.tile_bytes(TileKey::new(z, x, y)?).await? else {
                    continue;
                };
                bytes += payload.len();
                tiles += 1;
                let decoded = decode_mvt(payload)?;
                if !decoded.land.is_empty()
                    || !decoded.water.is_empty()
                    || !decoded.green.is_empty()
                    || !decoded.place.is_empty()
                {
                    return Err("proof contains a non-road layer".into());
                }
                roads += decoded.road_major.len()
                    + decoded.road_collector.len()
                    + decoded.road_local.len();
                surfaces += decoded.road_surface.len();
            }
        }
        println!(
            "z={z} tiles={tiles} decoded_mvt_bytes={bytes} road_lines={roads} road_surfaces={surfaces}"
        );
    }
    println!("archive_bytes={}", std::fs::metadata(&args[2])?.len());
    Ok(())
}
