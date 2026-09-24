//! Report which coarse world cells contain each layer in a local PMTiles archive.
//! Layer presence is not a claim that roads, settlements, or coastlines are complete.

use mappa_map_core::TileKey;
use mappa_map_data::{LocalPmTiles, TileSource, decode_mvt};
use std::{fs::File, io::Write, path::Path};

type DynError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Clone, Copy, Default)]
struct Cell {
    decoded: u32,
    absent: u32,
    land: u32,
    water: u32,
    waterway: u32,
    boundary: u32,
    road: u32,
    place: u32,
}

#[tokio::main]
async fn main() -> Result<(), DynError> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: audit_world_tile_presence FILE.pmtiles GROUP_ZOOM OUTPUT.csv".into());
    }
    let archive = LocalPmTiles::open(Path::new(&args[0])).await?;
    let group_zoom: u8 = args[1].to_string_lossy().parse()?;
    let zoom = archive.max_zoom;
    if archive.bounds[0] > -180.0 || archive.bounds[2] < 180.0 || group_zoom >= zoom || zoom > 8 {
        return Err("expected a full-longitude world archive at zoom <= 8".into());
    }
    let n = 1u32 << zoom;
    let groups = 1u32 << group_zoom;
    let mut cells = vec![Cell::default(); (groups * groups) as usize];
    for y in 0..n {
        for x in 0..n {
            let index = ((y >> (zoom - group_zoom)) * groups + (x >> (zoom - group_zoom))) as usize;
            let cell = &mut cells[index];
            match archive.tile_bytes(TileKey::new(zoom, x, y)?).await? {
                Some(bytes) => {
                    let tile = decode_mvt(bytes)?;
                    cell.decoded += 1;
                    cell.land += u32::from(!tile.land.is_empty());
                    cell.water += u32::from(!tile.water.is_empty());
                    cell.waterway += u32::from(!tile.waterway.is_empty());
                    cell.boundary += u32::from(!tile.boundary.is_empty());
                    cell.road += u32::from(
                        !tile.road.is_empty()
                            || !tile.road_major.is_empty()
                            || !tile.road_collector.is_empty()
                            || !tile.road_local.is_empty(),
                    );
                    cell.place += u32::from(!tile.place.is_empty());
                }
                None => cell.absent += 1,
            }
        }
    }
    let mut output = File::create(&args[2])?;
    writeln!(
        output,
        "group_z,x,y,west,south,east,north,decoded_tiles,absent_tiles,land_tiles,water_tiles,waterway_tiles,boundary_tiles,road_tiles,place_tiles"
    )?;
    let mut land_cells = 0;
    let mut road_cells = 0;
    let mut waterway_cells = 0;
    let mut land_without_road_cells = 0;
    for y in 0..groups {
        for x in 0..groups {
            let cell = cells[(y * groups + x) as usize];
            land_cells += usize::from(cell.land > 0);
            road_cells += usize::from(cell.road > 0);
            waterway_cells += usize::from(cell.waterway > 0);
            land_without_road_cells += usize::from(cell.land > 0 && cell.road == 0);
            let west = x as f64 / groups as f64 * 360.0 - 180.0;
            let east = (x + 1) as f64 / groups as f64 * 360.0 - 180.0;
            let lat = |row: u32| {
                (std::f64::consts::PI * (1.0 - 2.0 * row as f64 / groups as f64))
                    .sinh()
                    .atan()
                    .to_degrees()
            };
            writeln!(
                output,
                "{group_zoom},{x},{y},{west:.6},{:.6},{east:.6},{:.6},{},{},{},{},{},{},{},{}",
                lat(y + 1),
                lat(y),
                cell.decoded,
                cell.absent,
                cell.land,
                cell.water,
                cell.waterway,
                cell.boundary,
                cell.road,
                cell.place
            )?;
        }
    }
    println!(
        "z{zoom} source presence: {} group cells; {land_cells} with land, {waterway_cells} with a waterway, {road_cells} with a road, {land_without_road_cells} with land and no road; {} decoded tiles, {} absent tiles. Presence is not completeness.",
        cells.len(),
        cells.iter().map(|cell| cell.decoded).sum::<u32>(),
        cells.iter().map(|cell| cell.absent).sum::<u32>()
    );
    Ok(())
}
