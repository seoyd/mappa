//! Deterministic, offline Natural Earth fixture builder.

use geo::{
    Area, BooleanOps, BoundingRect, Centroid, Coord, Geometry, LineString, MapCoords, MultiPolygon,
    Polygon, Rect,
};
use geojson::GeoJson;
use mappa_map_core::{TileKey, WorldPoint, project, unproject};
use mvt::{GeomEncoder, GeomType, Tile};
use pmtiles::{PmTilesWriter, TileCoord, TileType};
use rstar::{AABB, RTree, RTreeObject};
use std::{collections::HashMap, fs::File, path::Path, str::FromStr};

use crate::PlaceKind;

type DynError = Box<dyn std::error::Error + Send + Sync>;

pub mod canonical_tiles;
pub mod first_party;

#[cfg(test)]
mod spatial_tests {
    use super::*;

    #[test]
    fn indexed_queries_match_linear_over_world_tiles() {
        let source = std::env::var_os("MAPPA_SPATIAL_TEST_SOURCE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../assets/map/source/50m/ne_50m_admin_0_boundary_lines_land.geojson")
            });
        let geometries = read_geometries(&source).unwrap();
        let indexed = SpatialGeometries::new(geometries);
        for y in 0..32 {
            for x in 0..32 {
                let rect = Rect::new(
                    Coord {
                        x: x as f64 / 32.0,
                        y: y as f64 / 32.0,
                    },
                    Coord {
                        x: (x + 1) as f64 / 32.0,
                        y: (y + 1) as f64 / 32.0,
                    },
                );
                let expected: Vec<_> = indexed
                    .geometries
                    .iter()
                    .filter(|geometry| {
                        geometry
                            .bounding_rect()
                            .is_some_and(|bounds| overlaps(bounds, rect))
                    })
                    .collect();
                let actual = indexed.query(rect);
                assert_eq!(expected.len(), actual.len(), "tile {x},{y}");
                assert!(
                    expected
                        .into_iter()
                        .zip(actual)
                        .all(|(left, right)| std::ptr::eq(left, right))
                );
            }
        }
    }
}

fn read_geometries(path: &Path) -> Result<Vec<Geometry<f64>>, DynError> {
    let source = std::fs::read_to_string(path)?;
    let parsed = GeoJson::from_str(&source)?;
    let mut result = Vec::new();
    if let GeoJson::FeatureCollection(collection) = parsed {
        for feature in collection.features {
            if let Some(geom) = feature.geometry {
                let converted: Geometry<f64> = (&geom.value).try_into()?;
                result.push(converted.map_coords(|c| {
                    // Preserve +180 as x=1 at tile seams instead of wrapping to x=0.
                    let x = ((c.x + 180.0) / 360.0).clamp(0.0, 1.0);
                    let y = project(c.x, c.y)
                        .expect("Natural Earth coordinates are finite")
                        .y;
                    Coord { x, y }
                }));
            }
        }
    }
    Ok(result)
}

fn polygons(geometry: &Geometry<f64>) -> Vec<&Polygon<f64>> {
    match geometry {
        Geometry::Polygon(p) => vec![p],
        Geometry::MultiPolygon(mp) => mp.0.iter().collect(),
        _ => Vec::new(),
    }
}

fn lines(geometry: &Geometry<f64>) -> Vec<&LineString<f64>> {
    match geometry {
        Geometry::LineString(l) => vec![l],
        Geometry::MultiLineString(ml) => ml.0.iter().collect(),
        _ => Vec::new(),
    }
}

fn overlaps(a: Rect<f64>, b: Rect<f64>) -> bool {
    a.min().x <= b.max().x
        && a.max().x >= b.min().x
        && a.min().y <= b.max().y
        && a.max().y >= b.min().y
}

fn ring_area(points: &[(f64, f64)]) -> f64 {
    points
        .windows(2)
        .map(|p| p[0].0 * p[1].1 - p[1].0 * p[0].1)
        .sum::<f64>()
        / 2.0
}

fn add_ring(
    encoder: &mut GeomEncoder<f64>,
    ring: &LineString<f64>,
    key: TileKey,
    exterior: bool,
) -> Result<bool, DynError> {
    let n = (1u32 << key.z) as f64;
    let mut points: Vec<(f64, f64)> = ring
        .0
        .iter()
        .map(|p| {
            (
                ((p.x * n - key.x as f64) * 4096.0)
                    .round()
                    .clamp(0.0, 4096.0),
                ((p.y * n - key.y as f64) * 4096.0)
                    .round()
                    .clamp(0.0, 4096.0),
            )
        })
        .collect();
    points.dedup();
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    if points.len() < 3 {
        return Ok(false);
    }
    let area = {
        let mut closed = points.clone();
        closed.push(points[0]);
        ring_area(&closed)
    };
    if area.abs() < 1.0 {
        return Ok(false);
    }
    if (area > 0.0) != exterior {
        points.reverse();
    }
    for (x, y) in points {
        encoder.add_point(x, y)?;
    }
    encoder.complete_geom()?;
    Ok(true)
}

fn add_polygons<'a>(
    tile: &mut Tile,
    name: &str,
    geoms: impl IntoIterator<Item = &'a Geometry<f64>>,
    rect: Rect<f64>,
    key: TileKey,
) -> Result<usize, DynError> {
    add_polygons_impl(tile, name, geoms, rect, key, false)
}

fn add_preselected_polygons<'a>(
    tile: &mut Tile,
    name: &str,
    geoms: impl IntoIterator<Item = &'a Geometry<f64>>,
    rect: Rect<f64>,
    key: TileKey,
) -> Result<usize, DynError> {
    // Canonical tile buckets were already selected from each geometry's bounds.
    add_polygons_impl(tile, name, geoms, rect, key, true)
}

