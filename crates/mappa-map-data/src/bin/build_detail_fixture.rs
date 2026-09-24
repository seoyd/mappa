use std::{path::Path, str::FromStr};

fn parse<T: FromStr>(value: &str) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    T::Err: std::error::Error + Send + Sync + 'static,
{
    Ok(value.parse()?)
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 10 {
        return Err("usage: build_detail_fixture SOURCE_DIR OUTPUT MIN_Z MAX_Z X0 Y0 X1 Y1 ROAD_MAX_RANK PLACE_MAX_RANK".into());
    }
    let source = Path::new(&args[0]);
    let output = Path::new(&args[1]);
    let min_zoom = parse(&args[2])?;
    let max_zoom = parse(&args[3])?;
    let bounds = [
        parse(&args[4])?,
        parse(&args[5])?,
        parse(&args[6])?,
        parse(&args[7])?,
    ];
    let max_road_rank = parse(&args[8])?;
    let max_place_rank = parse(&args[9])?;
    let (tiles, features) = mappa_map_data::builder::build_detail_fixture(
        source,
        output,
        min_zoom,
        max_zoom,
        bounds,
        max_road_rank,
        max_place_rank,
    )?;
    println!(
        "{tiles} tiles, {features} features, {} bytes",
        std::fs::metadata(output)?.len()
    );
    Ok(())
}
