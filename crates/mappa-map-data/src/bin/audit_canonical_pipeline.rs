//! Measure source GeoJSON → adapter → GeoDB → z15 decoded road geometry loss.
//! This does not measure the accuracy of the government's original survey.

use geo::LineString;
use geojson::{GeoJson, GeometryValue};
use mappa_map_core::{MapCamera, TileKey, WorldPoint, project, unproject};
use mappa_map_data::{
    DecodedTile, LocalPmTiles, TileSource,
    canonical::{BBox, FeatureKind, GeoDb, Geometry, SourceManifest, adapt_naju_roads},
    decode_mvt,
};
use std::{collections::BTreeMap, error::Error, fs, path::Path, str::FromStr};

type DynError = Box<dyn Error + Send + Sync>;

fn nearest_line_distance(point: [f64; 2], lines: &[LineString<f32>]) -> f64 {
    let mut nearest = f64::INFINITY;
    for line in lines {
        for segment in line.0.windows(2) {
            let a = [segment[0].x as f64, segment[0].y as f64];
            let b = [segment[1].x as f64, segment[1].y as f64];
            let delta = [b[0] - a[0], b[1] - a[1]];
            let squared = delta[0] * delta[0] + delta[1] * delta[1];
            if squared == 0.0 {
                continue;
            }
            let fraction = (((point[0] - a[0]) * delta[0] + (point[1] - a[1]) * delta[1])
                / squared)
                .clamp(0.0, 1.0);
            let dx = point[0] - a[0] - fraction * delta[0];
            let dy = point[1] - a[1] - fraction * delta[1];
            nearest = nearest.min(dx.hypot(dy));
        }
    }
    nearest
}

fn sample_world_point(line: &[[f64; 2]], region: BBox) -> Result<Option<WorldPoint>, DynError> {
    let mut longest = None;
    for segment in line.windows(2) {
        let a = project(segment[0][0], segment[0][1])?;
        let b = project(segment[1][0], segment[1][1])?;
        let point = WorldPoint {
            x: (a.x + b.x) / 2.0,
            y: (a.y + b.y) / 2.0,
        };
        let (lon, lat) = unproject(point)?;
        if lon < region.west || lon > region.east || lat < region.south || lat > region.north {
            continue;
        }
        let length = (b.x - a.x).hypot(b.y - a.y);
        if longest.is_none_or(|(previous, _)| length > previous) {
            longest = Some((length, point));
        }
    }
    Ok(longest.map(|(_, point)| point))
}

fn percentile(sorted: &[f64], percentage: usize) -> f64 {
    sorted[((sorted.len() - 1) * percentage) / 100]
}