fn add_polygons_impl<'a>(
    tile: &mut Tile,
    name: &str,
    geoms: impl IntoIterator<Item = &'a Geometry<f64>>,
    rect: Rect<f64>,
    key: TileKey,
    preselected: bool,
) -> Result<usize, DynError> {
    let mut layer = tile.create_layer(name);
    let mut count = 0;
    let clip = rect.to_polygon();
    let min_area_px = if key.z < 8 {
        0.0
    } else {
        match name {
            "land" => match key.z {
                8 => 6.0,
                9 => 2.0,
                10 => 1.0,
                _ => 0.25,
            },
            "water" => match key.z {
                8 => 8.0,
                9 => 4.0,
                10 => 2.0,
                _ => 1.0,
            },
            "green" => match key.z {
                8 => 16.0,
                9 => 8.0,
                10 => 4.0,
                11 => 2.0,
                _ => 1.0,
            },
            _ => 0.0,
        }
    };
    let pixels_per_world = 512.0 * (1u32 << key.z) as f64;
    let min_world_area = min_area_px / pixels_per_world.powi(2);
    for geom in geoms {
        for polygon in polygons(geom) {
            if !preselected && !polygon.bounding_rect().is_some_and(|r| overlaps(r, rect)) {
                continue;
            }
            let clipped: MultiPolygon<f64> = polygon.intersection(&clip);
            for part in clipped.0 {
                if part.unsigned_area() < min_world_area {
                    continue;
                }
                let mut encoder = GeomEncoder::<f64>::new(GeomType::Polygon);
                if !add_ring(&mut encoder, part.exterior(), key, true)? {
                    continue;
                }
                for hole in part.interiors() {
                    add_ring(&mut encoder, hole, key, false)?;
                }
                layer = layer.into_feature(encoder.encode()?).into_layer();
                count += 1;
            }
        }
    }
    tile.add_layer(layer)?;
    Ok(count)
}

fn clip_segment(a: Coord<f64>, b: Coord<f64>, r: Rect<f64>) -> Option<(Coord<f64>, Coord<f64>)> {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let mut t0: f64 = 0.0;
    let mut t1: f64 = 1.0;
    for (p, q) in [
        (-dx, a.x - r.min().x),
        (dx, r.max().x - a.x),
        (-dy, a.y - r.min().y),
        (dy, r.max().y - a.y),
    ] {
        if p.abs() < 1e-15 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
            if t0 > t1 {
                return None;
            }
        }
    }
    Some((
        Coord {
            x: a.x + t0 * dx,
            y: a.y + t0 * dy,
        },
        Coord {
            x: a.x + t1 * dx,
            y: a.y + t1 * dy,
        },
    ))
}

// Encode the exact crossing on the unbuffered tile edge in both adjacent tiles.
// Quantizing only the buffered segment endpoints can shift an interpolated seam
// crossing by multiple MVT units when a long segment is clipped independently.
fn tile_edge_crossings(
    raw_a: Coord<f64>,
    raw_b: Coord<f64>,
    clipped_a: Coord<f64>,
    clipped_b: Coord<f64>,
    tile_rect: Rect<f64>,
) -> Vec<Coord<f64>> {
    let dx = raw_b.x - raw_a.x;
    let dy = raw_b.y - raw_a.y;
    if dx == 0.0 && dy == 0.0 {
        return Vec::new();
    }
    let fraction = |point: Coord<f64>| {
        if dx.abs() >= dy.abs() {
            (point.x - raw_a.x) / dx
        } else {
            (point.y - raw_a.y) / dy
        }
    };
    let start = fraction(clipped_a);
    let end = fraction(clipped_b);
    let mut hits = Vec::with_capacity(4);
    for x in [tile_rect.min().x, tile_rect.max().x] {
        if dx != 0.0 {
            let t = (x - raw_a.x) / dx;
            let y = raw_a.y + t * dy;
            if t > start && t < end && y >= tile_rect.min().y && y <= tile_rect.max().y {
                hits.push((t, Coord { x, y }));
            }
        }
    }
    for y in [tile_rect.min().y, tile_rect.max().y] {
        if dy != 0.0 {
            let t = (y - raw_a.y) / dy;
            let x = raw_a.x + t * dx;
            if t > start && t < end && x >= tile_rect.min().x && x <= tile_rect.max().x {
                hits.push((t, Coord { x, y }));
            }
        }
    }
    hits.sort_by(|a, b| a.0.total_cmp(&b.0));
    hits.into_iter().map(|(_, point)| point).collect()
}

#[cfg(test)]
mod tile_edge_tests {
    use super::*;

    #[test]
    fn adjacent_buffered_tiles_insert_the_same_crossing() {
        let a = Coord { x: 0.4, y: 0.31123 };
        let b = Coord { x: 0.6, y: 0.65987 };
        let left = Rect::new(Coord { x: 0.0, y: 0.0 }, Coord { x: 0.5, y: 1.0 });
        let right = Rect::new(Coord { x: 0.5, y: 0.0 }, Coord { x: 1.0, y: 1.0 });
        let left_buffer = Rect::new(Coord { x: -0.05, y: -0.05 }, Coord { x: 0.55, y: 1.05 });
        let right_buffer = Rect::new(Coord { x: 0.45, y: -0.05 }, Coord { x: 1.05, y: 1.05 });
        let (left_a, left_b) = clip_segment(a, b, left_buffer).unwrap();
        let (right_a, right_b) = clip_segment(a, b, right_buffer).unwrap();
        let left_hits = tile_edge_crossings(a, b, left_a, left_b, left);
        let right_hits = tile_edge_crossings(a, b, right_a, right_b, right);
        assert_eq!(left_hits, right_hits);
        assert_eq!(left_hits.len(), 1);
        assert_eq!(left_hits[0].x, 0.5);
    }
}

fn add_lines<'a>(
    tile: &mut Tile,
    name: &str,
    geoms: impl IntoIterator<Item = &'a Geometry<f64>>,
    rect: Rect<f64>,
    key: TileKey,
) -> Result<usize, DynError> {
    add_lines_with_buffer(tile, name, geoms, rect, key, 0.0)
}

fn add_lines_with_buffer<'a>(
    tile: &mut Tile,
    name: &str,
    geoms: impl IntoIterator<Item = &'a Geometry<f64>>,
    rect: Rect<f64>,
    key: TileKey,
    buffer_units: f64,
) -> Result<usize, DynError> {
    add_named_lines_with_buffer(
        tile,
        name,
        geoms.into_iter().map(|geom| (geom, None)),
        rect,
        key,
        buffer_units,
    )
}

