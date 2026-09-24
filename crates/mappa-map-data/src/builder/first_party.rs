//! Tiles made only from Mappa field records. No fallback basemap is read here.

use super::{DynError, PlaceSource, add_lines, add_places, add_polygons};
use geo::{BoundingRect, Coord, Geometry, LineString, MapCoords, Rect, Validation};
use geojson::{Feature, GeoJson, GeometryValue};
use mappa_map_core::{MAX_LAT, TileKey, project};
use mvt::Tile;
use pmtiles::{PmTilesWriter, TileCoord, TileType};
use serde_json::Value as JsonValue;
use std::{collections::BTreeSet, fs::File, path::Path, str::FromStr};

use crate::PlaceKind;

pub const MAX_DATA_ZOOM: u8 = 14;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SurveyStats {
    pub areas: usize,
    pub boundaries: usize,
    pub roads: usize,
    pub places: usize,
    pub tiles: usize,
}

struct Survey {
    areas: Vec<(Geometry<f64>, &'static str)>,
    boundaries: Vec<Geometry<f64>>,
    roads: Vec<(Geometry<f64>, &'static str)>,
    places: Vec<PlaceSource>,
    center: Option<(f64, f64)>,
    bounds: Option<(f64, f64, f64, f64)>,
}

impl Survey {
    fn include_position(&mut self, lon: f64, lat: f64) {
        self.center.get_or_insert((lon, lat));
        self.bounds = Some(match self.bounds {
            Some((west, south, east, north)) => {
                (west.min(lon), south.min(lat), east.max(lon), north.max(lat))
            }
            None => (lon, lat, lon, lat),
        });
    }
}

fn coordinate(raw: &[f64]) -> Result<(f64, f64), DynError> {
    let [lon, lat] = raw else {
        return Err("coordinate must contain longitude and latitude only".into());
    };
    if !lon.is_finite()
        || !lat.is_finite()
        || !(-180.0..180.0).contains(lon)
        || !(-MAX_LAT..=MAX_LAT).contains(lat)
    {
        return Err("coordinate outside supported WGS84 map range".into());
    }
    Ok((*lon, *lat))
}

fn property<'a>(feature: &'a Feature, key: &str) -> Result<&'a str, DynError> {
    feature
        .property(key)
        .and_then(JsonValue::as_str)
        .filter(|text| !text.is_empty() && text.len() <= 128)
        .ok_or_else(|| format!("missing or invalid {key}").into())
}

fn utc_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
        || !bytes
            .iter()
            .enumerate()
            .all(|(i, c)| matches!(i, 4 | 7 | 10 | 13 | 16 | 19) || c.is_ascii_digit())
    {
        return false;
    }
    let number = |start, end| value[start..end].parse::<u32>().unwrap_or(0);
    let year = number(0, 4);
    let month = number(5, 7);
    let day = number(8, 10);
    let leap_year =
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let month_days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => 0,
    };
    (1970..=9999).contains(&year)
        && (1..=month_days).contains(&day)
        && number(11, 13) < 24
        && number(14, 16) < 60
        && number(17, 19) < 60
}

