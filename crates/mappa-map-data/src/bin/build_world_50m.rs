use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let source = Path::new("assets/map/source/50m");
    let output = Path::new("assets/map/world_50m.pmtiles");
    let (tiles, features) = mappa_map_data::builder::build_world_50m(source, output, 7)?;
    println!(
        "{tiles} tiles, {features} features, {} bytes",
        std::fs::metadata(output)?.len()
    );
    Ok(())
}