fn add_named_lines_with_buffer<'a>(
    tile: &mut Tile,
    name: &str,
    geoms: impl IntoIterator<Item = (&'a Geometry<f64>, Option<&'a str>)>,
    rect: Rect<f64>,
    key: TileKey,
    buffer_units: f64,
) -> Result<usize, DynError> {
    let mut layer = tile.create_layer(name);
    let mut count = 0;
    let n = (1u32 << key.z) as f64;
    let buffer_world = buffer_units / (4096.0 * n);
    let tile_rect = rect;
    let rect = Rect::new(
        Coord {
            x: rect.min().x - buffer_world,
            y: rect.min().y - buffer_world,
        },
        Coord {
            x: rect.max().x + buffer_world,
            y: rect.max().y + buffer_world,
        },
    );
    for (geom, road_name) in geoms {
        for line in lines(geom) {
            if !line.bounding_rect().is_some_and(|r| overlaps(r, rect)) {
                continue;
            }
            let mut encoder = GeomEncoder::<f64>::new(GeomType::Linestring);
            let mut previous_end: Option<(f64, f64)> = None;
            let mut has_geometry = false;
            for pair in line.0.windows(2) {
                let Some((a, b)) = clip_segment(pair[0], pair[1], rect) else {
                    continue;
                };
                let quantize = |point: Coord<f64>| {
                    (
                        ((point.x * n - key.x as f64) * 4096.0).round(),
                        ((point.y * n - key.y as f64) * 4096.0).round(),
                    )
                };
                let mut points = vec![quantize(a)];
                if buffer_units > 0.0 {
                    points.extend(
                        tile_edge_crossings(pair[0], pair[1], a, b, tile_rect)
                            .into_iter()
                            .map(quantize),
                    );
                }
                points.push(quantize(b));
                points.dedup();
                if points.len() < 2 {
                    continue;
                }
                let start = points[0];
                if previous_end != Some(start) {
                    if previous_end.is_some() {
                        encoder.complete_geom()?;
                    }
                    encoder.add_point(start.0, start.1)?;
                }
                for point in points.into_iter().skip(1) {
                    encoder.add_point(point.0, point.1)?;
                    previous_end = Some(point);
                }
                has_geometry = true;
            }
            if has_geometry {
                encoder.complete_geom()?;
                let mut feature = layer.into_feature(encoder.encode()?);
                if let Some(road_name) =
                    road_name.filter(|name| !name.is_empty() && name.len() <= 128)
                {
                    feature.add_tag_string("name", road_name);
                }
                layer = feature.into_layer();
                count += 1;
            }
        }
    }
    tile.add_layer(layer)?;
    Ok(count)
}

pub fn build_fixture(
    source_dir: &Path,
    output: &Path,
    max_zoom: u8,
) -> Result<(usize, usize), DynError> {
    if max_zoom > 5 {
        return Err("fixture max zoom must be <= 5".into());
    }
    let land = read_geometries(&source_dir.join("ne_110m_land.geojson"))?;
    let water = read_geometries(&source_dir.join("ne_110m_lakes.geojson"))?;
    let boundary =
        read_geometries(&source_dir.join("ne_110m_admin_0_boundary_lines_land.geojson"))?;
    let file = File::create(output)?;
    let metadata = r#"{"vector_layers":[{"id":"land","fields":{},"minzoom":0,"maxzoom":4},{"id":"water","fields":{},"minzoom":0,"maxzoom":4},{"id":"boundary","fields":{},"minzoom":0,"maxzoom":4}]}"#;
    let mut writer = PmTilesWriter::new(TileType::Mvt)
        .min_zoom(0)
        .max_zoom(max_zoom)
        .metadata(metadata)
        .create(file)?;
    let mut tiles = 0;
    let mut features = 0;
    for z in 0..=max_zoom {
        let n = 1u32 << z;
        for y in 0..n {
            for x in 0..n {
                let key = TileKey::new(z, x, y)?;
                let rect = Rect::new(
                    Coord {
                        x: x as f64 / n as f64,
                        y: y as f64 / n as f64,
                    },
                    Coord {
                        x: (x + 1) as f64 / n as f64,
                        y: (y + 1) as f64 / n as f64,
                    },
                );
                let mut tile = Tile::new(4096);
                let count = add_polygons(&mut tile, "land", &land, rect, key)?
                    + add_polygons(&mut tile, "water", &water, rect, key)?
                    + add_lines(&mut tile, "boundary", &boundary, rect, key)?;
                if count > 0 {
                    writer.add_tile(TileCoord::new(z, x, y)?, &tile.to_bytes()?)?;
                    tiles += 1;
                    features += count;
                }
            }
        }
    }
    writer.finalize()?;
    Ok((tiles, features))
}

struct PlaceSource {
    point: Coord<f64>,
    name: String,
    rank: u8,
    kind: PlaceKind,
}

#[derive(Clone, Copy)]
struct SpatialEntry {
    index: usize,
    envelope: AABB<[f64; 2]>,
}

impl RTreeObject for SpatialEntry {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.envelope
    }
}

struct SpatialGeometries {
    geometries: Vec<Geometry<f64>>,
    index: RTree<SpatialEntry>,
}

struct SpatialPlaces {
    places: Vec<PlaceSource>,
    index: RTree<SpatialEntry>,
}

impl SpatialPlaces {
    fn new(places: Vec<PlaceSource>) -> Self {
        let entries = places
            .iter()
            .enumerate()
            .map(|(index, place)| SpatialEntry {
                index,
                envelope: AABB::from_point([place.point.x, place.point.y]),
            })
            .collect();
        Self {
            places,
            index: RTree::bulk_load(entries),
        }
    }

    fn query(&self, bounds: Rect<f64>) -> Vec<&PlaceSource> {
        let envelope = AABB::from_corners(
            [bounds.min().x, bounds.min().y],
            [bounds.max().x, bounds.max().y],
        );
        let mut entries: Vec<_> = self
            .index
            .locate_in_envelope_intersecting(&envelope)
            .collect();
        entries.sort_unstable_by_key(|entry| entry.index);
        entries
            .into_iter()
            .map(|entry| &self.places[entry.index])
            .collect()
    }
}

impl SpatialGeometries {
    fn new(geometries: Vec<Geometry<f64>>) -> Self {
        let entries = geometries
            .iter()
            .enumerate()
            .filter_map(|(index, geometry)| {
                let bounds = geometry.bounding_rect()?;
                Some(SpatialEntry {
                    index,
                    envelope: AABB::from_corners(
                        [bounds.min().x, bounds.min().y],
                        [bounds.max().x, bounds.max().y],
                    ),
                })
            })
            .collect();
        Self {
            geometries,
            index: RTree::bulk_load(entries),
        }
    }

    fn query(&self, bounds: Rect<f64>) -> Vec<&Geometry<f64>> {
        let envelope = AABB::from_corners(
            [bounds.min().x, bounds.min().y],
            [bounds.max().x, bounds.max().y],
        );
        let mut entries: Vec<_> = self
            .index
            .locate_in_envelope_intersecting(&envelope)
            .collect();
        entries.sort_unstable_by_key(|entry| entry.index);
        entries
            .into_iter()
            .map(|entry| &self.geometries[entry.index])
            .collect()
    }
}

