use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let source = Path::new("assets/map/source");
    let output = Path::new("assets/map/world_110m.pmtiles");
    let (tiles, features) = mappa_map_data::builder::build_fixture(source, output, 4)?;
    println!(
        "{tiles} tiles, {features} features, {} bytes",
        std::fs::metadata(output)?.len()
    );
    Ok(())
}
