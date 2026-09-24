//! Local PMTiles → MVT → geometry. No service-backed tile source exists.

use geo::{Geometry, LineString, Point, Polygon};
use mappa_map_core::TileKey;
use pmtiles::{AsyncPmTilesReader, MmapBackend, NoCache, TileCoord, TileType};
use std::{io::Read, path::Path, str::FromStr};
use thiserror::Error;

pub mod builder;
pub mod canonical;

pub const EXTENT: f32 = 4096.0;
pub const MAX_TILE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_LAYER_FEATURES: usize = 50_000;

#[derive(Debug, Error)]
pub enum MapDataError {
    #[error("PMTiles: {0}")]
    Pmtiles(#[from] pmtiles::PmtError),
    #[error("MVT: {0}")]
    Mvt(#[from] mvt_reader::error::ParserError),
    #[error("unsupported tile type: expected MVT")]
    UnsupportedTileType,
    #[error("tile exceeds {MAX_TILE_BYTES} bytes")]
    TileTooLarge,
    #[error("invalid tile geometry or extent")]
    InvalidGeometry,
    #[error("decoder panicked on malformed tile")]
    DecoderPanicked,
    #[error("invalid PMTiles header or root directory extent")]
    InvalidArchive,
    #[error("local file: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Default)]
pub struct DecodedTile {
    /// OSM detail archives carry their own land geometry in overlapping chunks.
    pub detailed: bool,
    pub land: Vec<Polygon<f32>>,
    pub green: Vec<Polygon<f32>>,
    pub water: Vec<Polygon<f32>>,
    pub road_surface: Vec<Polygon<f32>>,
    pub boundary: Vec<LineString<f32>>,
    pub waterway: Vec<LineString<f32>>,
    pub road: Vec<LineString<f32>>,
    pub road_major: Vec<LineString<f32>>,
    pub road_collector: Vec<LineString<f32>>,
    pub road_local: Vec<LineString<f32>>,
    pub place: Vec<MapPlace>,
    pub raw_bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceKind {
    City,
    Station,
    Civic,
    District,
}

#[derive(Debug)]
pub struct MapPlace {
    pub point: Point<f32>,
    pub name: String,
    pub rank: u8,
    pub kind: PlaceKind,
}

#[derive(Debug)]
pub struct CountryLabel {
    pub longitude: f64,
    pub latitude: f64,
    pub name: String,
    pub rank: u8,
}

pub fn load_country_labels(path: &Path) -> Result<Vec<CountryLabel>, MapDataError> {
    let data = std::fs::read_to_string(path)?;
    let geojson = geojson::GeoJson::from_str(&data).map_err(|_| MapDataError::InvalidGeometry)?;
    let geojson::GeoJson::FeatureCollection(collection) = geojson else {
        return Err(MapDataError::InvalidGeometry);
    };
    let mut labels = Vec::new();
    for feature in collection.features {
        let name = feature
            .property("NAME_KO")
            .and_then(serde_json::Value::as_str)
            .filter(|name| !name.is_empty())
            .or_else(|| feature.property("NAME").and_then(serde_json::Value::as_str))
            .filter(|name| !name.is_empty() && name.len() <= 128);
        let lon = feature
            .property("LABEL_X")
            .and_then(serde_json::Value::as_f64);
        let lat = feature
            .property("LABEL_Y")
            .and_then(serde_json::Value::as_f64);
        let rank = feature
            .property("LABELRANK")
            .and_then(serde_json::Value::as_u64);
        if let (Some(name), Some(longitude), Some(latitude), Some(rank)) = (name, lon, lat, rank)
            && longitude.is_finite()
            && latitude.is_finite()
            && (-180.0..=180.0).contains(&longitude)
            && (-90.0..=90.0).contains(&latitude)
            && rank <= u8::MAX as u64
        {
            labels.push(CountryLabel {
                longitude,
                latitude,
                name: name.to_owned(),
                rank: rank as u8,
            });
        }
    }
    Ok(labels)
}

impl DecodedTile {
    /// Conservative payload estimate for the CPU cache. Allocator overhead is separate.
    pub fn estimated_bytes(&self) -> usize {
        use geo::CoordsIter;
        let land = self.land.iter().map(|p| p.coords_count()).sum::<usize>();
        let green = self.green.iter().map(|p| p.coords_count()).sum::<usize>();
        let water = self.water.iter().map(|p| p.coords_count()).sum::<usize>();
        let road_surface = self
            .road_surface
            .iter()
            .map(|p| p.coords_count())
            .sum::<usize>();
        let boundary = self
            .boundary
            .iter()
            .map(|l| l.coords_count())
            .sum::<usize>();
        let waterway = self
            .waterway
            .iter()
            .map(|l| l.coords_count())
            .sum::<usize>();
        let road = self.road.iter().map(|l| l.coords_count()).sum::<usize>();
        let road_major = self
            .road_major
            .iter()
            .map(|l| l.coords_count())
            .sum::<usize>();
        let road_collector = self
            .road_collector
            .iter()
            .map(|l| l.coords_count())
            .sum::<usize>();
        let road_local = self
            .road_local
            .iter()
            .map(|l| l.coords_count())
            .sum::<usize>();
        (land
            + green
            + water
            + road_surface
            + boundary
            + waterway
            + road
            + road_major
            + road_collector
            + road_local)
            * std::mem::size_of::<geo::Coord<f32>>()
            + (self.land.capacity()
                + self.green.capacity()
                + self.water.capacity()
                + self.road_surface.capacity())
                * std::mem::size_of::<Polygon<f32>>()
            + (self.boundary.capacity()
                + self.waterway.capacity()
                + self.road.capacity()
                + self.road_major.capacity()
                + self.road_collector.capacity()
                + self.road_local.capacity())
                * std::mem::size_of::<LineString<f32>>()
            + self.place.capacity() * std::mem::size_of::<MapPlace>()
            + self.place.iter().map(|p| p.name.capacity()).sum::<usize>()
    }
}

pub trait TileSource {
    fn tile_bytes(
        &self,
        key: TileKey,
    ) -> impl std::future::Future<Output = Result<Option<Vec<u8>>, MapDataError>> + Send;
}

pub struct LocalPmTiles {
    reader: AsyncPmTilesReader<MmapBackend, NoCache>,
    pub min_zoom: u8,
    pub max_zoom: u8,
    pub bounds: [f64; 4],
    pub center: [f64; 2],
    pub attribution: Option<String>,
}

impl LocalPmTiles {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, MapDataError> {
        // pmtiles-rs currently slices the initial root directory using header offsets.
        // Validate those bounds before handing it untrusted local files.
        let mut file = std::fs::File::open(path.as_ref())?;
        let file_len = file.metadata()?.len();
        if file_len < 127 {
            return Err(MapDataError::InvalidArchive);
        }
        let mut prefix = [0u8; 24];
        file.read_exact(&mut prefix)?;
        let root_offset = u64::from_le_bytes(
            prefix[8..16]
                .try_into()
                .map_err(|_| MapDataError::InvalidArchive)?,
        );
        let root_length = u64::from_le_bytes(
            prefix[16..24]
                .try_into()
                .map_err(|_| MapDataError::InvalidArchive)?,
        );
        if &prefix[..7] != b"PMTiles"
            || prefix[7] != 3
            || root_offset < 127
            || root_length == 0
            || root_offset
                .checked_add(root_length)
                .is_none_or(|end| end > file_len || end > 16_384)
        {
            return Err(MapDataError::InvalidArchive);
        }
        let reader = AsyncPmTilesReader::new_with_path(path).await?;
        if reader.get_header().tile_type != TileType::Mvt {
            return Err(MapDataError::UnsupportedTileType);
        }
        let header = reader.get_header();
        let min_zoom = header.min_zoom;
        let max_zoom = header.max_zoom;
        let bounds = [
            header.min_longitude,
            header.min_latitude,
            header.max_longitude,
            header.max_latitude,
        ];
        let center = [header.center_longitude, header.center_latitude];
        let attribution = reader
            .get_metadata()
            .await
            .ok()
            .and_then(|metadata| serde_json::from_str::<serde_json::Value>(&metadata).ok())
            .and_then(|metadata| metadata.get("attribution")?.as_str().map(str::to_owned))
            .filter(|value| !value.is_empty() && value.len() <= 512);
        Ok(Self {
            reader,
            min_zoom,
            max_zoom,
            bounds,
            center,
            attribution,
        })
    }
}

impl TileSource for LocalPmTiles {
    async fn tile_bytes(&self, key: TileKey) -> Result<Option<Vec<u8>>, MapDataError> {
        let coord = TileCoord::new(key.z, key.x, key.y)?;
        let bytes = self.reader.get_tile_decompressed(coord).await?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        if bytes.len() > MAX_TILE_BYTES {
            return Err(MapDataError::TileTooLarge);
        }
        Ok(Some(bytes.to_vec()))
    }
}

pub fn decode_mvt(bytes: Vec<u8>) -> Result<DecodedTile, MapDataError> {
    if bytes.len() > MAX_TILE_BYTES {
        return Err(MapDataError::TileTooLarge);
    }
    std::panic::catch_unwind(|| decode_inner(bytes)).map_err(|_| MapDataError::DecoderPanicked)?
}

fn decode_inner(bytes: Vec<u8>) -> Result<DecodedTile, MapDataError> {
    let raw_bytes = bytes.len();
    let reader = mvt_reader::Reader::new(bytes)?;
    let metadata = reader.get_layer_metadata()?;
    let mut tile = DecodedTile {
        raw_bytes,
        ..Default::default()
    };
    for layer in metadata {
        if layer.name == "road_major" {
            tile.detailed = true;
        }
        if layer.extent != EXTENT as u32 || layer.feature_count > MAX_LAYER_FEATURES {
            return Err(MapDataError::InvalidGeometry);
        }
        if !matches!(
            layer.name.as_str(),
            "land"
                | "green"
                | "water"
                | "road_surface"
                | "boundary"
                | "waterway"
                | "road"
                | "road_major"
                | "road_collector"
                | "road_local"
                | "place"
        ) {
            continue;
        }
        for feature in reader.get_features(layer.layer_index)? {
            if !geometry_finite(&feature.geometry) {
                return Err(MapDataError::InvalidGeometry);
            }
            let properties = feature.properties.unwrap_or_default();
            match (layer.name.as_str(), feature.geometry) {
                ("land", Geometry::Polygon(p)) => tile.land.push(p),
                ("land", Geometry::MultiPolygon(mp)) => tile.land.extend(mp.0),
                ("green", Geometry::Polygon(p)) => tile.green.push(p),
                ("green", Geometry::MultiPolygon(mp)) => tile.green.extend(mp.0),
                ("water", Geometry::Polygon(p)) => tile.water.push(p),
                ("water", Geometry::MultiPolygon(mp)) => tile.water.extend(mp.0),
                ("road_surface", Geometry::Polygon(p)) => tile.road_surface.push(p),
                ("road_surface", Geometry::MultiPolygon(mp)) => tile.road_surface.extend(mp.0),
                ("boundary", Geometry::LineString(l)) => tile.boundary.push(l),
                ("boundary", Geometry::MultiLineString(ml)) => tile.boundary.extend(ml.0),
                ("waterway", Geometry::LineString(l)) => tile.waterway.push(l),
                ("waterway", Geometry::MultiLineString(ml)) => tile.waterway.extend(ml.0),
                ("road", Geometry::LineString(l)) => tile.road.push(l),
                ("road", Geometry::MultiLineString(ml)) => tile.road.extend(ml.0),
                ("road_major", Geometry::LineString(l)) => tile.road_major.push(l),
                ("road_major", Geometry::MultiLineString(ml)) => tile.road_major.extend(ml.0),
                ("road_collector", Geometry::LineString(l)) => tile.road_collector.push(l),
                ("road_collector", Geometry::MultiLineString(ml)) => {
                    tile.road_collector.extend(ml.0)
                }
                ("road_local", Geometry::LineString(l)) => tile.road_local.push(l),
                ("road_local", Geometry::MultiLineString(ml)) => tile.road_local.extend(ml.0),
                ("place", Geometry::Point(point)) => {
                    tile.place.push(decode_place(&properties, point)?);
                }
                ("place", Geometry::MultiPoint(points)) if points.0.len() == 1 => {
                    tile.place.push(decode_place(&properties, points.0[0])?);
                }
                _ => return Err(MapDataError::InvalidGeometry),
            }
        }
    }
    Ok(tile)
}

fn decode_place(
    properties: &std::collections::HashMap<String, mvt_reader::feature::Value>,
    point: Point<f32>,
) -> Result<MapPlace, MapDataError> {
    let Some(mvt_reader::feature::Value::String(name)) = properties.get("name") else {
        return Err(MapDataError::InvalidGeometry);
    };
    let Some(mvt_reader::feature::Value::UInt(rank)) = properties.get("rank") else {
        return Err(MapDataError::InvalidGeometry);
    };
    if name.is_empty() || name.len() > 128 || *rank > u8::MAX as u64 {
        return Err(MapDataError::InvalidGeometry);
    }
    let kind = match properties.get("kind") {
        None => PlaceKind::City,
        Some(mvt_reader::feature::Value::String(kind)) if kind == "city" => PlaceKind::City,
        Some(mvt_reader::feature::Value::String(kind)) if kind == "station" => PlaceKind::Station,
        Some(mvt_reader::feature::Value::String(kind)) if kind == "civic" => PlaceKind::Civic,
        Some(mvt_reader::feature::Value::String(kind)) if kind == "district" => PlaceKind::District,
        _ => return Err(MapDataError::InvalidGeometry),
    };
    Ok(MapPlace {
        point,
        name: name.clone(),
        rank: *rank as u8,
        kind,
    })
}

fn geometry_finite(geometry: &Geometry<f32>) -> bool {
    use geo::CoordsIter;
    geometry.coords_iter().all(|c| {
        c.x.is_finite() && c.y.is_finite() && c.x.abs() <= 1_000_000.0 && c.y.abs() <= 1_000_000.0
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_mvt_is_rejected_without_panic() {
        for bytes in [
            vec![0xff],
            vec![0x08, 0xff, 0xff, 0xff, 0xff, 0xff],
            vec![0u8; 17],
            vec![0xff; 100],
        ] {
            let _ = decode_mvt(bytes);
        }
        assert!(matches!(
            decode_mvt(vec![0; MAX_TILE_BYTES + 1]),
            Err(MapDataError::TileTooLarge)
        ));
    }
    #[tokio::test]
    async fn bundled_world_has_real_layers() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/map/world_110m.pmtiles"
        );
        let source = LocalPmTiles::open(path).await.unwrap();
        let bytes = source
            .tile_bytes(TileKey::new(0, 0, 0).unwrap())
            .await
            .unwrap()
            .unwrap();
        let tile = decode_mvt(bytes).unwrap();
        assert!(!tile.land.is_empty());
        assert!(!tile.water.is_empty());
        assert!(!tile.boundary.is_empty());
    }
    #[tokio::test]
    async fn bundled_world_detail_contains_source_river_geometry() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/map/world_10m.pmtiles"
        );
        let source = LocalPmTiles::open(path).await.unwrap();
        let center = mappa_map_core::project(-60.0, -3.0).unwrap();
        let key = TileKey::new(6, (center.x * 64.0) as u32, (center.y * 64.0) as u32).unwrap();
        let tile = decode_mvt(source.tile_bytes(key).await.unwrap().unwrap()).unwrap();
        assert!(!tile.waterway.is_empty());
    }
    #[tokio::test]
    async fn bundled_detail_has_roads_and_places() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/map/east_asia_10m.pmtiles"
        );
        let source = LocalPmTiles::open(path).await.unwrap();
        assert_eq!((source.min_zoom, source.max_zoom), (5, 7));
        let z5 = source
            .tile_bytes(TileKey::new(5, 27, 12).unwrap())
            .await
            .unwrap()
            .unwrap();
        let coast = decode_mvt(z5).unwrap();
        assert!(!coast.land.is_empty());
        assert!(coast.road.is_empty() && coast.place.is_empty());
        let bytes = source
            .tile_bytes(TileKey::new(6, 54, 24).unwrap())
            .await
            .unwrap()
            .unwrap();
        let tile = decode_mvt(bytes).unwrap();
        assert!(!tile.road.is_empty());
        assert!(tile.place.iter().any(|place| place.name == "Seoul"));
    }
    #[tokio::test]
    async fn bundled_street_tile_has_real_stations_and_civic_places() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/map/seoul_osm_streets.pmtiles"
        );
        let source = LocalPmTiles::open(path).await.unwrap();
        assert_eq!((source.min_zoom, source.max_zoom), (10, 12));
        let city_raw = source
            .tile_bytes(TileKey::new(10, 873, 396).unwrap())
            .await
            .unwrap()
            .unwrap();
        let city = decode_mvt(city_raw).unwrap();
        assert!(city.detailed);
        assert!(!city.road_major.is_empty());
        assert!(!city.green.is_empty());
        assert!(city.place.iter().any(|p| p.name == "서울특별시"));
        let station = decode_mvt(
            source
                .tile_bytes(TileKey::new(12, 3492, 1586).unwrap())
                .await
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        assert!(
            station
                .place
                .iter()
                .any(|p| p.kind == PlaceKind::Station && p.name == "광화문")
        );
        assert!(
            station
                .place
                .iter()
                .any(|p| p.kind == PlaceKind::Civic && p.name == "서울특별시청")
        );
    }
    #[tokio::test]
    async fn bundled_korea_tile_has_osm_roads_and_cities() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/map/korea_osm_roads.pmtiles"
        );
        let source = LocalPmTiles::open(path).await.unwrap();
        assert_eq!((source.min_zoom, source.max_zoom), (8, 9));
        let raw = source
            .tile_bytes(TileKey::new(8, 218, 99).unwrap())
            .await
            .unwrap()
            .unwrap();
        let tile = decode_mvt(raw).unwrap();
        assert!(tile.detailed);
        assert!(!tile.road_major.is_empty());
        assert!(tile.place.iter().any(|p| p.name == "서울특별시"));
        assert!(tile.place.iter().all(|p| p.kind == PlaceKind::City));
    }
    #[tokio::test]
    async fn bundled_public_roads_use_source_geometry_and_center() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/map/naju_public_roads.pmtiles"
        );
        let source = LocalPmTiles::open(path).await.unwrap();
        assert_eq!((source.min_zoom, source.max_zoom), (8, 12));
        assert!((126.674_854..=126.807_885).contains(&source.center[0]));
        assert!((34.967_209..=35.071_595).contains(&source.center[1]));
        let raw = source
            .tile_bytes(TileKey::new(8, 218, 101).unwrap())
            .await
            .unwrap()
            .unwrap();
        let tile = decode_mvt(raw).unwrap();
        assert!(!tile.road_major.is_empty());
        assert!(tile.place.is_empty());
    }
    #[test]
    fn country_names_are_from_source_label_coordinates() {
        let path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/map/source/ne_110m_admin_0_countries.geojson"
        ));
        let countries = load_country_labels(path).unwrap();
        assert!(countries.iter().any(|c| c.name == "대한민국"));
        assert!(countries.iter().any(|c| c.name == "일본"));
    }
    #[tokio::test]
    async fn invalid_archive_header_is_error() {
        let path =
            std::env::temp_dir().join(format!("mappa-invalid-pmtiles-{}", std::process::id()));
        let mut header = vec![0u8; 127];
        header[..7].copy_from_slice(b"PMTiles");
        header[7] = 3;
        header[8..16].copy_from_slice(&126u64.to_le_bytes());
        header[16..24].copy_from_slice(&100u64.to_le_bytes());
        std::fs::write(&path, &header).unwrap();
        assert!(matches!(
            LocalPmTiles::open(&path).await,
            Err(MapDataError::InvalidArchive)
        ));
        std::fs::remove_file(path).unwrap();
    }
}