fn clip_to_region(geoms: Vec<Geometry<f64>>, region: Rect<f64>) -> Vec<Geometry<f64>> {
    let clip = region.to_polygon();
    let mut out = Vec::new();
    for geom in &geoms {
        for polygon in polygons(geom) {
            if polygon.bounding_rect().is_some_and(|r| overlaps(r, region)) {
                let clipped = polygon.intersection(&clip);
                if !clipped.0.is_empty() {
                    out.push(Geometry::MultiPolygon(clipped));
                }
            }
        }
    }
    out
}

fn read_regional_lines(
    path: &Path,
    region_lonlat: Rect<f64>,
    max_rank: Option<u8>,
) -> Result<Vec<Geometry<f64>>, DynError> {
    let source = std::fs::read_to_string(path)?;
    let parsed = GeoJson::from_str(&source)?;
    let mut result = Vec::new();
    if let GeoJson::FeatureCollection(collection) = parsed {
        for feature in collection.features {
            if let Some(limit) = max_rank {
                let rank = feature
                    .property("scalerank")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(f64::INFINITY);
                if rank > f64::from(limit) {
                    continue;
                }
            }
            let Some(geom) = feature.geometry else {
                continue;
            };
            let converted: Geometry<f64> = (&geom.value).try_into()?;
            if !converted
                .bounding_rect()
                .is_some_and(|bounds| overlaps(bounds, region_lonlat))
            {
                continue;
            }
            result.push(converted.map_coords(|c| {
                let projected = project(c.x, c.y).expect("Natural Earth coordinates are finite");
                Coord {
                    x: ((c.x + 180.0) / 360.0).clamp(0.0, 1.0),
                    y: projected.y,
                }
            }));
        }
    }
    Ok(result)
}

fn read_regional_places(
    path: &Path,
    region_lonlat: Rect<f64>,
    max_rank: u8,
) -> Result<Vec<PlaceSource>, DynError> {
    let source = std::fs::read_to_string(path)?;
    let parsed = GeoJson::from_str(&source)?;
    let mut result = Vec::new();
    if let GeoJson::FeatureCollection(collection) = parsed {
        for feature in collection.features {
            let rank = feature
                .property("SCALERANK")
                .and_then(|value| value.as_u64())
                .unwrap_or(u64::MAX);
            if rank > max_rank as u64 {
                continue;
            }
            let Some(name) = feature
                .property("NAME")
                .and_then(|value| value.as_str())
                .filter(|name| !name.is_empty() && name.len() <= 128)
            else {
                continue;
            };
            let Some(geom) = &feature.geometry else {
                continue;
            };
            let converted: Geometry<f64> = (&geom.value).try_into()?;
            let Geometry::Point(point) = converted else {
                continue;
            };
            let lon = point.x();
            let lat = point.y();
            if lon < region_lonlat.min().x
                || lon >= region_lonlat.max().x
                || lat < region_lonlat.min().y
                || lat >= region_lonlat.max().y
            {
                continue;
            }
            let projected = project(lon, lat)?;
            result.push(PlaceSource {
                point: Coord {
                    x: projected.x,
                    y: projected.y,
                },
                name: name.to_owned(),
                rank: rank as u8,
                kind: PlaceKind::City,
            });
        }
    }
    Ok(result)
}

fn add_places<'a>(
    tile: &mut Tile,
    places: impl IntoIterator<Item = &'a PlaceSource>,
    rect: Rect<f64>,
    key: TileKey,
) -> Result<usize, DynError> {
    let mut layer = tile.create_layer("place");
    let n = (1u32 << key.z) as f64;
    let mut count = 0;
    for place in places {
        let p = place.point;
        if p.x < rect.min().x || p.x >= rect.max().x || p.y < rect.min().y || p.y >= rect.max().y {
            continue;
        }
        let x = ((p.x * n - key.x as f64) * 4096.0).round();
        let y = ((p.y * n - key.y as f64) * 4096.0).round();
        let geometry = GeomEncoder::<f64>::new(GeomType::Point)
            .point(x, y)?
            .encode()?;
        let mut feature = layer.into_feature(geometry);
        feature.add_tag_string("name", &place.name);
        feature.add_tag_uint("rank", place.rank as u64);
        let kind = match place.kind {
            PlaceKind::City => "city",
            PlaceKind::Station => "station",
            PlaceKind::Civic => "civic",
            PlaceKind::District => "district",
        };
        feature.add_tag_string("kind", kind);
        layer = feature.into_layer();
        count += 1;
    }
    tile.add_layer(layer)?;
    Ok(count)
}

/// Build a bounded 1:10m offline detail archive. Tile bounds are [x0, y0, x1, y1) at min_zoom.
pub fn build_detail_fixture(
    source_dir: &Path,
    output: &Path,
    min_zoom: u8,
    max_zoom: u8,
    tile_bounds: [u32; 4],
    max_road_rank: u8,
    max_place_rank: u8,
) -> Result<(usize, usize), DynError> {
    build_detail_fixture_inner(
        source_dir,
        output,
        min_zoom,
        max_zoom,
        tile_bounds,
        [max_road_rank, max_place_rank],
        None,
    )
}

/// Build the same overview with ranked Natural Earth river centerlines.
pub fn build_world_detail_with_rivers(
    source_dir: &Path,
    output: &Path,
    min_zoom: u8,
    max_zoom: u8,
    max_road_rank: u8,
    max_place_rank: u8,
    river_ranks: [u8; 2],
) -> Result<(usize, usize), DynError> {
    if !(5..=8).contains(&min_zoom) {
        return Err("world detail min zoom must be 5..=8".into());
    }
    let n = 1u32 << min_zoom;
    build_detail_fixture_inner(
        source_dir,
        output,
        min_zoom,
        max_zoom,
        [0, 0, n, n],
        [max_road_rank, max_place_rank],
        Some(river_ranks),
    )
}