fn read(path: &Path) -> Result<Survey, DynError> {
    let source = std::fs::read_to_string(path)?;
    let GeoJson::FeatureCollection(collection) = GeoJson::from_str(&source)? else {
        return Err("survey must be a GeoJSON FeatureCollection".into());
    };
    let mut survey = Survey {
        areas: Vec::new(),
        boundaries: Vec::new(),
        roads: Vec::new(),
        places: Vec::new(),
        center: None,
        bounds: None,
    };
    for (index, feature) in collection.features.iter().enumerate() {
        let result = (|| -> Result<(), DynError> {
            if property(feature, "origin")? != "mappa-field-survey" {
                return Err("origin must be mappa-field-survey".into());
            }
            let method = property(feature, "method")?;
            if method != "device-gps" && method != "field-note" {
                return Err("method must be device-gps or field-note".into());
            }
            let observed_at = property(feature, "observed_at")?;
            if !utc_timestamp(observed_at) {
                return Err("observed_at must be a UTC timestamp".into());
            }
            let accuracy = feature
                .property("accuracy_m")
                .and_then(JsonValue::as_f64)
                .ok_or("missing accuracy_m")?;
            if !accuracy.is_finite() || !(0.0 < accuracy && accuracy <= 100.0) {
                return Err("accuracy_m must be greater than 0 and at most 100".into());
            }
            let name = property(feature, "name")?.to_owned();
            let kind = property(feature, "kind")?;
            let geometry = feature.geometry.as_ref().ok_or("missing geometry")?;
            match (&geometry.value, kind) {
                (GeometryValue::Point { coordinates: raw }, "city" | "station" | "civic") => {
                    let (lon, lat) = coordinate(raw.as_slice())?;
                    let world = project(lon, lat)?;
                    survey.include_position(lon, lat);
                    survey.places.push(PlaceSource {
                        point: Coord {
                            x: world.x,
                            y: world.y,
                        },
                        name,
                        rank: 1,
                        kind: match kind {
                            "city" => PlaceKind::City,
                            "station" => PlaceKind::Station,
                            _ => PlaceKind::Civic,
                        },
                    });
                }
                (
                    GeometryValue::LineString { coordinates: raw },
                    "boundary" | "road-major" | "road-collector" | "road-local",
                ) => {
                    if raw.len() < 2 {
                        return Err("road needs at least two recorded positions".into());
                    }
                    let coords = raw
                        .iter()
                        .map(|p| coordinate(p.as_slice()))
                        .collect::<Result<Vec<_>, _>>()?;
                    if coords.windows(2).all(|pair| pair[0] == pair[1]) {
                        return Err("road needs two distinct positions".into());
                    }
                    let (min_lon, max_lon) =
                        coords.iter().fold((180.0_f64, -180.0_f64), |(lo, hi), p| {
                            (lo.min(p.0), hi.max(p.0))
                        });
                    if max_lon - min_lon >= 180.0 {
                        return Err("road crossing the antimeridian is not supported yet".into());
                    }
                    for &(lon, lat) in &coords {
                        survey.include_position(lon, lat);
                    }
                    let line = LineString::from(coords);
                    let projected = Geometry::LineString(line).map_coords(|c| {
                        let p = project(c.x, c.y).expect("validated coordinate");
                        Coord { x: p.x, y: p.y }
                    });
                    if kind == "boundary" {
                        survey.boundaries.push(projected);
                    } else {
                        survey.roads.push((
                            projected,
                            match kind {
                                "road-major" => "road_major",
                                "road-collector" => "road_collector",
                                _ => "road_local",
                            },
                        ));
                    }
                }
                (GeometryValue::Polygon { coordinates: rings }, "land" | "water") => {
                    if rings.is_empty()
                        || rings
                            .iter()
                            .any(|ring| ring.len() < 4 || ring.first() != ring.last())
                    {
                        return Err("area needs closed rings with at least four positions".into());
                    }
                    let mut min_lon = 180.0_f64;
                    let mut max_lon = -180.0_f64;
                    for ring in rings {
                        for raw in ring {
                            let (lon, _) = coordinate(raw.as_slice())?;
                            min_lon = min_lon.min(lon);
                            max_lon = max_lon.max(lon);
                        }
                    }
                    if max_lon - min_lon >= 180.0 {
                        return Err("area crossing the antimeridian is not supported yet".into());
                    }
                    let converted: Geometry<f64> = (&geometry.value).try_into()?;
                    let Geometry::Polygon(polygon) = converted else {
                        return Err("invalid area geometry".into());
                    };
                    polygon.check_validation()?;
                    let first = rings[0][0].as_slice();
                    survey.include_position(first[0], first[1]);
                    for ring in rings {
                        for raw in ring {
                            survey.include_position(raw[0], raw[1]);
                        }
                    }
                    let projected = Geometry::Polygon(polygon).map_coords(|c| {
                        let p = project(c.x, c.y).expect("validated coordinate");
                        Coord { x: p.x, y: p.y }
                    });
                    survey
                        .areas
                        .push((projected, if kind == "land" { "land" } else { "water" }));
                }
                _ => return Err("kind and geometry do not match a supported field record".into()),
            }
            Ok(())
        })();
        result.map_err(|error| format!("feature {}: {error}", index + 1))?;
    }
    Ok(survey)
}

