use std::{path::Path, str::FromStr};

fn parse<T: FromStr>(value: &str) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    T::Err: std::error::Error + Send + Sync + 'static,
{
    Ok(value.parse()?)
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 14 {
        return Err(
            "usage: build_street_fixture NATURAL_EARTH_DIR OSM_LAND OSM_WATER OSM_GREEN OSM_ROADS OSM_POINTS OSM_AREAS OUTPUT MIN_Z MAX_Z X0 Y0 X1 Y1"
                .into(),
        );
    }
    let bounds = [
        parse(&args[10])?,
        parse(&args[11])?,
        parse(&args[12])?,
        parse(&args[13])?,
    ];
    let output = Path::new(&args[7]);
    let sources = mappa_map_data::builder::OsmSources {
        natural_earth_dir: Path::new(&args[0]),
        land: Path::new(&args[1]),
        water: Path::new(&args[2]),
        green: Path::new(&args[3]),
        roads: Path::new(&args[4]),
        points: Path::new(&args[5]),
        areas: Path::new(&args[6]),
    };
    let (tiles, features) = mappa_map_data::builder::build_osm_fixture(
        sources,
        output,
        parse(&args[8])?,
        parse(&args[9])?,
        bounds,
    )?;
    println!(
        "{tiles} tiles, {features} features, {} bytes",
        std::fs::metadata(output)?.len()
    );
    Ok(())
}