fn build_detail_fixture_inner(
    source_dir: &Path,
    output: &Path,
    min_zoom: u8,
    max_zoom: u8,
    tile_bounds: [u32; 4],
    detail_ranks: [u8; 2],
    river_ranks: Option<[u8; 2]>,
) -> Result<(usize, usize), DynError> {
    let [max_road_rank, max_place_rank] = detail_ranks;
    if min_zoom < 5 || max_zoom < min_zoom || max_zoom > 8 {
        return Err("detail zooms must be within 5..=8".into());
    }
    let n = 1u32 << min_zoom;
    let [x0, y0, x1, y1] = tile_bounds;
    if x0 >= x1 || y0 >= y1 || x1 > n || y1 > n {
        return Err("invalid detail tile bounds".into());
    }
    let region = Rect::new(
        Coord {
            x: x0 as f64 / n as f64,
            y: y0 as f64 / n as f64,
        },
        Coord {
            x: x1 as f64 / n as f64,
            y: y1 as f64 / n as f64,
        },
    );
    let min_lon = x0 as f64 / n as f64 * 360.0 - 180.0;
    let max_lon = x1 as f64 / n as f64 * 360.0 - 180.0;
    let max_lat = unproject(WorldPoint {
        x: 0.0,
        y: region.min().y,
    })?
    .1;
    let min_lat = unproject(WorldPoint {
        x: 0.0,
        y: region.max().y,
    })?
    .1;
    let region_lonlat = Rect::new(
        Coord {
            x: min_lon,
            y: min_lat,
        },
        Coord {
            x: max_lon,
            y: max_lat,
        },
    );
    let land = SpatialGeometries::new(clip_to_region(
        read_geometries(&source_dir.join("ne_10m_land.geojson"))?,
        region,
    ));
    let water = SpatialGeometries::new(clip_to_region(
        read_geometries(&source_dir.join("ne_10m_lakes.geojson"))?,
        region,
    ));
    let boundary = SpatialGeometries::new(read_regional_lines(
        &source_dir.join("ne_10m_admin_0_boundary_lines_land.geojson"),
        region_lonlat,
        None,
    )?);
    let road = SpatialGeometries::new(read_regional_lines(
        &source_dir.join("ne_10m_roads.geojson"),
        region_lonlat,
        Some(max_road_rank),
    )?);
    let rivers = if let Some([mid_rank, detail_rank]) = river_ranks {
        if mid_rank > detail_rank {
            return Err("river detail rank must be >= mid rank".into());
        }
        let path = source_dir.join("ne_10m_rivers_lake_centerlines.geojson");
        Some([
            SpatialGeometries::new(read_regional_lines(&path, region_lonlat, Some(mid_rank))?),
            SpatialGeometries::new(read_regional_lines(
                &path,
                region_lonlat,
                Some(detail_rank),
            )?),
        ])
    } else {
        None
    };
    let places = SpatialPlaces::new(read_regional_places(
        &source_dir.join("ne_10m_populated_places.geojson"),
        region_lonlat,
        max_place_rank,
    )?);
    let mut layers = vec![
        serde_json::json!({"id":"land","fields":{},"minzoom":min_zoom,"maxzoom":max_zoom}),
        serde_json::json!({"id":"water","fields":{},"minzoom":min_zoom,"maxzoom":max_zoom}),
        serde_json::json!({"id":"boundary","fields":{},"minzoom":min_zoom,"maxzoom":max_zoom}),
        serde_json::json!({"id":"road","fields":{},"minzoom":min_zoom+1,"maxzoom":max_zoom}),
        serde_json::json!({"id":"place","fields":{"name":"String","rank":"Number"},"minzoom":min_zoom+1,"maxzoom":max_zoom}),
    ];
    if rivers.is_some() {
        layers.push(serde_json::json!({"id":"waterway","fields":{},"minzoom":min_zoom+1,"maxzoom":max_zoom}));
    }
    let metadata = serde_json::json!({"vector_layers":layers}).to_string();
    let file = File::create(output)?;
    let mut writer = PmTilesWriter::new(TileType::Mvt)
        .min_zoom(min_zoom)
        .max_zoom(max_zoom)
        .bounds(min_lon, min_lat, max_lon, max_lat)
        .center((min_lon + max_lon) / 2.0, (min_lat + max_lat) / 2.0)
        .center_zoom(min_zoom)
        .metadata(&metadata)
        .create(file)?;
    let mut tiles = 0;
    let mut features = 0;
    for z in min_zoom..=max_zoom {
        let shift = z - min_zoom;
        let count = 1u32 << z;
        for y in (y0 << shift)..(y1 << shift) {
            for x in (x0 << shift)..(x1 << shift) {
                let key = TileKey::new(z, x, y)?;
                let rect = Rect::new(
                    Coord {
                        x: x as f64 / count as f64,
                        y: y as f64 / count as f64,
                    },
                    Coord {
                        x: (x + 1) as f64 / count as f64,
                        y: (y + 1) as f64 / count as f64,
                    },
                );
                let mut tile = Tile::new(4096);
                let mut count = add_polygons(&mut tile, "land", land.query(rect), rect, key)?
                    + add_polygons(&mut tile, "water", water.query(rect), rect, key)?
                    + add_lines(&mut tile, "boundary", boundary.query(rect), rect, key)?;
                if z > min_zoom {
                    if let Some(rivers) = &rivers {
                        let selected = if z == min_zoom + 1 {
                            &rivers[0]
                        } else {
                            &rivers[1]
                        };
                        count += add_lines(&mut tile, "waterway", selected.query(rect), rect, key)?;
                    }
                    count += add_lines(&mut tile, "road", road.query(rect), rect, key)?;
                    count += add_places(&mut tile, places.query(rect), rect, key)?;
                }
                if count > 0 {
                    writer.add_tile(TileCoord::new(z, x, y)?, &tile.to_bytes()?)?;
                    tiles += 1;
                    features += count;
                }
            }
        }
    }
    writer.finalize()?;
    Ok((tiles, features))
}