async fn audit(manifest_path: &Path, geodb_path: &Path, tiles_path: &Path) -> Result<(), DynError> {
    let manifest = SourceManifest::open(manifest_path)?;
    let source = manifest
        .source
        .iter()
        .find(|source| source.adapter == "naju-road-centerline")
        .ok_or("missing road centerline source")?;
    let [west, south, east, north] = manifest.proof_bbox_wgs84;
    let region = BBox {
        west,
        south,
        east,
        north,
    };
    let GeoJson::FeatureCollection(raw) = GeoJson::from_str(&fs::read_to_string(&source.file)?)?
    else {
        return Err("expected source FeatureCollection".into());
    };
    let mut raw_lines = BTreeMap::new();
    for feature in raw.features {
        let id = feature
            .property("gid")
            .and_then(|value| value.as_i64())
            .ok_or("missing source road gid")?
            .to_string();
        let Some(geojson::Geometry {
            value: GeometryValue::LineString { coordinates },
            ..
        }) = feature.geometry
        else {
            return Err("expected source road LineString".into());
        };
        let line = coordinates
            .into_iter()
            .map(|coordinate| {
                let [lon, lat] = coordinate.as_slice() else {
                    return Err("invalid source road coordinate");
                };
                Ok([*lon, *lat])
            })
            .collect::<Result<Vec<_>, _>>()?;
        raw_lines.insert(id, line);
    }
    let adapted = adapt_naju_roads(source, region)?;
    let mut database = GeoDb::open(geodb_path)?;
    let canonical = database
        .query(region)?
        .into_iter()
        .map(|feature| (feature.id, feature))
        .collect::<BTreeMap<_, _>>();
    let archive = LocalPmTiles::open(tiles_path).await?;
    let mut decoded_tiles: BTreeMap<TileKey, DecodedTile> = BTreeMap::new();
    let mut errors_units = Vec::new();
    let mut errors_meters = Vec::new();
    let mut errors_screen_px = Vec::new();
    let step = (adapted.len() / 120).max(1);
    for (feature, provenance) in adapted.iter().step_by(step) {
        if errors_units.len() == 100 {
            break;
        }
        let Geometry::Line(points) = &feature.geometry else {
            return Err("road adapter produced non-line geometry".into());
        };
        if raw_lines.get(&provenance.source_feature_id) != Some(points) {
            return Err("source GeoJSON → adapter changed road coordinates".into());
        }
        if canonical.get(&feature.id) != Some(feature) {
            return Err("adapter → GeoDB changed road feature".into());
        }
        if !(feature.min_zoom..=feature.max_zoom).contains(&15) {
            return Err("z15 LOD omitted a sampled road".into());
        }
        let Some(point) = sample_world_point(points, region)? else {
            continue;
        };
        let count = 1u32 << 15;
        let x = (point.x * count as f64).floor() as u32;
        let y = (point.y * count as f64).floor() as u32;
        let key = TileKey::new(15, x, y)?;
        if let std::collections::btree_map::Entry::Vacant(entry) = decoded_tiles.entry(key) {
            let bytes = archive
                .tile_bytes(key)
                .await?
                .ok_or("sampled road tile is missing")?;
            entry.insert(decode_mvt(bytes)?);
        }
        let tile = &decoded_tiles[&key];
        let lines = match feature.kind {
            FeatureKind::RoadPrimary => &tile.road_major,
            FeatureKind::RoadSecondary => &tile.road_collector,
            FeatureKind::RoadResidential => &tile.road_local,
            _ => return Err("non-road sampled".into()),
        };
        let local = [
            (point.x * count as f64 - x as f64) * 4096.0,
            (point.y * count as f64 - y as f64) * 4096.0,
        ];
        let distance = nearest_line_distance(local, lines);
        if !distance.is_finite() {
            return Err("sampled road has no decoded same-class line".into());
        }
        let (lon, lat) = unproject(point)?;
        let camera = MapCamera::new(lon, lat, 15.2, 1200, 720, 1.0)?;
        errors_units.push(distance);
        errors_meters
            .push(distance * camera.meters_per_pixel_at_center() * 2.0_f64.powf(0.2) / 8.0);
        errors_screen_px.push(distance * camera.world_size_px() / (count as f64 * 4096.0));
    }
    if errors_units.len() < 50 {
        return Err("fewer than 50 road pipeline samples".into());
    }
    errors_units.sort_by(f64::total_cmp);
    errors_meters.sort_by(f64::total_cmp);
    errors_screen_px.sort_by(f64::total_cmp);
    println!(
        "samples={} source_to_adapter_exact={} adapter_to_geodb_exact={} z15_lod_present={} decoded_nearest_mvt_units_p50={:.3} p95={:.3} max={:.3} decoded_nearest_meters_p50={:.3} p95={:.3} max={:.3} projected_screen_px_p50={:.3} p95={:.3} max={:.3}",
        errors_units.len(),
        errors_units.len(),
        errors_units.len(),
        errors_units.len(),
        percentile(&errors_units, 50),
        percentile(&errors_units, 95),
        errors_units.last().unwrap(),
        percentile(&errors_meters, 50),
        percentile(&errors_meters, 95),
        errors_meters.last().unwrap(),
        percentile(&errors_screen_px, 50),
        percentile(&errors_screen_px, 95),
        errors_screen_px.last().unwrap(),
    );
    if *errors_units.last().unwrap() > 2.0 {
        return Err("sampled road pipeline loss exceeds two MVT units".into());
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), DynError> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: audit_canonical_pipeline MANIFEST GEODB PMTILES".into());
    }
    audit(
        Path::new(&args[1]),
        Path::new(&args[2]),
        Path::new(&args[3]),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn committed_proof_preserves_sampled_road_geometry() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        audit(
            &root.join("data/sources.toml"),
            &root.join("artifacts/map-v0.3c/naju-roads.mgeodb"),
            &root.join("artifacts/map-v0.3c/naju-roads.pmtiles"),
        )
        .await
        .unwrap();
    }
}
