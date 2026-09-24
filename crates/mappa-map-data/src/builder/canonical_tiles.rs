//! Runtime tiles built only from MappaGeoDB, never from a raw source or a base map.

use super::{DynError, add_lines, add_polygons};
use crate::canonical::{BBox, FeatureKind, GeoDb, Geometry as CanonicalGeometry};
use geo::{BoundingRect, Coord, Geometry, LineString, Polygon, Rect};
use mappa_map_core::{TileKey, project};
use mvt::Tile;
use pmtiles::{PmTilesWriter, TileCoord, TileType};
use std::{collections::BTreeMap, fs::File, path::Path};

struct ProjectedFeature {
    kind: FeatureKind,
    geometry: Geometry<f64>,
    min_zoom: u8,
    max_zoom: u8,
}

fn world_rect(bounds: BBox) -> Result<Rect<f64>, DynError> {
    let northwest = project(bounds.west, bounds.north)?;
    let southeast = project(bounds.east, bounds.south)?;
    Ok(Rect::new(
        Coord {
            x: northwest.x,
            y: northwest.y,
        },
        Coord {
            x: southeast.x,
            y: southeast.y,
        },
    ))
}

fn tile_span(rect: Rect<f64>, zoom: u8) -> (u32, u32, u32, u32) {
    let n = (1u32 << zoom) as f64;
    let last = (n as u32) - 1;
    (
        (rect.min().x * n).floor().max(0.0) as u32,
        (rect.max().x * n).floor().min(last as f64) as u32,
        (rect.min().y * n).floor().max(0.0) as u32,
        (rect.max().y * n).floor().min(last as f64) as u32,
    )
}

/// Returns (nonempty tiles, encoded road features). Empty areas stay empty.
pub fn build_canonical_tiles(
    geodb_path: &Path,
    output: &Path,
    region: BBox,
    min_zoom: u8,
    max_zoom: u8,
    attribution: &str,
) -> Result<(usize, usize), DynError> {
    if min_zoom > max_zoom || max_zoom > 15 || min_zoom < 10 {
        return Err("canonical proof zoom must be within 10..=15".into());
    }
    let mut database = GeoDb::open(geodb_path)?;
    let features = database.query(region)?;
    if features.is_empty() {
        return Err("canonical proof region has no features".into());
    }
    let mut projected = Vec::with_capacity(features.len());
    for feature in features {
        let geometry = match feature.geometry {
            CanonicalGeometry::Line(points)
                if matches!(
                    feature.kind,
                    FeatureKind::RoadPrimary
                        | FeatureKind::RoadSecondary
                        | FeatureKind::RoadResidential
                ) =>
            {
                Geometry::LineString(project_ring(points)?)
            }
            CanonicalGeometry::Polygon(rings) if feature.kind == FeatureKind::RoadSurface => {
                let mut projected_rings = rings
                    .into_iter()
                    .map(project_ring)
                    .collect::<Result<Vec<_>, _>>()?;
                let exterior = projected_rings.remove(0);
                Geometry::Polygon(Polygon::new(exterior, projected_rings))
            }
            _ => {
                return Err(
                    "canonical proof tile builder has no renderer for a feature kind".into(),
                );
            }
        };
        projected.push(ProjectedFeature {
            kind: feature.kind,
            geometry,
            min_zoom: feature.min_zoom,
            max_zoom: feature.max_zoom,
        });
    }
    let region_world = world_rect(region)?;
    if attribution.is_empty() || attribution.len() > 512 {
        return Err("invalid canonical attribution".into());
    }
    let metadata = serde_json::json!({
        "attribution": attribution,
        "vector_layers": [
            {"id": "road_surface", "fields": {}, "minzoom": min_zoom, "maxzoom": max_zoom},
            {"id": "road_major", "fields": {}, "minzoom": min_zoom, "maxzoom": max_zoom},
            {"id": "road_collector", "fields": {}, "minzoom": min_zoom, "maxzoom": max_zoom},
            {"id": "road_local", "fields": {}, "minzoom": min_zoom, "maxzoom": max_zoom}
        ]
    })
    .to_string();
    let mut writer = PmTilesWriter::new(TileType::Mvt)
        .min_zoom(min_zoom)
        .max_zoom(max_zoom)
        .bounds(region.west, region.south, region.east, region.north)
        .center(
            (region.west + region.east) / 2.0,
            (region.south + region.north) / 2.0,
        )
        .center_zoom(13)
        .metadata(&metadata)
        .create(File::create(output)?)?;
    let mut tile_total = 0;
    let mut feature_total = 0;
    for zoom in min_zoom..=max_zoom {
        let mut buckets: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
        let (region_x0, region_x1, region_y0, region_y1) = tile_span(region_world, zoom);
        for (index, feature) in projected.iter().enumerate() {
            if !(feature.min_zoom..=feature.max_zoom).contains(&zoom) {
                continue;
            }
            let Some(bounds) = feature.geometry.bounding_rect() else {
                continue;
            };
            let (x0, x1, y0, y1) = tile_span(bounds, zoom);
            for y in y0.max(region_y0)..=y1.min(region_y1) {
                for x in x0.max(region_x0)..=x1.min(region_x1) {
                    buckets.entry((y, x)).or_default().push(index);
                }
            }
        }
        for ((y, x), candidates) in buckets {
            let key = TileKey::new(zoom, x, y)?;
            let n = (1u32 << zoom) as f64;
            let tile_bounds = Rect::new(
                Coord {
                    x: x as f64 / n,
                    y: y as f64 / n,
                },
                Coord {
                    x: (x + 1) as f64 / n,
                    y: (y + 1) as f64 / n,
                },
            );
            let mut tile = Tile::new(4096);
            let mut count = add_polygons(
                &mut tile,
                "road_surface",
                candidates
                    .iter()
                    .filter(|&&index| projected[index].kind == FeatureKind::RoadSurface)
                    .map(|&index| &projected[index].geometry),
                tile_bounds,
                key,
            )?;
            for (kind, layer) in [
                (FeatureKind::RoadPrimary, "road_major"),
                (FeatureKind::RoadSecondary, "road_collector"),
                (FeatureKind::RoadResidential, "road_local"),
            ] {
                count += add_lines(
                    &mut tile,
                    layer,
                    candidates
                        .iter()
                        .filter(|&&index| projected[index].kind == kind)
                        .map(|&index| &projected[index].geometry),
                    tile_bounds,
                    key,
                )?;
            }
            if count > 0 {
                writer.add_tile(TileCoord::new(zoom, x, y)?, &tile.to_bytes()?)?;
                tile_total += 1;
                feature_total += count;
            }
        }
    }
    writer.finalize()?;
    Ok((tile_total, feature_total))
}