/// Build an offline global mid-zoom archive from pinned Natural Earth 1:50m inputs.
/// The input directory must contain land, lakes, land boundaries, and populated places.
pub fn build_world_50m(
    source_dir: &Path,
    output: &Path,
    max_zoom: u8,
) -> Result<(usize, usize), DynError> {
    if !(5..=7).contains(&max_zoom) {
        return Err("world 50m max zoom must be 5..=7".into());
    }
    let land = read_geometries(&source_dir.join("ne_50m_land.geojson"))?;
    let water = read_geometries(&source_dir.join("ne_50m_lakes.geojson"))?;
    let boundary = read_geometries(&source_dir.join("ne_50m_admin_0_boundary_lines_land.geojson"))?;
    let places = read_regional_places(
        &source_dir.join("ne_50m_populated_places.geojson"),
        Rect::new(
            Coord {
                x: -180.0,
                y: -85.0,
            },
            Coord { x: 180.0, y: 85.0 },
        ),
        5,
    )?;
    let metadata = format!(
        "{{\"name\":\"Mappa world 50m\",\"source\":\"Natural Earth public domain\",\"vector_layers\":[{{\"id\":\"land\",\"fields\":{{}},\"minzoom\":5,\"maxzoom\":{max_zoom}}},{{\"id\":\"water\",\"fields\":{{}},\"minzoom\":5,\"maxzoom\":{max_zoom}}},{{\"id\":\"boundary\",\"fields\":{{}},\"minzoom\":5,\"maxzoom\":{max_zoom}}},{{\"id\":\"place\",\"fields\":{{\"name\":\"String\",\"rank\":\"Number\"}},\"minzoom\":6,\"maxzoom\":{max_zoom}}}]}}"
    );
    let file = File::create(output)?;
    let mut writer = PmTilesWriter::new(TileType::Mvt)
        .min_zoom(5)
        .max_zoom(max_zoom)
        .bounds(-180.0, -85.0, 180.0, 85.0)
        .center(0.0, 0.0)
        .center_zoom(5)
        .metadata(&metadata)
        .create(file)?;
    let mut tiles = 0;
    let mut features = 0;
    for z in 5..=max_zoom {
        let n = 1u32 << z;
        for y in 0..n {
            for x in 0..n {
                let key = TileKey::new(z, x, y)?;
                let rect = Rect::new(
                    Coord {
                        x: x as f64 / n as f64,
                        y: y as f64 / n as f64,
                    },
                    Coord {
                        x: (x + 1) as f64 / n as f64,
                        y: (y + 1) as f64 / n as f64,
                    },
                );
                let mut tile = Tile::new(4096);
                let mut count = add_polygons(&mut tile, "land", &land, rect, key)?
                    + add_polygons(&mut tile, "water", &water, rect, key)?
                    + add_lines(&mut tile, "boundary", &boundary, rect, key)?;
                if z >= 6 {
                    count += add_places(&mut tile, &places, rect, key)?;
                }
                if count > 0 {
                    writer.add_tile(TileCoord::new(z, x, y)?, &tile.to_bytes()?)?;
                    tiles += 1;
                    features += count;
                }
            }
        }
    }
    writer.finalize()?;
    Ok((tiles, features))
}

struct RoadSource {
    geometry: Geometry<f64>,
    class: u8,
    link: bool,
}

fn read_osm_roads(path: &Path, region: Rect<f64>) -> Result<Vec<RoadSource>, DynError> {
    let source = std::fs::read_to_string(path)?;
    let parsed = GeoJson::from_str(&source)?;
    let GeoJson::FeatureCollection(collection) = parsed else {
        return Err("OSM roads must be a GeoJSON feature collection".into());
    };
    let mut result = Vec::new();
    for feature in collection.features {
        let highway = feature.property("highway").and_then(|value| value.as_str());
        let class = match highway {
            Some("motorway" | "motorway_link" | "trunk" | "trunk_link") => 0,
            Some("primary" | "primary_link" | "secondary" | "secondary_link") => 1,
            Some("tertiary" | "tertiary_link") => 2,
            Some("residential" | "unclassified" | "living_street" | "pedestrian") => 3,
            _ => continue,
        };
        let link = highway.is_some_and(|name| name.ends_with("_link"));
        let Some(geometry) = feature.geometry else {
            continue;
        };
        let converted: Geometry<f64> = (&geometry.value).try_into()?;
        if !converted
            .bounding_rect()
            .is_some_and(|bounds| overlaps(bounds, region))
        {
            continue;
        }
        let projected = converted.map_coords(|coordinate| {
            let world = project(coordinate.x, coordinate.y).expect("valid OSM coordinates");
            Coord {
                x: world.x,
                y: world.y,
            }
        });
        result.push(RoadSource {
            geometry: projected,
            class,
            link,
        });
    }
    Ok(result)
}

fn read_osm_places(path: &Path, region: Rect<f64>) -> Result<Vec<PlaceSource>, DynError> {
    let source = std::fs::read_to_string(path)?;
    let parsed = GeoJson::from_str(&source)?;
    let GeoJson::FeatureCollection(collection) = parsed else {
        return Err("OSM places must be a GeoJSON feature collection".into());
    };
    let mut result = Vec::new();
    for feature in collection.features {
        let railway = feature.property("railway").and_then(|value| value.as_str());
        let amenity = feature.property("amenity").and_then(|value| value.as_str());
        let office = feature.property("office").and_then(|value| value.as_str());
        let place = feature.property("place").and_then(|value| value.as_str());
        let (kind, rank) = if matches!(place, Some("city" | "town")) {
            (PlaceKind::City, if place == Some("city") { 0 } else { 1 })
        } else if matches!(railway, Some("station" | "halt")) {
            (PlaceKind::Station, 3)
        } else if matches!(
            amenity,
            Some("townhall" | "police" | "fire_station" | "post_office" | "courthouse")
        ) || office == Some("government")
        {
            (PlaceKind::Civic, 5)
        } else {
            continue;
        };
        let Some(name) = feature
            .property("name_ko")
            .and_then(|value| value.as_str())
            .filter(|name| !name.is_empty())
            .or_else(|| feature.property("name").and_then(|value| value.as_str()))
            .filter(|name| !name.is_empty() && name.len() <= 128)
        else {
            continue;
        };
        let name = name.to_owned();
        let Some(geometry) = feature.geometry else {
            continue;
        };
        let converted: Geometry<f64> = (&geometry.value).try_into()?;
        let Some(point) = converted.centroid() else {
            continue;
        };
        if point.x() < region.min().x
            || point.x() >= region.max().x
            || point.y() < region.min().y
            || point.y() >= region.max().y
        {
            continue;
        }
        let world = project(point.x(), point.y())?;
        result.push(PlaceSource {
            point: Coord {
                x: world.x,
                y: world.y,
            },
            name,
            rank,
            kind,
        });
    }
    Ok(result)
}

