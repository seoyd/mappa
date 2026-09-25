//! Split a large approved canonical archive at a source-derived z10 tile column.
//! Tile payload bytes are copied exactly; the full local archive remains the proof baseline.

use futures_util::StreamExt;
use mappa_map_core::{WorldPoint, unproject};
use mappa_map_data::{LocalPmTiles, canonical::SourceManifest};
use pmtiles::{AsyncPmTilesReader, PmTilesWriter, TileCoord, TileType};
use std::{collections::BTreeSet, error::Error, fs::File, path::Path, sync::Arc};

type DynError = Box<dyn Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), DynError> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 && !(args.len() == 5 && args[4] == "--tight-bounds") {
        return Err(
            "usage: shard_canonical_pmtiles FULL.toml FULL.pmtiles OUTPUT_PREFIX [--tight-bounds]"
                .into(),
        );
    }
    let tight_bounds = args.len() == 5;
    let manifest_path = Path::new(&args[1]);
    let full_path = Path::new(&args[2]);
    let prefix = &args[3];
    let raw: SourceManifest = toml::from_str(&std::fs::read_to_string(manifest_path)?)?;
    let approved = SourceManifest::open(manifest_path)?;
    let archive = LocalPmTiles::open(full_path).await?;
    if archive.min_zoom != 10
        || archive.max_zoom != 15
        || archive
            .bounds
            .iter()
            .zip(approved.proof_bbox_wgs84)
            .any(|(actual, expected)| (actual - expected).abs() > 1e-6)
    {
        return Err("full archive header differs from its approved manifest".into());
    }
    let reader = Arc::new(AsyncPmTilesReader::new_with_path(full_path).await?);
    if reader.get_header().tile_type != TileType::Mvt {
        return Err("expected MVT source archive".into());
    }
    let metadata = reader.get_metadata().await?;
    for credit in approved
        .source
        .iter()
        .filter_map(|source| source.attribution_text.as_deref())
        .collect::<BTreeSet<_>>()
    {
        if !metadata.contains(credit) {
            return Err(format!("source archive omits required credit: {credit}").into());
        }
    }

    let [west, south, east, north] = approved.proof_bbox_wgs84;
    let x0 = ((west + 180.0) / 360.0 * 1024.0).floor() as u32;
    let x1 = ((east + 180.0) / 360.0 * 1024.0).ceil() as u32;
    let split_x = (x0 + x1) / 2;
    let split_lon = f64::from(split_x) / 1024.0 * 360.0 - 180.0;
    if split_lon <= west || split_lon >= east || split_x == 0 || split_x >= 1024 {
        return Err("source bounds do not span two z10 tile columns".into());
    }
    let mut shard_bounds = [
        [west, south, split_lon, north],
        [split_lon, south, east, north],
    ];
    if tight_bounds {
        shard_bounds = [
            [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ],
            [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ],
        ];
        let mut entries = Arc::clone(&reader).entries();
        while let Some(entry) = entries.next().await {
            for id in entry?.iter_coords() {
                let coord = TileCoord::from(id);
                if !(10..=15).contains(&coord.z()) {
                    return Err("source tile outside declared zoom range".into());
                }
                let side = usize::from(coord.x() >= split_x << (coord.z() - 10));
                let n = f64::from(1u32 << coord.z());
                let (tile_west, tile_north) = unproject(WorldPoint {
                    x: f64::from(coord.x()) / n,
                    y: f64::from(coord.y()) / n,
                })?;
                let (tile_east, tile_south) = unproject(WorldPoint {
                    x: f64::from(coord.x() + 1) / n,
                    y: f64::from(coord.y() + 1) / n,
                })?;
                let bounds = &mut shard_bounds[side];
                bounds[0] = bounds[0].min(tile_west);
                bounds[1] = bounds[1].min(tile_south);
                bounds[2] = bounds[2].max(tile_east);
                bounds[3] = bounds[3].max(tile_north);
            }
        }
        if shard_bounds.iter().any(|bounds| {
            bounds.iter().any(|value| !value.is_finite())
                || bounds[0] >= bounds[2]
                || bounds[1] >= bounds[3]
        }) {
            return Err("each tight-bounds shard must contain at least one tile".into());
        }
    }
    let full_stem = manifest_path
        .file_stem()
        .ok_or("manifest has no file stem")?
        .to_string_lossy();
    let manifest_dir = manifest_path.parent().ok_or("manifest has no parent")?;
    let west_manifest_path = manifest_dir.join(format!("{full_stem}_west.toml"));
    let east_manifest_path = manifest_dir.join(format!("{full_stem}_east.toml"));
    let west_path = format!("{prefix}-west.pmtiles");
    let east_path = format!("{prefix}-east.pmtiles");
    let mut west_manifest = raw.clone();
    west_manifest.proof_region.push_str("-west");
    west_manifest.proof_bbox_wgs84 = shard_bounds[0];
    let mut east_manifest = raw;
    east_manifest.proof_region.push_str("-east");
    east_manifest.proof_bbox_wgs84 = shard_bounds[1];
    std::fs::write(&west_manifest_path, toml::to_string_pretty(&west_manifest)?)?;
    std::fs::write(&east_manifest_path, toml::to_string_pretty(&east_manifest)?)?;
    SourceManifest::open(&west_manifest_path)?;
    SourceManifest::open(&east_manifest_path)?;

    let compression = reader.get_header().tile_compression;
    let make_writer = |path: &str, bounds: [f64; 4]| {
        PmTilesWriter::new(TileType::Mvt)
            .tile_compression(compression)
            .min_zoom(10)
            .max_zoom(15)
            .bounds(bounds[0], bounds[1], bounds[2], bounds[3])
            .center((bounds[0] + bounds[2]) / 2.0, (bounds[1] + bounds[3]) / 2.0)
            .center_zoom(13)
            .metadata(&metadata)
            .create(File::create(path)?)
    };
    let mut west_writer = make_writer(&west_path, west_manifest.proof_bbox_wgs84)?;
    let mut east_writer = make_writer(&east_path, east_manifest.proof_bbox_wgs84)?;
    let mut counts = [0usize; 2];
    let mut entries = Arc::clone(&reader).entries();
    while let Some(entry) = entries.next().await {
        for id in entry?.iter_coords() {
            let coord = TileCoord::from(id);
            if !(10..=15).contains(&coord.z()) {
                return Err("source tile outside declared zoom range".into());
            }
            let bytes = reader.get_tile(id).await?.ok_or("missing source tile")?;
            if coord.x() < split_x << (coord.z() - 10) {
                west_writer.add_raw_tile(coord, &bytes)?;
                counts[0] += 1;
            } else {
                east_writer.add_raw_tile(coord, &bytes)?;
                counts[1] += 1;
            }
        }
    }
    west_writer.finalize()?;
    east_writer.finalize()?;

    let west_reader = AsyncPmTilesReader::new_with_path(&west_path).await?;
    let east_reader = AsyncPmTilesReader::new_with_path(&east_path).await?;
    let mut verified = 0usize;
    let mut entries = Arc::clone(&reader).entries();
    while let Some(entry) = entries.next().await {
        for id in entry?.iter_coords() {
            let coord = TileCoord::from(id);
            let shard = if coord.x() < split_x << (coord.z() - 10) {
                &west_reader
            } else {
                &east_reader
            };
            if reader.get_tile(id).await? != shard.get_tile(id).await? {
                return Err(format!(
                    "shard changed source tile z{}/{}/{}",
                    coord.z(),
                    coord.x(),
                    coord.y()
                )
                .into());
            }
            verified += 1;
        }
    }
    if verified != counts[0] + counts[1] {
        return Err("shard tile counts changed during verification".into());
    }
    println!(
        "split_lon={split_lon} z10_split_x={split_x} west_tiles={} east_tiles={} byte_identical_tiles={verified} west_bytes={} east_bytes={} west_bounds={:?} east_bounds={:?}",
        counts[0],
        counts[1],
        std::fs::metadata(&west_path)?.len(),
        std::fs::metadata(&east_path)?.len(),
        west_manifest.proof_bbox_wgs84,
        east_manifest.proof_bbox_wgs84,
    );
    Ok(())
}