fn project_ring(points: Vec<[f64; 2]>) -> Result<LineString<f64>, mappa_map_core::MapError> {
    points
        .into_iter()
        .map(|[lon, lat]| {
            let point = project(lon, lat)?;
            Ok(Coord {
                x: point.x,
                y: point.y,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LocalPmTiles, TileSource, decode_mvt};
    use mappa_map_core::MapCamera;

    #[tokio::test]
    async fn committed_proof_has_only_canonical_road_layers_at_every_zoom() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../artifacts/map-v0.3c/naju-roads.pmtiles");
        let archive = LocalPmTiles::open(path).await.unwrap();
        assert!(archive.attribution.as_deref().unwrap().contains("나주시"));
        for zoom in 10..=15 {
            let camera =
                MapCamera::new(126.715, 35.025, zoom as f64 + 0.2, 1200, 720, 1.0).unwrap();
            let mut roads = 0;
            let mut surfaces = 0;
            for tile in camera.visible_tiles(15, 0) {
                let Some(bytes) = archive.tile_bytes(tile.key).await.unwrap() else {
                    continue;
                };
                let decoded = decode_mvt(bytes).unwrap();
                roads += decoded.road_major.len()
                    + decoded.road_collector.len()
                    + decoded.road_local.len();
                surfaces += decoded.road_surface.len();
                assert!(decoded.land.is_empty());
                assert!(decoded.water.is_empty());
                assert!(decoded.green.is_empty());
                assert!(decoded.place.is_empty());
            }
            assert!(roads > 0, "no decoded roads at zoom {zoom}");
            if zoom >= 13 {
                assert!(surfaces > 0, "no decoded road surfaces at zoom {zoom}");
            }
        }
    }
}
