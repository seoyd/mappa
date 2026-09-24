//! A fixture-only MVT versus stable MLT tag-01 comparison.
use flate2::{Compression, write::GzEncoder};
use mappa_map_core::TileKey;
use mappa_map_data::{LocalPmTiles, TileSource, decode_mvt};
use mlt_core::{Decoder, Parser, encoder::EncoderConfig, mvt::mvt_to_tile_layers};
use std::{io::Write, time::Instant};

fn gzip_len(bytes: &[u8]) -> Result<usize, std::io::Error> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes)?;
    Ok(encoder.finish()?.len())
}
fn decode_mlt(bytes: &[u8]) -> Result<usize, mlt_core::MltError> {
    let layers = Parser::default().parse_layers(bytes)?;
    let mut decoder = Decoder::with_max_size(128 * 1024 * 1024);
    let parsed = decoder.decode_all(layers)?;
    let mut features = 0;
    for layer in parsed {
        if let Some(layer) = layer.into_layer01() {
            features += layer.into_tile(&mut decoder)?.feature_count();
        }
    }
    Ok(features)
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let source = LocalPmTiles::open("assets/map/world_110m.pmtiles").await?;
    println!(
        "tile,mvt_bytes,mvt_gzip,mlt_v1_bytes,mlt_v1_gzip,mvt_decode_us,mlt_v1_decode_us,feature_count"
    );
    for key in [
        TileKey::new(0, 0, 0)?,
        TileKey::new(3, 6, 3)?,
        TileKey::new(4, 13, 6)?,
    ] {
        let Some(mvt) = source.tile_bytes(key).await? else {
            continue;
        };
        let mut mlt = Vec::new();
        for layer in mvt_to_tile_layers(&mvt)? {
            mlt.extend(layer.encode(EncoderConfig::default())?);
        }
        let mvt_count = {
            let decoded = decode_mvt(mvt.clone())?;
            decoded.land.len() + decoded.water.len() + decoded.boundary.len()
        };
        let mlt_count = decode_mlt(&mlt)?;
        if mvt_count != mlt_count {
            return Err(
                format!("feature mismatch for {key:?}: MVT {mvt_count}, MLT {mlt_count}").into(),
            );
        }
        let repetitions = 100;
        let start = Instant::now();
        for _ in 0..repetitions {
            std::hint::black_box(decode_mvt(mvt.clone())?);
        }
        let mvt_us = start.elapsed().as_secs_f64() * 1e6 / repetitions as f64;
        let start = Instant::now();
        for _ in 0..repetitions {
            std::hint::black_box(decode_mlt(&mlt)?);
        }
        let mlt_us = start.elapsed().as_secs_f64() * 1e6 / repetitions as f64;
        println!(
            "{}/{}/{},{},{},{},{},{:.2},{:.2},{}",
            key.z,
            key.x,
            key.y,
            mvt.len(),
            gzip_len(&mvt)?,
            mlt.len(),
            gzip_len(&mlt)?,
            mvt_us,
            mlt_us,
            mvt_count
        );
    }
    Ok(())
}
