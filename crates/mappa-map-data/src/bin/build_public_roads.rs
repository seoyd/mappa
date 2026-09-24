use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 6 {
        return Err(
            "usage: build_public_roads LAND.geojson ROADS_WGS84.geojson ROAD_SURFACE_WGS84.geojson OUTPUT.pmtiles MIN_Z MAX_Z"
                .into(),
        );
    }
    let output = Path::new(&args[3]);
    let (tiles, features) = mappa_map_data::builder::build_public_roads_fixture(
        Path::new(&args[0]),
        Path::new(&args[1]),
        Path::new(&args[2]),
        output,
        args[4].parse()?,
        args[5].parse()?,
    )?;
    println!(
        "{tiles} tiles, {features} features, {} bytes",
        std::fs::metadata(output)?.len()
    );
    Ok(())
}