fn road_buckets(
    roads: &[RoadSource],
    zoom: u8,
    tile_bounds: [u32; 4],
) -> HashMap<(u32, u32), Vec<usize>> {
    let [x0, y0, x1, y1] = tile_bounds;
    let n = (1u32 << zoom) as f64;
    let max_class = match zoom {
        8 => 0,
        9 | 10 => 1,
        11 => 2,
        _ => 3,
    };
    let mut buckets: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for (index, road) in roads.iter().enumerate() {
        if road.class > max_class || (zoom <= 9 && road.link) {
            continue;
        }
        let Some(bounds) = road.geometry.bounding_rect() else {
            continue;
        };
        let left = (bounds.min().x * n).floor().max(x0 as f64) as u32;
        let right = (bounds.max().x * n).floor().min((x1 - 1) as f64) as u32;
        let top = (bounds.min().y * n).floor().max(y0 as f64) as u32;
        let bottom = (bounds.max().y * n).floor().min((y1 - 1) as f64) as u32;
        if left > right || top > bottom {
            continue;
        }
        for y in top..=bottom {
            for x in left..=right {
                buckets.entry((x, y)).or_default().push(index);
            }
        }
    }
    buckets
}

pub struct OsmSources<'a> {
    pub natural_earth_dir: &'a Path,
    pub land: &'a Path,
    pub water: &'a Path,
    pub green: &'a Path,
    pub roads: &'a Path,
    pub points: &'a Path,
    pub areas: &'a Path,
}