pub fn inspect(path: &Path) -> Result<SurveyStats, DynError> {
    let survey = read(path)?;
    Ok(SurveyStats {
        areas: survey.areas.len(),
        boundaries: survey.boundaries.len(),
        roads: survey.roads.len(),
        places: survey.places.len(),
        tiles: 0,
    })
}

/// Builds a local archive from one validated Mappa survey file.
pub fn build(source: &Path, output: &Path) -> Result<SurveyStats, DynError> {
    let survey = read(source)?;
    let land: Vec<_> = survey
        .areas
        .iter()
        .filter(|(_, class)| *class == "land")
        .map(|(geometry, _)| geometry.clone())
        .collect();
    let water: Vec<_> = survey
        .areas
        .iter()
        .filter(|(_, class)| *class == "water")
        .map(|(geometry, _)| geometry.clone())
        .collect();
    let metadata = format!(
        "{{\"name\":\"Mappa field survey\",\"origin\":\"mappa-field-survey\",\"area_count\":{},\"boundary_count\":{},\"road_count\":{},\"place_count\":{},\"vector_layers\":[{{\"id\":\"land\",\"fields\":{{}},\"minzoom\":0,\"maxzoom\":{MAX_DATA_ZOOM}}},{{\"id\":\"water\",\"fields\":{{}},\"minzoom\":0,\"maxzoom\":{MAX_DATA_ZOOM}}},{{\"id\":\"boundary\",\"fields\":{{}},\"minzoom\":0,\"maxzoom\":{MAX_DATA_ZOOM}}},{{\"id\":\"road_major\",\"fields\":{{}},\"minzoom\":0,\"maxzoom\":{MAX_DATA_ZOOM}}},{{\"id\":\"road_collector\",\"fields\":{{}},\"minzoom\":0,\"maxzoom\":{MAX_DATA_ZOOM}}},{{\"id\":\"road_local\",\"fields\":{{}},\"minzoom\":0,\"maxzoom\":{MAX_DATA_ZOOM}}},{{\"id\":\"place\",\"fields\":{{\"name\":\"String\",\"kind\":\"String\"}},\"minzoom\":0,\"maxzoom\":{MAX_DATA_ZOOM}}}]}}",
        survey.areas.len(),
        survey.boundaries.len(),
        survey.roads.len(),
        survey.places.len()
    );
    let mut keys = BTreeSet::new();
    for zoom in 0..=MAX_DATA_ZOOM {
        let n = (1u32 << zoom) as f64;
        for bounds in survey
            .areas
            .iter()
            .filter_map(|(geom, _)| geom.bounding_rect())
            .chain(survey.boundaries.iter().filter_map(Geometry::bounding_rect))
            .chain(
                survey
                    .roads
                    .iter()
                    .filter_map(|(geom, _)| geom.bounding_rect()),
            )
            .chain(survey.places.iter().map(|p| Rect::new(p.point, p.point)))
        {
            let x0 = (bounds.min().x * n).floor() as u32;
            let x1 = (bounds.max().x * n).floor() as u32;
            let y0 = (bounds.min().y * n).floor() as u32;
            let y1 = (bounds.max().y * n).floor() as u32;
            for y in y0.min((n as u32) - 1)..=y1.min((n as u32) - 1) {
                for x in x0.min((n as u32) - 1)..=x1.min((n as u32) - 1) {
                    keys.insert(TileKey::new(zoom, x, y)?);
                    if keys.len() > 20_000 {
                        return Err("survey covers too many tiles for one archive".into());
                    }
                }
            }
        }
    }
    let temporary = output.with_extension("pmtiles.tmp");
    let file = File::create(&temporary)?;
    let center = survey.center.unwrap_or((0.0, 0.0));
    let bounds = survey.bounds.unwrap_or((-180.0, -MAX_LAT, 180.0, MAX_LAT));
    let mut writer = PmTilesWriter::new(TileType::Mvt)
        .min_zoom(0)
        .max_zoom(MAX_DATA_ZOOM)
        .bounds(bounds.0, bounds.1, bounds.2, bounds.3)
        .center(center.0, center.1)
        .center_zoom(if survey.center.is_some() { 12 } else { 0 })
        .metadata(&metadata)
        .create(file)?;
    let mut stats = SurveyStats {
        areas: survey.areas.len(),
        boundaries: survey.boundaries.len(),
        roads: survey.roads.len(),
        places: survey.places.len(),
        tiles: 0,
    };
    for key in keys {
        let n = (1u32 << key.z) as f64;
        let rect = Rect::new(
            Coord {
                x: key.x as f64 / n,
                y: key.y as f64 / n,
            },
            Coord {
                x: (key.x + 1) as f64 / n,
                y: (key.y + 1) as f64 / n,
            },
        );
        let mut tile = Tile::new(4096);
        let mut count = 0;
        count += add_polygons(&mut tile, "land", &land, rect, key)?;
        count += add_polygons(&mut tile, "water", &water, rect, key)?;
        count += add_lines(&mut tile, "boundary", &survey.boundaries, rect, key)?;
        for layer in ["road_major", "road_collector", "road_local"] {
            count += add_lines(
                &mut tile,
                layer,
                survey
                    .roads
                    .iter()
                    .filter(|(_, class)| *class == layer)
                    .map(|(geom, _)| geom),
                rect,
                key,
            )?;
        }
        count += add_places(&mut tile, &survey.places, rect, key)?;
        if count > 0 {
            writer.add_tile(TileCoord::new(key.z, key.x, key.y)?, &tile.to_bytes()?)?;
            stats.tiles += 1;
        }
    }
    writer.finalize()?;
    std::fs::rename(temporary, output)?;
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LocalPmTiles, TileSource, decode_mvt};

    fn temporary(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "mappa-first-party-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn reject_unattributed_geometry() {
        let path = temporary("invalid.geojson");
        std::fs::write(&path, r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"kind":"road-local","name":"Road"},"geometry":{"type":"LineString","coordinates":[[127,37],[127.001,37]]}}]}"#).unwrap();
        assert!(read(&path).err().unwrap().to_string().contains("origin"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn utc_time_has_a_fixed_recording_format() {
        assert!(utc_timestamp("2026-09-24T00:00:00Z"));
        assert!(!utc_timestamp("yesterdayTZ"));
        assert!(!utc_timestamp("2026-13-24T00:00:00Z"));
        assert!(!utc_timestamp("2026-02-29T00:00:00Z"));
        assert!(utc_timestamp("2028-02-29T00:00:00Z"));
    }

    #[test]
    fn rejects_unclosed_recorded_area() {
        let path = temporary("unclosed.geojson");
        std::fs::write(&path, r#"{"type":"FeatureCollection","features":[
            {"type":"Feature","properties":{"origin":"mappa-field-survey","method":"field-note","observed_at":"2026-09-24T00:00:00Z","accuracy_m":5,"name":"Measured shore","kind":"land"},"geometry":{"type":"Polygon","coordinates":[[[127,37],[127.001,37],[127.001,37.001],[127,37.001]]]}}
        ]}"#).unwrap();
        assert!(
            read(&path)
                .err()
                .unwrap()
                .to_string()
                .contains("closed rings")
        );
        std::fs::remove_file(path).unwrap();
    }

    #[tokio::test]
    async fn empty_survey_opens_without_geography() {
        let source = temporary("empty.geojson");
        let output = temporary("empty.pmtiles");
        std::fs::write(&source, r#"{"type":"FeatureCollection","features":[]}"#).unwrap();
        assert_eq!(build(&source, &output).unwrap(), SurveyStats::default());
        let archive = LocalPmTiles::open(&output).await.unwrap();
        assert!(
            archive
                .tile_bytes(TileKey::new(0, 0, 0).unwrap())
                .await
                .unwrap()
                .is_none()
        );
        std::fs::remove_file(source).unwrap();
        std::fs::remove_file(output).unwrap();
    }

    #[tokio::test]
    async fn recorded_geometry_round_trips_through_tiles() {
        let source = temporary("recorded.geojson");
        let output = temporary("recorded.pmtiles");
        std::fs::write(&source, r#"{"type":"FeatureCollection","features":[
            {"type":"Feature","properties":{"origin":"mappa-field-survey","method":"field-note","observed_at":"2026-09-24T00:00:00Z","accuracy_m":5,"name":"Measured land","kind":"land"},"geometry":{"type":"Polygon","coordinates":[[[127,37],[127.001,37],[127.001,37.001],[127,37.001],[127,37]]]}},
            {"type":"Feature","properties":{"origin":"mappa-field-survey","method":"field-note","observed_at":"2026-09-24T00:00:00Z","accuracy_m":5,"name":"Measured pond","kind":"water"},"geometry":{"type":"Polygon","coordinates":[[[127.0001,37.0001],[127.0004,37.0001],[127.0004,37.0004],[127.0001,37.0004],[127.0001,37.0001]]]}},
            {"type":"Feature","properties":{"origin":"mappa-field-survey","method":"field-note","observed_at":"2026-09-24T00:00:00Z","accuracy_m":5,"name":"Measured boundary","kind":"boundary"},"geometry":{"type":"LineString","coordinates":[[127,37],[127.0005,37.0002]]}},
            {"type":"Feature","properties":{"origin":"mappa-field-survey","method":"field-note","observed_at":"2026-09-24T00:00:00Z","accuracy_m":5,"name":"Observed road","kind":"road-local"},"geometry":{"type":"LineString","coordinates":[[127,37],[127.0005,37.0002]]}},
            {"type":"Feature","properties":{"origin":"mappa-field-survey","method":"field-note","observed_at":"2026-09-24T00:00:00Z","accuracy_m":5,"name":"Observed station","kind":"station"},"geometry":{"type":"Point","coordinates":[127.0001,37.0001]}}
        ]}"#).unwrap();
        let stats = build(&source, &output).unwrap();
        assert_eq!(stats.areas, 2);
        assert_eq!(stats.boundaries, 1);
        assert_eq!(stats.roads, 1);
        assert_eq!(stats.places, 1);
        assert!(stats.tiles > 0);
        let archive = LocalPmTiles::open(&output).await.unwrap();
        let world = project(127.0001, 37.0001).unwrap();
        let n = 1u32 << MAX_DATA_ZOOM;
        let key = TileKey::new(
            MAX_DATA_ZOOM,
            (world.x * n as f64) as u32,
            (world.y * n as f64) as u32,
        )
        .unwrap();
        let tile = decode_mvt(archive.tile_bytes(key).await.unwrap().unwrap()).unwrap();
        assert!(!tile.land.is_empty());
        assert!(!tile.water.is_empty());
        assert!(!tile.boundary.is_empty());
        assert!(!tile.road_local.is_empty());
        assert_eq!(tile.place.len(), 1);
        assert_eq!(tile.place[0].name, "Observed station");
        std::fs::remove_file(source).unwrap();
        std::fs::remove_file(output).unwrap();
    }
}
