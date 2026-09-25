//! Runtime tiles built only from MappaGeoDB, never from a raw source or a base map.

use super::{DynError, PlaceSource, add_named_lines_with_buffer, add_places, add_polygons};
use crate::PlaceKind;
use crate::canonical::{BBox, FeatureKind, GeoDb, Geometry as CanonicalGeometry};
use geo::{BoundingRect, Coord, Geometry, LineString, Point, Polygon, Rect};
use mappa_map_core::{TileKey, project};
use mvt::Tile;
use pmtiles::{PmTilesWriter, TileCoord, TileType};
use std::{collections::BTreeMap, fs::File, path::Path};

const ROAD_TILE_BUFFER_UNITS: f64 = 32.0;

struct ProjectedFeature {
    kind: FeatureKind,
    geometry: Geometry<f64>,
    min_zoom: u8,
    max_zoom: u8,
    name: Option<String>,
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
    let upper = |min: f64, max: f64| {
        let value = if min == max {
            (max * n).floor()
        } else {
            (max * n).ceil() - 1.0
        };
        value.clamp(0.0, last as f64) as u32
    };
    (
        (rect.min().x * n).floor().max(0.0) as u32,
        upper(rect.min().x, rect.max().x),
        (rect.min().y * n).floor().max(0.0) as u32,
        upper(rect.min().y, rect.max().y),
    )
}