/// Build a bounded offline OSM detail archive from static extracted GeoJSON.
/// Tile bounds are [x0, y0, x1, y1) at min_zoom.
pub fn build_osm_fixture(
    sources: OsmSources<'_>,
    output: &Path,
    min_zoom: u8,
    max_zoom: u8,
    tile_bounds: [u32; 4],
) -> Result<(usize, usize), DynError> {
    if min_zoom < 8 || max_zoom > 12 || min_zoom > max_zoom {
        return Err("OSM zooms must be within 8..=12".into());
    }
    let [x0, y0, x1, y1] = tile_bounds;
    if x0 >= x1 || y0 >= y1 || x1 > 1 << min_zoom || y1 > 1 << min_zoom {
        return Err("invalid OSM tile bounds".into());
    }
    let n = (1u32 << min_zoom) as f64;
    let region = Rect::new(
        Coord {
            x: x0 as f64 / n,
            y: y0 as f64 / n,
        },
        Coord {
            x: x1 as f64 / n,
            y: y1 as f64 / n,
        },
    );
    let min_lon = x0 as f64 / n * 360.0 - 180.0;
    let max_lon = x1 as f64 / n * 360.0 - 180.0;
    let max_lat = unproject(WorldPoint {
        x: 0.0,
        y: region.min().y,
    })?
    .1;
    let min_lat = unproject(WorldPoint {
        x: 0.0,
        y: region.max().y,
    })?
    .1;
    let geographic_region = Rect::new(
        Coord {
            x: min_lon,
            y: min_lat,
        },
        Coord {
            x: max_lon,
            y: max_lat,
        },
    );
    let land = clip_to_region(read_geometries(sources.land)?, region);
    let water = clip_to_region(read_geometries(sources.water)?, region);
    let green = clip_to_region(read_geometries(sources.green)?, region);
    let boundary = read_regional_lines(
        &sources
            .natural_earth_dir
            .join("ne_10m_admin_0_boundary_lines_land.geojson"),
        geographic_region,
        None,
    )?;
    let roads = read_osm_roads(sources.roads, geographic_region)?;
    let mut places = read_osm_places(sources.points, geographic_region)?;
    places.extend(read_osm_places(sources.areas, geographic_region)?);
    let metadata = format!(
        "{{\"vector_layers\":[{{\"id\":\"land\",\"fields\":{{}},\"minzoom\":{min_zoom},\"maxzoom\":{max_zoom}}},{{\"id\":\"water\",\"fields\":{{}},\"minzoom\":{min_zoom},\"maxzoom\":{max_zoom}}},{{\"id\":\"green\",\"fields\":{{}},\"minzoom\":{min_zoom},\"maxzoom\":{max_zoom}}},{{\"id\":\"boundary\",\"fields\":{{}},\"minzoom\":{min_zoom},\"maxzoom\":{max_zoom}}},{{\"id\":\"road_major\",\"fields\":{{}},\"minzoom\":{min_zoom},\"maxzoom\":{max_zoom}}},{{\"id\":\"road_collector\",\"fields\":{{}},\"minzoom\":{min_zoom},\"maxzoom\":{max_zoom}}},{{\"id\":\"road_local\",\"fields\":{{}},\"minzoom\":{min_zoom},\"maxzoom\":{max_zoom}}},{{\"id\":\"place\",\"fields\":{{\"name\":\"String\",\"rank\":\"Number\",\"kind\":\"String\"}},\"minzoom\":{min_zoom},\"maxzoom\":{max_zoom}}}]}}"
    );
    let file = File::create(output)?;
    let mut writer = PmTilesWriter::new(TileType::Mvt)
        .min_zoom(min_zoom)
        .max_zoom(max_zoom)
        .bounds(min_lon, min_lat, max_lon, max_lat)
        .center((min_lon + max_lon) / 2.0, (min_lat + max_lat) / 2.0)
        .center_zoom(min_zoom)
        .metadata(&metadata)
        .create(file)?;
    let mut tiles = 0;
    let mut features = 0;
    for zoom in min_zoom..=max_zoom {
        let shift = zoom - min_zoom;
        let bounds = [x0 << shift, y0 << shift, x1 << shift, y1 << shift];
        let candidates = road_buckets(&roads, zoom, bounds);
        let n = (1u32 << zoom) as f64;
        for y in bounds[1]..bounds[3] {
            for x in bounds[0]..bounds[2] {
                let key = TileKey::new(zoom, x, y)?;
                let rect = Rect::new(
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
                let mut count = add_polygons(&mut tile, "land", &land, rect, key)?
                    + add_polygons(&mut tile, "green", &green, rect, key)?
                    + add_polygons(&mut tile, "water", &water, rect, key)?
                    + add_lines(&mut tile, "boundary", &boundary, rect, key)?;
                for (name, classes) in [
                    ("road_major", 0..=1),
                    ("road_collector", 2..=2),
                    ("road_local", 3..=3),
                ] {
                    count += add_lines(
                        &mut tile,
                        name,
                        candidates
                            .get(&(x, y))
                            .into_iter()
                            .flatten()
                            .filter(|index| classes.contains(&roads[**index].class))
                            .map(|index| &roads[*index].geometry),
                        rect,
                        key,
                    )?;
                }
                count += add_places(
                    &mut tile,
                    places.iter().filter(|place| match place.kind {
                        PlaceKind::City => zoom >= 9 || place.rank == 0,
                        PlaceKind::Station => zoom >= 11,
                        PlaceKind::Civic => zoom >= 12,
                        PlaceKind::District => zoom >= 12,
                    }),
                    rect,
                    key,
                )?;
                if count > 0 {
                    writer.add_tile(TileCoord::new(zoom, x, y)?, &tile.to_bytes()?)?;
                    tiles += 1;
                    features += count;
                }
            }
        }
    }
    writer.finalize()?;
    Ok((tiles, features))
}

/// Build an offline road pilot from non-OSM WGS84 GeoJSON and Natural Earth land.
/// The covered tile rectangle is derived from the source road geometry.
pub fn build_public_roads_fixture(
    land_path: &Path,
    roads_path: &Path,
    surface_path: &Path,
    output: &Path,
    min_zoom: u8,
    max_zoom: u8,
) -> Result<(usize, usize), DynError> {
    if min_zoom > max_zoom || max_zoom > 12 {
        return Err("invalid public roads zoom range".into());
    }
    let world = Rect::new(
        Coord {
            x: -180.0,
            y: -85.0,
        },
        Coord { x: 180.0, y: 85.0 },
    );
    let roads = read_public_roads(roads_path, world)?;
    if roads.is_empty() {
        return Err("public road source contains no lines".into());
    }
    let road_bounds = roads
        .iter()
        .filter_map(|road| road.geometry.bounding_rect())
        .reduce(|a, b| {
            Rect::new(
                Coord {
                    x: a.min().x.min(b.min().x),
                    y: a.min().y.min(b.min().y),
                },
                Coord {
                    x: a.max().x.max(b.max().x),
                    y: a.max().y.max(b.max().y),
                },
            )
        })
        .ok_or("public road source contains no bounded lines")?;
    let n = (1u32 << min_zoom) as f64;
    let x0 = (road_bounds.min().x * n).floor() as u32;
    let y0 = (road_bounds.min().y * n).floor() as u32;
    let x1 = (road_bounds.max().x * n).floor() as u32 + 1;
    let y1 = (road_bounds.max().y * n).floor() as u32 + 1;
    let region = Rect::new(
        Coord {
            x: x0 as f64 / n,
            y: y0 as f64 / n,
        },
        Coord {
            x: x1 as f64 / n,
            y: y1 as f64 / n,
        },
    );
    let land = clip_to_region(read_geometries(land_path)?, region);
    let surfaces = read_geometries(surface_path)?;
    let min_lon = x0 as f64 / n * 360.0 - 180.0;
    let max_lon = x1 as f64 / n * 360.0 - 180.0;
    let max_lat = unproject(WorldPoint {
        x: 0.0,
        y: region.min().y,
    })?
    .1;
    let min_lat = unproject(WorldPoint {
        x: 0.0,
        y: region.max().y,
    })?
    .1;
    let metadata = format!(
        "{{\"vector_layers\":[{{\"id\":\"land\",\"fields\":{{}},\"minzoom\":{min_zoom},\"maxzoom\":{max_zoom}}},{{\"id\":\"road_surface\",\"fields\":{{}},\"minzoom\":{min_zoom},\"maxzoom\":{max_zoom}}},{{\"id\":\"road_major\",\"fields\":{{}},\"minzoom\":{min_zoom},\"maxzoom\":{max_zoom}}},{{\"id\":\"road_collector\",\"fields\":{{}},\"minzoom\":{min_zoom},\"maxzoom\":{max_zoom}}},{{\"id\":\"road_local\",\"fields\":{{}},\"minzoom\":{min_zoom},\"maxzoom\":{max_zoom}}}]}}"
    );
    let road_center = unproject(WorldPoint {
        x: (road_bounds.min().x + road_bounds.max().x) / 2.0,
        y: (road_bounds.min().y + road_bounds.max().y) / 2.0,
    })?;
    let file = File::create(output)?;
    let mut writer = PmTilesWriter::new(TileType::Mvt)
        .min_zoom(min_zoom)
        .max_zoom(max_zoom)
        .bounds(min_lon, min_lat, max_lon, max_lat)
        .center(road_center.0, road_center.1)
        .center_zoom(min_zoom)
        .metadata(&metadata)
        .create(file)?;
    let mut tiles = 0;
    let mut features = 0;
    for zoom in min_zoom..=max_zoom {
        let shift = zoom - min_zoom;
        let n = (1u32 << zoom) as f64;
        for y in (y0 << shift)..(y1 << shift) {
            for x in (x0 << shift)..(x1 << shift) {
                let key = TileKey::new(zoom, x, y)?;
                let rect = Rect::new(
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
                let mut count = add_polygons(&mut tile, "land", &land, rect, key)?;
                if zoom >= 11 {
                    count += add_polygons(&mut tile, "road_surface", &surfaces, rect, key)?;
                }
                for (layer, class) in [("road_major", 0), ("road_collector", 2), ("road_local", 3)]
                {
                    if (zoom <= 8 && class > 0) || (zoom <= 10 && class > 2) {
                        continue;
                    }
                    count += add_lines(
                        &mut tile,
                        layer,
                        roads
                            .iter()
                            .filter(|road| road.class == class)
                            .map(|road| &road.geometry),
                        rect,
                        key,
                    )?;
                }
                if count > 0 {
                    writer.add_tile(TileCoord::new(zoom, x, y)?, &tile.to_bytes()?)?;
                    tiles += 1;
                    features += count;
                }
            }
        }
    }
    writer.finalize()?;
    Ok((tiles, features))
}

fn read_public_roads(path: &Path, region: Rect<f64>) -> Result<Vec<RoadSource>, DynError> {
    let source = std::fs::read_to_string(path)?;
    let parsed = GeoJson::from_str(&source)?;
    let GeoJson::FeatureCollection(collection) = parsed else {
        return Err("public roads must be a GeoJSON feature collection".into());
    };
    let mut roads = Vec::new();
    for feature in collection.features {
        let width = feature
            .property("rdl_wid")
            .and_then(|value| value.as_i64())
            .unwrap_or(0);
        let Some(geometry) = feature.geometry else {
            continue;
        };
        let converted: Geometry<f64> = (&geometry.value).try_into()?;
        if !converted
            .bounding_rect()
            .is_some_and(|bounds| overlaps(bounds, region))
        {
            continue;
        }
        let class = if width >= 20 {
            0
        } else if width >= 10 {
            2
        } else {
            3
        };
        roads.push(RoadSource {
            geometry: converted.map_coords(|coordinate| {
                let world =
                    project(coordinate.x, coordinate.y).expect("valid public road coordinates");
                Coord {
                    x: world.x,
                    y: world.y,
                }
            }),
            class,
            link: false,
        });
    }
    Ok(roads)
}
