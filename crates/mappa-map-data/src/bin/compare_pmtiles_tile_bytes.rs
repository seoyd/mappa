//! Compare every tile coordinate and payload in two PMTiles archives.

use futures_util::StreamExt;
use pmtiles::{AsyncPmTilesReader, TileCoord};
use std::{error::Error, path::Path, sync::Arc};

type DynError = Box<dyn Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), DynError> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: compare_pmtiles_tile_bytes FIRST.pmtiles SECOND.pmtiles".into());
    }
    let first = Arc::new(AsyncPmTilesReader::new_with_path(Path::new(&args[1])).await?);
    let second = Arc::new(AsyncPmTilesReader::new_with_path(Path::new(&args[2])).await?);
    let mut count = 0usize;
    let mut changed = 0usize;
    let mut entries = Arc::clone(&first).entries();
    while let Some(entry) = entries.next().await {
        for id in entry?.iter_coords() {
            let coordinate = TileCoord::from(id);
            let before = first.get_tile(id).await?.ok_or("missing first tile")?;
            let after = second.get_tile(id).await?.ok_or("missing second tile")?;
            if before != after {
                changed += 1;
                if changed <= 10 {
                    eprintln!(
                        "changed z={} x={} y={}",
                        coordinate.z(),
                        coordinate.x(),
                        coordinate.y()
                    );
                }
            }
            count += 1;
        }
    }
    let mut second_count = 0usize;
    let mut entries = Arc::clone(&second).entries();
    while let Some(entry) = entries.next().await {
        for _ in entry?.iter_coords() {
            second_count += 1;
        }
    }
    println!("first_tiles={count} second_tiles={second_count} changed_payloads={changed}");
    if count != second_count || changed > 0 {
        return Err("tile payloads differ".into());
    }
    Ok(())
}