/// Returns (nonempty tiles, encoded features). Empty areas stay empty.
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
            CanonicalGeometry::Polygon(rings)
                if matches!(
                    feature.kind,
                    FeatureKind::RoadSurface
                        | FeatureKind::Water
                        | FeatureKind::Vegetation
                        | FeatureKind::Park
                        | FeatureKind::Building
                ) =>
            {
                let mut projected_rings = rings
                    .into_iter()
                    .map(project_ring)
                    .collect::<Result<Vec<_>, _>>()?;
                let exterior = projected_rings.remove(0);
                Geometry::Polygon(Polygon::new(exterior, projected_rings))
            }
            CanonicalGeometry::Point([lon, lat])
                if matches!(
                    feature.kind,
                    FeatureKind::PlaceDistrict | FeatureKind::PlaceStation
                ) =>
            {
                let point = project(lon, lat)?;
                Geometry::Point(Point::new(point.x, point.y))
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
            name: feature.name,
        });
    }
    let region_world = world_rect(region)?;
    if attribution.is_empty() || attribution.len() > 512 {
        return Err("invalid canonical attribution".into());
    }
    let green_min_zoom = projected
        .iter()
        .filter(|feature| matches!(feature.kind, FeatureKind::Vegetation | FeatureKind::Park))
        .map(|feature| feature.min_zoom)
        .min()
        .unwrap_or(14);
    let metadata = serde_json::json!({
        "attribution": attribution,
        "vector_layers": [
            {"id": "road_surface", "fields": {}, "minzoom": min_zoom, "maxzoom": max_zoom},
            {"id": "water", "fields": {}, "minzoom": min_zoom, "maxzoom": max_zoom},
            {"id": "green", "fields": {}, "minzoom": green_min_zoom, "maxzoom": max_zoom},
            {"id": "building", "fields": {}, "minzoom": 14, "maxzoom": max_zoom},
            {"id": "road_major", "fields": {"name": "String"}, "minzoom": min_zoom, "maxzoom": max_zoom},
            {"id": "road_collector", "fields": {"name": "String"}, "minzoom": min_zoom, "maxzoom": max_zoom},
            {"id": "road_local", "fields": {"name": "String"}, "minzoom": min_zoom, "maxzoom": max_zoom},
            {"id": "place", "fields": {"name": "String", "rank": "Number", "kind": "String"}, "minzoom": 12, "maxzoom": max_zoom}
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
            let buffer_world = ROAD_TILE_BUFFER_UNITS / (4096.0 * (1u32 << zoom) as f64);
            let bounds = if feature.kind == FeatureKind::RoadSurface {
                bounds
            } else {
                Rect::new(
                    Coord {
                        x: bounds.min().x - buffer_world,
                        y: bounds.min().y - buffer_world,
                    },
                    Coord {
                        x: bounds.max().x + buffer_world,
                        y: bounds.max().y + buffer_world,
                    },
                )
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
                "water",
                candidates
                    .iter()
                    .filter(|&&index| projected[index].kind == FeatureKind::Water)
                    .map(|&index| &projected[index].geometry),
                tile_bounds,
                key,
            )?;
            count += add_polygons(
                &mut tile,
                "green",
                candidates
                    .iter()
                    .filter(|&&index| {
                        matches!(
                            projected[index].kind,
                            FeatureKind::Vegetation | FeatureKind::Park
                        )
                    })
                    .map(|&index| &projected[index].geometry),
                tile_bounds,
                key,
            )?;
            count += add_polygons(
                &mut tile,
                "road_surface",
                candidates
                    .iter()
                    .filter(|&&index| projected[index].kind == FeatureKind::RoadSurface)
                    .map(|&index| &projected[index].geometry),
                tile_bounds,
                key,
            )?;
            count += add_polygons(
                &mut tile,
                "building",
                candidates
                    .iter()
                    .filter(|&&index| projected[index].kind == FeatureKind::Building)
                    .map(|&index| &projected[index].geometry),
                tile_bounds,
                key,
            )?;
            for (kind, layer) in [
                (FeatureKind::RoadPrimary, "road_major"),
                (FeatureKind::RoadSecondary, "road_collector"),
                (FeatureKind::RoadResidential, "road_local"),
            ] {
                count += add_named_lines_with_buffer(
                    &mut tile,
                    layer,
                    candidates
                        .iter()
                        .filter(|&&index| projected[index].kind == kind)
                        .map(|&index| {
                            let feature = &projected[index];
                            (
                                &feature.geometry,
                                if zoom >= 14 {
                                    feature.name.as_deref()
                                } else {
                                    None
                                },
                            )
                        }),
                    tile_bounds,
                    key,
                    ROAD_TILE_BUFFER_UNITS,
                )?;
            }
            let places = candidates
                .iter()
                .filter_map(|&index| {
                    let feature = &projected[index];
                    if !matches!(
                        feature.kind,
                        FeatureKind::PlaceDistrict | FeatureKind::PlaceStation
                    ) {
                        return None;
                    }
                    let Geometry::Point(point) = &feature.geometry else {
                        return None;
                    };
                    let (rank, kind) = if feature.kind == FeatureKind::PlaceStation {
                        (3, PlaceKind::Station)
                    } else {
                        (4, PlaceKind::District)
                    };
                    Some(PlaceSource {
                        point: point.0,
                        name: feature.name.clone()?,
                        rank,
                        kind,
                    })
                })
                .collect::<Vec<_>>();
            if !places.is_empty() {
                count += add_places(&mut tile, places.iter(), tile_bounds, key)?;
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

    #[test]
    fn exact_tile_bounds_do_not_include_neighbor() {
        let n = 1_024.0;
        let rect = Rect::new(
            Coord {
                x: 100.0 / n,
                y: 200.0 / n,
            },
            Coord {
                x: 101.0 / n,
                y: 201.0 / n,
            },
        );
        assert_eq!(tile_span(rect, 10), (100, 100, 200, 200));
        let point = Rect::new(rect.max(), rect.max());
        assert_eq!(tile_span(point, 10), (101, 101, 201, 201));
    }

    #[tokio::test]
    async fn committed_proof_has_approved_roads_water_tree_cover_and_districts() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../artifacts/map-v0.3c/naju-roads.pmtiles");
        let archive = LocalPmTiles::open(path).await.unwrap();
        assert!(archive.attribution.as_deref().unwrap().contains("나주시"));
        assert!(
            archive
                .attribution
                .as_deref()
                .unwrap()
                .contains("국가데이터처")
        );
        assert!(
            archive
                .attribution
                .as_deref()
                .unwrap()
                .contains("ESA WorldCover")
        );
        for zoom in 10..=15 {
            let camera =
                MapCamera::new(126.715, 35.025, zoom as f64 + 0.2, 1200, 720, 1.0).unwrap();
            let mut roads = 0;
            let mut surfaces = 0;
            let mut water = 0;
            let mut vegetation = 0;
            for tile in camera.visible_tiles(15, 0) {
                let Some(bytes) = archive.tile_bytes(tile.key).await.unwrap() else {
                    continue;
                };
                let decoded = decode_mvt(bytes).unwrap();
                roads += decoded.road_major.len()
                    + decoded.road_collector.len()
                    + decoded.road_local.len();
                surfaces += decoded.road_surface.len();
                water += decoded.water.len();
                vegetation += decoded.green.len();
                assert!(decoded.land.is_empty());
                assert!(
                    decoded
                        .place
                        .iter()
                        .all(|place| place.kind == PlaceKind::District)
                );
            }
            assert!(roads > 0, "no decoded roads at zoom {zoom}");
            assert!(water > 0, "no decoded water at zoom {zoom}");
            if zoom >= 14 {
                assert!(vegetation > 0, "no decoded tree cover at zoom {zoom}");
            }
            if zoom >= 13 {
                assert!(surfaces > 0, "no decoded road surfaces at zoom {zoom}");
            }
        }
    }
}
