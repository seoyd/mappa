//! Build-time canonical geography. Raw source fields end at the adapter boundary.

use flate2::read::GzDecoder;
use geo::{
    BooleanOps, Contains, Coord, InteriorPoint, LineString, MultiPolygon, Point, Polygon, Rect,
    Validation,
};
use geojson::{GeoJson, GeometryValue};
use rstar::{AABB, RTree, RTreeObject};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::Path,
    str::FromStr,
};
use thiserror::Error;

mod act_roads;
mod ign_bdtopo;
mod lambert93;
mod lu_geobase;
mod nrn;
mod qld_roads;
mod tas_roads;
mod vic_dtp_roads;
mod vicmap_roads;
mod wa_roads;
pub use act_roads::adapt_au_act_road_centrelines;
pub use ign_bdtopo::{adapt_ign_bdtopo_roads, adapt_ign_bdtopo_stations, adapt_ign_bdtopo_water};
pub use lambert93::inverse_lambert93;
pub use lu_geobase::adapt_lu_geobase_roads;
pub use nrn::adapt_ca_nrn_roads;
pub use qld_roads::adapt_au_qld_qrt_roads;
pub use tas_roads::adapt_au_tas_list_transport_segments;
pub use vic_dtp_roads::adapt_au_vic_dtp_roads;
pub use vicmap_roads::adapt_au_vic_vicmap_roads;
pub use wa_roads::adapt_wa_road_network;

const MAGIC: &[u8; 8] = b"MAPPAGEO";
const SCHEMA_VERSION: u32 = 1;
const HEADER_BYTES: u64 = 40;
const INDEX_BYTES: u64 = 56;
const MAX_FEATURES: u32 = 1_000_000;
const MAX_RECORD_BYTES: u32 = 8 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum CanonicalError {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("manifest: {0}")]
    Manifest(#[from] toml::de::Error),
    #[error("GeoJSON: {0}")]
    GeoJson(#[from] geojson::Error),
    #[error("Shapefile: {0}")]
    Shapefile(#[from] shapefile::Error),
    #[error("dBase: {0}")]
    Dbase(#[from] shapefile::dbase::Error),
    #[error("binary record: {0}")]
    Binary(#[from] Box<bincode::ErrorKind>),
    #[error("source checksum mismatch: {0}")]
    SourceChecksum(String),
    #[error("license gate rejected source: {0}")]
    License(String),
    #[error("invalid source feature: {0}")]
    Feature(String),
    #[error("corrupt MappaGeoDB: {0}")]
    Corrupt(&'static str),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SourceManifest {
    pub schema_version: u32,
    pub proof_region: String,
    pub proof_bbox_wgs84: [f64; 4],
    pub source: Vec<SourceRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SourceRecord {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub source_version: String,
    pub download_date: String,
    pub official_url: String,
    pub download_page_url: String,
    pub file: String,
    pub sha256: String,
    #[serde(default)]
    pub upstream_file: Option<String>,
    #[serde(default)]
    pub upstream_sha256: Option<String>,
    #[serde(default)]
    pub dedup_file: Option<String>,
    #[serde(default)]
    pub dedup_sha256: Option<String>,
    pub crs: String,
    pub format: String,
    pub coverage: String,
    pub resolution: String,
    pub update_frequency: String,
    pub license_id: String,
    pub license_url: String,
    pub license_status: String,
    pub commercial_use: bool,
    pub modification: bool,
    pub redistribution: bool,
    pub attribution_required: bool,
    #[serde(default)]
    pub attribution_text: Option<String>,
    pub share_alike: bool,
    pub adapter: String,
    pub adapter_version: u32,
}

impl SourceManifest {
    pub fn open(path: &Path) -> Result<Self, CanonicalError> {
        let mut manifest: Self = toml::from_str(&fs::read_to_string(path)?)?;
        if manifest.schema_version != SCHEMA_VERSION || manifest.source.is_empty() {
            return Err(CanonicalError::Corrupt(
                "unsupported or empty source manifest",
            ));
        }
        validate_bbox(manifest.proof_bbox_wgs84)?;
        if manifest.proof_bbox_wgs84[0] == manifest.proof_bbox_wgs84[2]
            || manifest.proof_bbox_wgs84[1] == manifest.proof_bbox_wgs84[3]
        {
            return Err(CanonicalError::Corrupt("empty proof region"));
        }
        let mut ids = BTreeSet::new();
        for source in &mut manifest.source {
            if !Path::new(&source.file).is_absolute() {
                source.file = path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(&source.file)
                    .to_string_lossy()
                    .into_owned();
            }
            if let Some(upstream_file) = &mut source.upstream_file
                && !Path::new(upstream_file).is_absolute()
            {
                *upstream_file = path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(&*upstream_file)
                    .to_string_lossy()
                    .into_owned();
            }
            if let Some(dedup_file) = &mut source.dedup_file
                && !Path::new(dedup_file).is_absolute()
            {
                *dedup_file = path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(&*dedup_file)
                    .to_string_lossy()
                    .into_owned();
            }
            if source.id.is_empty() || !ids.insert(source.id.as_str()) {
                return Err(CanonicalError::Corrupt("duplicate or empty source ID"));
            }
            if !matches!(
                source.license_status.as_str(),
                "APPROVED" | "APPROVED_WITH_ATTRIBUTION"
            ) || !source.commercial_use
                || !source.modification
                || !source.redistribution
                || source.share_alike
                || (source.attribution_required
                    && (source.license_status != "APPROVED_WITH_ATTRIBUTION"
                        || source.attribution_text.as_deref().is_none_or(str::is_empty)))
            {
                return Err(CanonicalError::License(source.id.clone()));
            }
            if source.sha256.len() != 64
                || !source.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(CanonicalError::Corrupt("invalid source SHA-256"));
            }
            match (&source.upstream_file, &source.upstream_sha256) {
                (None, None) => {}
                (Some(_), Some(hash))
                    if hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()) => {}
                _ => return Err(CanonicalError::Corrupt("invalid upstream source SHA-256")),
            }
            match (&source.dedup_file, &source.dedup_sha256) {
                (None, None) if source.adapter != "os-open-roads" => {}
                (Some(_), Some(hash))
                    if hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()) => {}
                _ => return Err(CanonicalError::Corrupt("invalid deduplication index")),
            }
            if !matches!(
                source.adapter.as_str(),
                "naju-road-centerline"
                    | "naju-road-surface"
                    | "sgis-admin-district"
                    | "esa-worldcover-water"
                    | "esa-worldcover-tree"
                    | "microsoft-ml-building-footprints"
                    | "us-census-tiger-roads"
                    | "us-census-tiger-areawater"
                    | "us-census-tiger-arealm-parks"
                    | "ca-nrn-roadseg"
                    | "au-wa-road-network"
                    | "au-vic-dtp-managed-roads"
                    | "au-vic-vicmap-roads"
                    | "au-act-road-centrelines"
                    | "au-tas-list-transport-segments"
                    | "au-qld-qrt-roads"
                    | "ign-bdtopo-road-segment"
                    | "ign-bdtopo-surface-water"
                    | "ign-bdtopo-passenger-station"
                    | "lu-geobase-road"
                    | "os-open-roads"
            ) || (source.adapter == "os-open-roads" && source.adapter_version != 2)
                || (source.adapter != "os-open-roads" && source.adapter_version != 1)
            {
                return Err(CanonicalError::Corrupt("unsupported source adapter"));
            }
        }
        Ok(manifest)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BBox {
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
}

impl BBox {
    fn intersects(self, other: Self) -> bool {
        self.west <= other.east
            && self.east >= other.west
            && self.south <= other.north
            && self.north >= other.south
    }
}

fn validate_bbox(raw: [f64; 4]) -> Result<BBox, CanonicalError> {
    let [west, south, east, north] = raw;
    if !raw.iter().all(|v| v.is_finite())
        || west < -180.0
        || east > 180.0
        || south < -85.051_128_78
        || north > 85.051_128_78
        || west > east
        || south > north
    {
        return Err(CanonicalError::Corrupt("invalid WGS84 proof bounds"));
    }
    Ok(BBox {
        west,
        south,
        east,
        north,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeatureKind {
    RoadPrimary,
    RoadSecondary,
    RoadResidential,
    RoadSurface,
    Building,
    Water,
    Park,
    Rail,
    Place,
    PlaceDistrict,
    Vegetation,
    PlaceStation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Geometry {
    Point([f64; 2]),
    Line(Vec<[f64; 2]>),
    Polygon(Vec<Vec<[f64; 2]>>),
}

impl Geometry {
    fn bbox(&self) -> Result<BBox, CanonicalError> {
        let coordinates: Vec<[f64; 2]> = match self {
            Self::Point(point) => vec![*point],
            Self::Line(line) => line.clone(),
            Self::Polygon(rings) => rings.iter().flatten().copied().collect(),
        };
        let mut bounds = BBox {
            west: f64::INFINITY,
            south: f64::INFINITY,
            east: f64::NEG_INFINITY,
            north: f64::NEG_INFINITY,
        };
        for [lon, lat] in &coordinates {
            if !lon.is_finite()
                || !lat.is_finite()
                || !(-180.0..180.0).contains(lon)
                || !(-85.051_128_78..=85.051_128_78).contains(lat)
            {
                return Err(CanonicalError::Feature(
                    "coordinate outside map range".into(),
                ));
            }
            bounds.west = bounds.west.min(*lon);
            bounds.east = bounds.east.max(*lon);
            bounds.south = bounds.south.min(*lat);
            bounds.north = bounds.north.max(*lat);
        }
        if coordinates.is_empty() || bounds.east - bounds.west >= 180.0 {
            return Err(CanonicalError::Feature(
                "empty or antimeridian geometry".into(),
            ));
        }
        match self {
            Self::Point(_) => {}
            Self::Line(line) => {
                if line.len() < 2 || line.windows(2).any(|pair| pair[0] == pair[1]) {
                    return Err(CanonicalError::Feature("degenerate line".into()));
                }
            }
            Self::Polygon(rings) => {
                if rings.is_empty()
                    || rings
                        .iter()
                        .any(|ring| ring.len() < 4 || ring.first() != ring.last())
                {
                    return Err(CanonicalError::Feature("unclosed polygon ring".into()));
                }
                let to_line = |ring: &Vec<[f64; 2]>| -> LineString<f64> {
                    ring.iter().map(|p| Coord { x: p[0], y: p[1] }).collect()
                };
                let polygon =
                    Polygon::new(to_line(&rings[0]), rings[1..].iter().map(to_line).collect());
                polygon
                    .check_validation()
                    .map_err(|_| CanonicalError::Feature("invalid polygon topology".into()))?;
            }
        }
        Ok(bounds)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CanonicalFeature {
    pub id: u128,
    pub kind: FeatureKind,
    pub geometry: Geometry,
    pub bbox: BBox,
    pub importance: u16,
    pub min_zoom: u8,
    pub max_zoom: u8,
    pub name: Option<String>,
    pub revision: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    pub feature_id: u128,
    pub source_id: String,
    pub source_feature_id: String,
    pub source_revision: String,
    pub adapter_version: u32,
    pub source_feature_sha256: [u8; 32],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RejectedFeature {
    pub source_id: String,
    pub source_feature_id: String,
    pub reason: String,
}

pub type AdaptedFeatures = Vec<(CanonicalFeature, Provenance)>;

fn source_hash(path: &Path) -> Result<String, CanonicalError> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn stable_id(source_id: &str, feature_id: &str) -> u128 {
    let mut hash = Sha256::new();
    hash.update(source_id.as_bytes());
    hash.update([0]);
    hash.update(feature_id.as_bytes());
    let digest = hash.finalize();
    u128::from_be_bytes(digest[..16].try_into().expect("fixed SHA-256 prefix"))
}

/// Microsoft's official partition files are gzip-compressed GeoJSON Features,
/// one per line. A raw record digest supplies the ID because the feed has no
/// feature ID. No footprint is repaired or synthesized here.
pub fn adapt_microsoft_buildings(
    source: &SourceRecord,
    region: BBox,
) -> Result<(AdaptedFeatures, Vec<RejectedFeature>), CanonicalError> {
    if source.adapter != "microsoft-ml-building-footprints" {
        return Err(CanonicalError::Feature("wrong building adapter".into()));
    }
    if source_hash(Path::new(&source.file))? != source.sha256.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(source.id.clone()));
    }
    let reader = BufReader::new(GzDecoder::new(File::open(&source.file)?));
    let mut output = Vec::new();
    let mut rejected = Vec::new();
    let mut raw_ids = BTreeSet::new();
    for (line_number, line) in reader.lines().enumerate() {
        let line = line?;
        let digest: [u8; 32] = Sha256::digest(line.as_bytes()).into();
        let source_feature_id: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        let raw = match GeoJson::from_str(&line) {
            Ok(GeoJson::Feature(raw)) => raw,
            _ => {
                rejected.push(RejectedFeature {
                    source_id: source.id.clone(),
                    source_feature_id,
                    reason: format!("line {}: expected GeoJSON Feature", line_number + 1),
                });
                continue;
            }
        };
        let Some(geojson::Geometry {
            value: GeometryValue::Polygon { coordinates },
            ..
        }) = raw.geometry.as_ref()
        else {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id,
                reason: "expected building Polygon".into(),
            });
            continue;
        };
        let rings = coordinates
            .iter()
            .map(|ring| {
                ring.iter()
                    .map(|position| {
                        let [lon, lat] = position.as_slice() else {
                            return Err(CanonicalError::Feature(
                                "invalid building coordinate".into(),
                            ));
                        };
                        Ok([*lon, *lat])
                    })
                    .collect::<Result<Vec<_>, CanonicalError>>()
            })
            .collect::<Result<Vec<_>, CanonicalError>>();
        let rings = match rings {
            Ok(rings) => rings,
            Err(error) => {
                rejected.push(RejectedFeature {
                    source_id: source.id.clone(),
                    source_feature_id,
                    reason: error.to_string(),
                });
                continue;
            }
        };
        let geometry = Geometry::Polygon(rings);
        let bbox = match geometry.bbox() {
            Ok(bbox) => bbox,
            Err(error) => {
                rejected.push(RejectedFeature {
                    source_id: source.id.clone(),
                    source_feature_id,
                    reason: error.to_string(),
                });
                continue;
            }
        };
        if !bbox.intersects(region) {
            continue;
        }
        if !raw_ids.insert(source_feature_id.clone()) {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id,
                reason: "duplicate source feature within proof region".into(),
            });
            continue;
        }
        let id = stable_id(&source.id, &source_feature_id);
        output.push((
            CanonicalFeature {
                id,
                kind: FeatureKind::Building,
                geometry,
                bbox,
                importance: 100,
                min_zoom: 14,
                max_zoom: 15,
                name: None,
                revision: 1,
            },
            Provenance {
                feature_id: id,
                source_id: source.id.clone(),
                source_feature_id,
                source_revision: source.source_version.clone(),
                adapter_version: source.adapter_version,
                source_feature_sha256: digest,
            },
        ));
    }
    output.sort_by_key(|(feature, _)| feature.id);
    Ok((output, rejected))
}

fn zip_member(
    archive: &mut zip::ZipArchive<File>,
    suffix: &str,
) -> Result<Vec<u8>, CanonicalError> {
    let matches = archive
        .file_names()
        .filter(|name| name.ends_with(suffix))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let [name] = matches.as_slice() else {
        return Err(CanonicalError::Feature(format!(
            "expected exactly one {suffix} in source ZIP"
        )));
    };
    let mut member = archive
        .by_name(name)
        .map_err(|error| CanonicalError::Feature(error.to_string()))?;
    if member.size() > 64 * 1024 * 1024 {
        return Err(CanonicalError::Feature(format!("{suffix} exceeds 64 MiB")));
    }
    let mut bytes = Vec::with_capacity(member.size() as usize);
    member.read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// Read an official county All Roads ZIP directly in Rust. TIGER/Line's NAD83
/// coordinates are retained numerically; independent WGS84 datum accuracy is
/// an explicit pending gate, not a precision claim.
pub fn adapt_us_census_roads(
    source: &SourceRecord,
    region: BBox,
) -> Result<(AdaptedFeatures, Vec<RejectedFeature>), CanonicalError> {
    if source.adapter != "us-census-tiger-roads" {
        return Err(CanonicalError::Feature("wrong TIGER road adapter".into()));
    }
    if source_hash(Path::new(&source.file))? != source.sha256.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(source.id.clone()));
    }
    let mut archive = zip::ZipArchive::new(File::open(&source.file)?)
        .map_err(|error| CanonicalError::Feature(error.to_string()))?;
    let prj = zip_member(&mut archive, ".prj")?;
    if !std::str::from_utf8(&prj).is_ok_and(|text| text.contains("GCS_North_American_1983")) {
        return Err(CanonicalError::Feature(
            "unexpected TIGER source CRS".into(),
        ));
    }
    let shp = zip_member(&mut archive, ".shp")?;
    let dbf = zip_member(&mut archive, ".dbf")?;
    let shape_reader = shapefile::ShapeReader::new(std::io::Cursor::new(shp))?;
    let attribute_reader = shapefile::dbase::Reader::new(std::io::Cursor::new(dbf))?;
    let mut reader = shapefile::Reader::new(shape_reader, attribute_reader);
    let mut output = Vec::new();
    let mut rejected = Vec::new();
    let mut raw_ids = BTreeSet::new();
    for (record_index, record) in reader.iter_shapes_and_records().enumerate() {
        let (shape, attributes) = record?;
        let string_field = |field: &str| -> Option<&str> {
            match attributes.get(field) {
                Some(shapefile::dbase::FieldValue::Character(Some(value))) => Some(value.trim()),
                _ => None,
            }
        };
        let source_feature_id = string_field("LINEARID")
            .filter(|id| !id.is_empty())
            .ok_or_else(|| CanonicalError::Feature("missing TIGER LINEARID".into()))?
            .to_owned();
        if !raw_ids.insert(source_feature_id.clone()) {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id,
                reason: "duplicate LINEARID in county All Roads".into(),
            });
            continue;
        }
        let (kind, importance, min_zoom) = match string_field("MTFCC") {
            Some("S1100") => (FeatureKind::RoadPrimary, 700, 10),
            Some("S1200") => (FeatureKind::RoadSecondary, 500, 11),
            Some("S1400" | "S1630" | "S1640") => (FeatureKind::RoadResidential, 200, 12),
            class => {
                rejected.push(RejectedFeature {
                    source_id: source.id.clone(),
                    source_feature_id,
                    reason: format!("road/path class excluded: {class:?}"),
                });
                continue;
            }
        };
        let name = string_field("FULLNAME")
            .filter(|name| !name.is_empty() && name.len() <= 128)
            .map(str::to_owned);
        let shapefile::Shape::Polyline(line) = shape else {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id,
                reason: format!("row {}: expected Polyline", record_index + 1),
            });
            continue;
        };
        for (part_index, part) in line.parts().iter().enumerate() {
            let part_id = format!("{source_feature_id}:{part_index}");
            let geometry = Geometry::Line(part.iter().map(|point| [point.x, point.y]).collect());
            let bbox = match geometry.bbox() {
                Ok(bbox) => bbox,
                Err(error) => {
                    rejected.push(RejectedFeature {
                        source_id: source.id.clone(),
                        source_feature_id: part_id,
                        reason: error.to_string(),
                    });
                    continue;
                }
            };
            if !bbox.intersects(region) {
                continue;
            }
            let id = stable_id(&source.id, &part_id);
            let mut hash = Sha256::new();
            hash.update(part_id.as_bytes());
            hash.update(string_field("MTFCC").unwrap_or_default().as_bytes());
            for point in part {
                hash.update(point.x.to_le_bytes());
                hash.update(point.y.to_le_bytes());
            }
            output.push((
                CanonicalFeature {
                    id,
                    kind,
                    geometry,
                    bbox,
                    importance,
                    min_zoom,
                    max_zoom: 15,
                    name: name.clone(),
                    revision: 1,
                },
                Provenance {
                    feature_id: id,
                    source_id: source.id.clone(),
                    source_feature_id: part_id,
                    source_revision: source.source_version.clone(),
                    adapter_version: source.adapter_version,
                    source_feature_sha256: hash.finalize().into(),
                },
            ));
        }
    }
    output.sort_by_key(|(feature, _)| feature.id);
    Ok((output, rejected))
}

/// Build-time OS Open Roads adapter. A selected 100 km source square is read
/// from the official national archive; OSTN15 converts its EPSG:27700 X/Y to
/// longitude/latitude. Z is not used by the planar map renderer.
#[cfg(feature = "gb-roads")]
pub fn adapt_os_open_roads_grid(
    source: &SourceRecord,
    grid: &str,
) -> Result<(AdaptedFeatures, Vec<RejectedFeature>, usize), CanonicalError> {
    if source.adapter != "os-open-roads"
        || grid.len() != 2
        || !grid.bytes().all(|byte| byte.is_ascii_uppercase())
    {
        return Err(CanonicalError::Feature(
            "invalid OS Open Roads adapter or grid".into(),
        ));
    }
    if source_hash(Path::new(&source.file))? != source.sha256.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(source.id.clone()));
    }
    let dedup_path = source
        .dedup_file
        .as_deref()
        .ok_or_else(|| CanonicalError::Feature("OS duplicate owner index is required".into()))?;
    if source_hash(Path::new(dedup_path))?
        != source
            .dedup_sha256
            .as_deref()
            .ok_or_else(|| CanonicalError::Feature("OS duplicate index SHA-256 missing".into()))?
    {
        return Err(CanonicalError::SourceChecksum(dedup_path.into()));
    }
    let index_text = fs::read_to_string(dedup_path)?;
    let mut index_lines = index_text.lines();
    if index_lines.next() != Some(format!("# source_sha256={}", source.sha256).as_str())
        || index_lines.next() != Some("identifier\towner_grid\tduplicate_grid")
    {
        return Err(CanonicalError::Feature(
            "OS duplicate index does not match source ZIP".into(),
        ));
    }
    let mut duplicate_ids = BTreeSet::new();
    for line in index_lines {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 3 || fields[0].is_empty() || fields[1] >= fields[2] {
            return Err(CanonicalError::Feature(
                "invalid OS duplicate index row".into(),
            ));
        }
        if fields[2] == grid && !duplicate_ids.insert(fields[0].to_owned()) {
            return Err(CanonicalError::Feature(
                "repeated OS duplicate index ID".into(),
            ));
        }
    }
    let expected_duplicates = duplicate_ids.len();
    let mut archive = zip::ZipArchive::new(File::open(&source.file)?)
        .map_err(|error| CanonicalError::Feature(error.to_string()))?;
    let stem = format!("data/{grid}_RoadLink");
    let read_member = |archive: &mut zip::ZipArchive<File>, suffix: &str| {
        let mut member = archive
            .by_name(&format!("{stem}{suffix}"))
            .map_err(|error| CanonicalError::Feature(error.to_string()))?;
        if member.size() > 512 * 1024 * 1024 {
            return Err(CanonicalError::Feature("OS member exceeds 512 MiB".into()));
        }
        let mut bytes = Vec::with_capacity(member.size() as usize);
        member.read_to_end(&mut bytes)?;
        Ok::<_, CanonicalError>(bytes)
    };
    let prj = read_member(&mut archive, ".prj")?;
    if !std::str::from_utf8(&prj).is_ok_and(|text| {
        text.contains("British_National_Grid") && text.contains("AUTHORITY[\"EPSG\",27700]")
    }) {
        return Err(CanonicalError::Feature(
            "unexpected OS Open Roads CRS".into(),
        ));
    }
    let shp = read_member(&mut archive, ".shp")?;
    let dbf = read_member(&mut archive, ".dbf")?;
    let shape_reader = shapefile::ShapeReader::new(std::io::Cursor::new(shp))?;
    let attribute_reader = shapefile::dbase::Reader::new(std::io::Cursor::new(dbf))?;
    let mut reader = shapefile::Reader::new(shape_reader, attribute_reader);
    let mut output = Vec::new();
    let mut rejected = Vec::new();
    let mut raw_ids = BTreeSet::new();
    for (record_index, record) in reader.iter_shapes_and_records().enumerate() {
        let (shape, attributes) = record?;
        let string_field = |field: &str| -> Option<&str> {
            match attributes.get(field) {
                Some(shapefile::dbase::FieldValue::Character(Some(value))) => Some(value.trim()),
                _ => None,
            }
        };
        let source_feature_id = string_field("identifier")
            .filter(|id| !id.is_empty())
            .ok_or_else(|| CanonicalError::Feature("missing OS RoadLink identifier".into()))?
            .to_owned();
        if duplicate_ids.remove(&source_feature_id) {
            continue;
        }
        if !raw_ids.insert(source_feature_id.clone()) {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id,
                reason: "duplicate identifier in OS grid".into(),
            });
            continue;
        }
        let (kind, importance, min_zoom) = match string_field("function") {
            Some("Motorway") => (FeatureKind::RoadPrimary, 750, 10),
            Some("A Road") => (FeatureKind::RoadPrimary, 700, 10),
            Some("B Road" | "Minor Road") => (FeatureKind::RoadSecondary, 500, 11),
            Some(
                "Local Road"
                | "Local Access Road"
                | "Restricted Local Access Road"
                | "Secondary Access Road",
            ) => (FeatureKind::RoadResidential, 200, 12),
            value => {
                rejected.push(RejectedFeature {
                    source_id: source.id.clone(),
                    source_feature_id,
                    reason: format!("unmapped OS road function: {value:?}"),
                });
                continue;
            }
        };
        let name = string_field("name1")
            .filter(|name| !name.is_empty() && name.len() <= 128)
            .map(str::to_owned);
        let shapefile::Shape::PolylineZ(line) = shape else {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id,
                reason: format!("row {}: expected PolylineZ", record_index + 1),
            });
            continue;
        };
        for (part_index, part) in line.parts().iter().enumerate() {
            let part_id = format!("{source_feature_id}:{part_index}");
            let mut points = Vec::with_capacity(part.len());
            let mut hash = Sha256::new();
            hash.update(part_id.as_bytes());
            hash.update(string_field("function").unwrap_or_default().as_bytes());
            let mut transform_error = None;
            for point in part {
                hash.update(point.x.to_le_bytes());
                hash.update(point.y.to_le_bytes());
                hash.update(point.z.to_le_bytes());
                match lonlat_bng::convert_osgb36_to_ll(point.x, point.y) {
                    Ok((lon, lat)) => points.push([lon, lat]),
                    Err(error) => {
                        transform_error = Some(error.to_string());
                        break;
                    }
                }
            }
            if let Some(error) = transform_error {
                rejected.push(RejectedFeature {
                    source_id: source.id.clone(),
                    source_feature_id: part_id,
                    reason: format!("OSTN15 transform: {error}"),
                });
                continue;
            }
            let geometry = Geometry::Line(points);
            let bbox = match geometry.bbox() {
                Ok(bbox) => bbox,
                Err(error) => {
                    rejected.push(RejectedFeature {
                        source_id: source.id.clone(),
                        source_feature_id: part_id,
                        reason: error.to_string(),
                    });
                    continue;
                }
            };
            let id = stable_id(&source.id, &part_id);
            output.push((
                CanonicalFeature {
                    id,
                    kind,
                    geometry,
                    bbox,
                    importance,
                    min_zoom,
                    max_zoom: 15,
                    name: name.clone(),
                    revision: 1,
                },
                Provenance {
                    feature_id: id,
                    source_id: source.id.clone(),
                    source_feature_id: part_id,
                    source_revision: source.source_version.clone(),
                    adapter_version: source.adapter_version,
                    source_feature_sha256: hash.finalize().into(),
                },
            ));
        }
    }
    if !duplicate_ids.is_empty() {
        return Err(CanonicalError::Feature(
            "OS duplicate index names IDs absent from selected grid".into(),
        ));
    }
    output.sort_by_key(|(feature, _)| feature.id);
    Ok((output, rejected, expected_duplicates))
}

/// Read selected polygon classes from an official TIGER/Line ZIP. Numeric NAD83
/// degrees are kept as published; no WGS84 accuracy is inferred.
fn adapt_us_census_polygons(
    source: &SourceRecord,
    region: BBox,
    adapter: &str,
    id_field: &str,
    kind: FeatureKind,
    min_zoom: u8,
    accepts: fn(&str) -> bool,
) -> Result<(AdaptedFeatures, Vec<RejectedFeature>), CanonicalError> {
    if source.adapter != adapter {
        return Err(CanonicalError::Feature(
            "wrong TIGER polygon adapter".into(),
        ));
    }
    if source_hash(Path::new(&source.file))? != source.sha256.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(source.id.clone()));
    }
    let mut archive = zip::ZipArchive::new(File::open(&source.file)?)
        .map_err(|error| CanonicalError::Feature(error.to_string()))?;
    let prj = zip_member(&mut archive, ".prj")?;
    if !std::str::from_utf8(&prj).is_ok_and(|text| text.contains("GCS_North_American_1983")) {
        return Err(CanonicalError::Feature(
            "unexpected TIGER polygon CRS".into(),
        ));
    }
    let shp = zip_member(&mut archive, ".shp")?;
    let dbf = zip_member(&mut archive, ".dbf")?;
    let shape_reader = shapefile::ShapeReader::new(std::io::Cursor::new(shp))?;
    let attribute_reader = shapefile::dbase::Reader::new(std::io::Cursor::new(dbf))?;
    let mut reader = shapefile::Reader::new(shape_reader, attribute_reader);
    let mut output = Vec::new();
    let mut rejected = Vec::new();
    let mut raw_ids = BTreeSet::new();
    for (record_index, record) in reader.iter_shapes_and_records().enumerate() {
        let (shape, attributes) = record?;
        let string_field = |field: &str| -> Option<&str> {
            match attributes.get(field) {
                Some(shapefile::dbase::FieldValue::Character(Some(value))) => Some(value.trim()),
                _ => None,
            }
        };
        let raw_id = string_field(id_field)
            .filter(|id| !id.is_empty())
            .ok_or_else(|| CanonicalError::Feature(format!("missing TIGER {id_field}")))?
            .to_owned();
        if !raw_ids.insert(raw_id.clone()) {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id: raw_id,
                reason: format!("duplicate {id_field} in TIGER polygon source"),
            });
            continue;
        }
        if !accepts(string_field("MTFCC").unwrap_or_default()) {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id: raw_id,
                reason: format!("excluded MTFCC {:?}", string_field("MTFCC")),
            });
            continue;
        }
        let shapefile::Shape::Polygon(polygon) = shape else {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id: raw_id,
                reason: format!("row {}: expected Polygon", record_index + 1),
            });
            continue;
        };
        let mut groups: Vec<Vec<Vec<[f64; 2]>>> = Vec::new();
        let mut inner = Vec::new();
        for ring in polygon.rings() {
            let points: Vec<[f64; 2]> = ring.points().iter().map(|p| [p.x, p.y]).collect();
            match ring {
                shapefile::PolygonRing::Outer(_) => groups.push(vec![points]),
                shapefile::PolygonRing::Inner(_) => inner.push(points),
            }
        }
        let mut bad_hole = false;
        for hole in inner {
            let Some(first) = hole.first() else {
                bad_hole = true;
                break;
            };
            let point = Point::new(first[0], first[1]);
            let containing = groups
                .iter()
                .enumerate()
                .filter(|(_, rings)| {
                    let exterior: LineString<f64> = rings[0]
                        .iter()
                        .map(|p| Coord { x: p[0], y: p[1] })
                        .collect();
                    Polygon::new(exterior, vec![]).contains(&point)
                })
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            if let [index] = containing.as_slice() {
                groups[*index].push(hole);
            } else {
                bad_hole = true;
                break;
            }
        }
        if bad_hole || groups.is_empty() {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id: raw_id,
                reason: "polygon has unpaired or ambiguous rings".into(),
            });
            continue;
        }
        for (part_index, rings) in groups.into_iter().enumerate() {
            let part_id = format!("{raw_id}:{part_index}");
            let geometry = Geometry::Polygon(rings);
            let bbox = match geometry.bbox() {
                Ok(bbox) => bbox,
                Err(error) => {
                    rejected.push(RejectedFeature {
                        source_id: source.id.clone(),
                        source_feature_id: part_id,
                        reason: error.to_string(),
                    });
                    continue;
                }
            };
            if !bbox.intersects(region) {
                continue;
            }
            let id = stable_id(&source.id, &part_id);
            let mut hasher = Sha256::new();
            hasher.update(part_id.as_bytes());
            hasher.update(string_field("MTFCC").unwrap_or_default().as_bytes());
            if let Geometry::Polygon(rings) = &geometry {
                for ring in rings {
                    for [lon, lat] in ring {
                        hasher.update(lon.to_le_bytes());
                        hasher.update(lat.to_le_bytes());
                    }
                }
            }
            output.push((
                CanonicalFeature {
                    id,
                    kind,
                    geometry,
                    bbox,
                    importance: 400,
                    min_zoom,
                    max_zoom: 15,
                    name: string_field("FULLNAME")
                        .filter(|name| !name.is_empty() && name.len() <= 128)
                        .map(str::to_owned),
                    revision: 1,
                },
                Provenance {
                    feature_id: id,
                    source_id: source.id.clone(),
                    source_feature_id: part_id,
                    source_revision: source.source_version.clone(),
                    adapter_version: source.adapter_version,
                    source_feature_sha256: hasher.finalize().into(),
                },
            ));
        }
    }
    output.sort_by_key(|(feature, _)| feature.id);
    Ok((output, rejected))
}

/// County Area Hydrography polygons.
pub fn adapt_us_census_areawater(
    source: &SourceRecord,
    region: BBox,
) -> Result<(AdaptedFeatures, Vec<RejectedFeature>), CanonicalError> {
    adapt_us_census_polygons(
        source,
        region,
        "us-census-tiger-areawater",
        "HYDROID",
        FeatureKind::Water,
        10,
        |_| true,
    )
}

/// Parks and recreation areas from state Area Landmark polygons. These are
/// administrative park footprints, not satellite-derived tree cover.
pub fn adapt_us_census_parks(
    source: &SourceRecord,
    region: BBox,
) -> Result<(AdaptedFeatures, Vec<RejectedFeature>), CanonicalError> {
    adapt_us_census_polygons(
        source,
        region,
        "us-census-tiger-arealm-parks",
        "AREAID",
        FeatureKind::Park,
        12,
        |class| {
            class
                .strip_prefix('K')
                .and_then(|value| value.parse::<u16>().ok())
                .is_some_and(|code| (2180..=2190).contains(&code))
        },
    )
}

pub fn adapt_naju_roads(
    source: &SourceRecord,
    region: BBox,
) -> Result<AdaptedFeatures, CanonicalError> {
    if source_hash(Path::new(&source.file))? != source.sha256.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(source.id.clone()));
    }
    let GeoJson::FeatureCollection(collection) =
        GeoJson::from_str(&fs::read_to_string(&source.file)?)?
    else {
        return Err(CanonicalError::Feature("expected FeatureCollection".into()));
    };
    let mut output = Vec::new();
    let mut raw_ids = BTreeSet::new();
    for raw in collection.features {
        let source_feature_id = raw
            .property("gid")
            .and_then(|value| value.as_i64())
            .ok_or_else(|| CanonicalError::Feature("missing government road gid".into()))?
            .to_string();
        if !raw_ids.insert(source_feature_id.clone()) {
            return Err(CanonicalError::Feature(format!(
                "duplicate gid {source_feature_id}"
            )));
        }
        let width = raw
            .property("rdl_wid")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let Some(geojson::Geometry {
            value: GeometryValue::LineString { coordinates },
            ..
        }) = raw.geometry.as_ref()
        else {
            return Err(CanonicalError::Feature(format!(
                "gid {source_feature_id}: expected LineString"
            )));
        };
        let points = coordinates
            .iter()
            .map(|position| {
                let [lon, lat] = position.as_slice() else {
                    return Err(CanonicalError::Feature(format!(
                        "gid {source_feature_id}: invalid coordinate"
                    )));
                };
                Ok([*lon, *lat])
            })
            .collect::<Result<Vec<_>, CanonicalError>>()?;
        let geometry = Geometry::Line(points);
        let bbox = geometry.bbox().map_err(|error| {
            CanonicalError::Feature(format!("gid {source_feature_id}: {error}"))
        })?;
        if !bbox.intersects(region) {
            continue;
        }
        let id = stable_id(&source.id, &source_feature_id);
        let (kind, importance, min_zoom) = if width >= 20 {
            (FeatureKind::RoadPrimary, 700, 10)
        } else if width >= 10 {
            (FeatureKind::RoadSecondary, 500, 11)
        } else {
            (FeatureKind::RoadResidential, 200, 12)
        };
        let mut feature_hash = Sha256::new();
        feature_hash.update(
            serde_json::to_vec(&raw)
                .map_err(|_| CanonicalError::Corrupt("source serialization"))?,
        );
        output.push((
            CanonicalFeature {
                id,
                kind,
                geometry,
                bbox,
                importance,
                min_zoom,
                max_zoom: 15,
                name: None,
                revision: 1,
            },
            Provenance {
                feature_id: id,
                source_id: source.id.clone(),
                source_feature_id,
                source_revision: source.source_version.clone(),
                adapter_version: source.adapter_version,
                source_feature_sha256: feature_hash.finalize().into(),
            },
        ));
    }
    output.sort_by_key(|(feature, _)| feature.id);
    Ok(output)
}

/// Import the separate road-area layer from the same government ZIP. Invalid
/// polygons are reported for audit and never silently repaired.
pub fn adapt_naju_road_surfaces(
    source: &SourceRecord,
    region: BBox,
) -> Result<(AdaptedFeatures, Vec<RejectedFeature>), CanonicalError> {
    if source_hash(Path::new(&source.file))? != source.sha256.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(source.id.clone()));
    }
    let GeoJson::FeatureCollection(collection) =
        GeoJson::from_str(&fs::read_to_string(&source.file)?)?
    else {
        return Err(CanonicalError::Feature("expected FeatureCollection".into()));
    };
    let mut output = Vec::new();
    let mut rejected = Vec::new();
    let mut raw_ids = BTreeSet::new();
    for raw in collection.features {
        let source_feature_id = raw
            .property("gid")
            .and_then(|value| value.as_i64())
            .ok_or_else(|| CanonicalError::Feature("missing road-surface gid".into()))?
            .to_string();
        if !raw_ids.insert(source_feature_id.clone()) {
            return Err(CanonicalError::Feature(format!(
                "duplicate surface gid {source_feature_id}"
            )));
        }
        let Some(raw_geometry) = raw.geometry.as_ref() else {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id,
                reason: "missing polygon geometry".into(),
            });
            continue;
        };
        let parts: Vec<_> = match &raw_geometry.value {
            GeometryValue::Polygon { coordinates } => vec![coordinates],
            GeometryValue::MultiPolygon { coordinates } => coordinates.iter().collect(),
            _ => {
                rejected.push(RejectedFeature {
                    source_id: source.id.clone(),
                    source_feature_id,
                    reason: "expected Polygon or MultiPolygon".into(),
                });
                continue;
            }
        };
        let mut feature_hash = Sha256::new();
        feature_hash.update(
            serde_json::to_vec(&raw)
                .map_err(|_| CanonicalError::Corrupt("source serialization"))?,
        );
        let source_feature_sha256: [u8; 32] = feature_hash.finalize().into();
        for (part_index, part) in parts.iter().enumerate() {
            let part_id = if parts.len() == 1 {
                source_feature_id.clone()
            } else {
                format!("{source_feature_id}:{part_index}")
            };
            let rings = part
                .iter()
                .map(|ring| {
                    ring.iter()
                        .map(|position| {
                            let [lon, lat] = position.as_slice() else {
                                return Err(CanonicalError::Feature(format!(
                                    "surface {part_id}: invalid coordinate"
                                )));
                            };
                            Ok([*lon, *lat])
                        })
                        .collect::<Result<Vec<_>, CanonicalError>>()
                })
                .collect::<Result<Vec<_>, CanonicalError>>()?;
            let geometry = Geometry::Polygon(rings);
            let bbox = match geometry.bbox() {
                Ok(bbox) => bbox,
                Err(error) => {
                    rejected.push(RejectedFeature {
                        source_id: source.id.clone(),
                        source_feature_id: part_id,
                        reason: error.to_string(),
                    });
                    continue;
                }
            };
            if !bbox.intersects(region) {
                continue;
            }
            let id = stable_id(&source.id, &part_id);
            output.push((
                CanonicalFeature {
                    id,
                    kind: FeatureKind::RoadSurface,
                    geometry,
                    bbox,
                    importance: 100,
                    min_zoom: 13,
                    max_zoom: 15,
                    name: None,
                    revision: 1,
                },
                Provenance {
                    feature_id: id,
                    source_id: source.id.clone(),
                    source_feature_id: part_id,
                    source_revision: source.source_version.clone(),
                    adapter_version: source.adapter_version,
                    source_feature_sha256,
                },
            ));
        }
    }
    output.sort_by_key(|(feature, _)| feature.id);
    rejected.sort_by(|a, b| a.source_feature_id.cmp(&b.source_feature_id));
    Ok((output, rejected))
}

/// The source preparation step polygonizes a single ESA 10 m class mask.
/// Keep water and tree cover distinct: tree cover is not a surveyed park.
pub fn adapt_worldcover_polygons(
    source: &SourceRecord,
    region: BBox,
) -> Result<(AdaptedFeatures, Vec<RejectedFeature>), CanonicalError> {
    let (kind, min_zoom) = match source.adapter.as_str() {
        "esa-worldcover-water" => (FeatureKind::Water, 10),
        "esa-worldcover-tree" => (FeatureKind::Vegetation, 14),
        _ => {
            return Err(CanonicalError::Feature(
                "unexpected WorldCover adapter".into(),
            ));
        }
    };
    if source_hash(Path::new(&source.file))? != source.sha256.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(source.id.clone()));
    }
    let (Some(upstream_file), Some(upstream_sha256)) =
        (&source.upstream_file, &source.upstream_sha256)
    else {
        return Err(CanonicalError::Corrupt(
            "WorldCover crop provenance missing",
        ));
    };
    if source_hash(Path::new(upstream_file))? != upstream_sha256.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(format!(
            "{} upstream crop",
            source.id
        )));
    }
    let GeoJson::FeatureCollection(collection) =
        GeoJson::from_str(&fs::read_to_string(&source.file)?)?
    else {
        return Err(CanonicalError::Feature("expected FeatureCollection".into()));
    };
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    let mut ids = BTreeSet::new();
    for raw in collection.features {
        if raw.property("DN").and_then(|value| value.as_i64()) != Some(1) {
            return Err(CanonicalError::Feature(
                "unexpected WorldCover mask class".into(),
            ));
        }
        let bytes = serde_json::to_vec(&raw)
            .map_err(|_| CanonicalError::Corrupt("source serialization"))?;
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        let source_feature_id = format!("{:x}", Sha256::digest(&bytes));
        if !ids.insert(source_feature_id.clone()) {
            return Err(CanonicalError::Feature(
                "duplicate WorldCover polygon".into(),
            ));
        }
        let Some(geojson::Geometry {
            value: GeometryValue::Polygon { coordinates },
            ..
        }) = raw.geometry.as_ref()
        else {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id,
                reason: "expected Polygon".into(),
            });
            continue;
        };
        let rings = coordinates
            .iter()
            .map(|ring| {
                ring.iter()
                    .map(|position| {
                        let [lon, lat] = position.as_slice() else {
                            return Err(CanonicalError::Feature(
                                "invalid WorldCover coordinate".into(),
                            ));
                        };
                        Ok([*lon, *lat])
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let geometry = Geometry::Polygon(rings);
        let bbox = match geometry.bbox() {
            Ok(bbox) => bbox,
            Err(error) => {
                rejected.push(RejectedFeature {
                    source_id: source.id.clone(),
                    source_feature_id,
                    reason: error.to_string(),
                });
                continue;
            }
        };
        if !bbox.intersects(region) {
            continue;
        }
        let id = stable_id(&source.id, &source_feature_id);
        accepted.push((
            CanonicalFeature {
                id,
                kind,
                geometry,
                bbox,
                importance: 100,
                min_zoom,
                max_zoom: 15,
                name: None,
                revision: 1,
            },
            Provenance {
                feature_id: id,
                source_id: source.id.clone(),
                source_feature_id,
                source_revision: source.source_version.clone(),
                adapter_version: source.adapter_version,
                source_feature_sha256: digest,
            },
        ));
    }
    accepted.sort_by_key(|(feature, _)| feature.id);
    Ok((accepted, rejected))
}

/// Locate an official district name inside the part of its boundary that falls
/// within the proof region. The source polygon is not retained as a road or land layer.
pub fn adapt_sgis_districts(
    source: &SourceRecord,
    region: BBox,
) -> Result<AdaptedFeatures, CanonicalError> {
    if source_hash(Path::new(&source.file))? != source.sha256.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(source.id.clone()));
    }
    let GeoJson::FeatureCollection(collection) =
        GeoJson::from_str(&fs::read_to_string(&source.file)?)?
    else {
        return Err(CanonicalError::Feature("expected FeatureCollection".into()));
    };
    let clip = Rect::new(
        Coord {
            x: region.west,
            y: region.south,
        },
        Coord {
            x: region.east,
            y: region.north,
        },
    )
    .to_polygon();
    let expected_date = source.source_version.replace('-', "");
    let mut output = Vec::new();
    let mut raw_ids = BTreeSet::new();
    for raw in collection.features {
        let source_feature_id = raw
            .property("ADM_CD")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| CanonicalError::Feature("missing SGIS ADM_CD".into()))?
            .to_owned();
        if !raw_ids.insert(source_feature_id.clone()) {
            return Err(CanonicalError::Feature(format!(
                "duplicate SGIS ADM_CD {source_feature_id}"
            )));
        }
        let name = raw
            .property("ADM_NM")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty() && value.len() <= 128)
            .ok_or_else(|| CanonicalError::Feature("missing SGIS ADM_NM".into()))?
            .to_owned();
        if raw.property("BASE_DATE").and_then(|value| value.as_str()) != Some(&expected_date) {
            return Err(CanonicalError::Feature(format!(
                "SGIS {source_feature_id}: source date mismatch"
            )));
        }
        let raw_geometry = raw
            .geometry
            .as_ref()
            .ok_or_else(|| CanonicalError::Feature("missing SGIS geometry".into()))?;
        let shape: geo::Geometry<f64> = (&raw_geometry.value)
            .try_into()
            .map_err(|_| CanonicalError::Feature("invalid SGIS polygon".into()))?;
        let polygons = match shape {
            geo::Geometry::Polygon(polygon) => MultiPolygon(vec![polygon]),
            geo::Geometry::MultiPolygon(polygons) => polygons,
            _ => return Err(CanonicalError::Feature("expected SGIS polygon".into())),
        };
        polygons.check_validation().map_err(|_| {
            CanonicalError::Feature(format!(
                "SGIS {source_feature_id}: invalid polygon topology"
            ))
        })?;
        let clipped = polygons.intersection(&clip);
        let Some(point) = clipped.interior_point() else {
            continue;
        };
        let geometry = Geometry::Point([point.x(), point.y()]);
        let bbox = geometry.bbox()?;
        let id = stable_id(&source.id, &source_feature_id);
        let mut feature_hash = Sha256::new();
        feature_hash.update(
            serde_json::to_vec(&raw)
                .map_err(|_| CanonicalError::Corrupt("source serialization"))?,
        );
        output.push((
            CanonicalFeature {
                id,
                kind: FeatureKind::PlaceDistrict,
                geometry,
                bbox,
                importance: 200,
                min_zoom: 12,
                max_zoom: 15,
                name: Some(name),
                revision: 1,
            },
            Provenance {
                feature_id: id,
                source_id: source.id.clone(),
                source_feature_id,
                source_revision: source.source_version.clone(),
                adapter_version: source.adapter_version,
                source_feature_sha256: feature_hash.finalize().into(),
            },
        ));
    }
    output.sort_by_key(|(feature, _)| feature.id);
    Ok(output)
}

#[derive(Clone, Copy)]
struct IndexEntry {
    id: u128,
    offset: u64,
    bbox: BBox,
}

impl RTreeObject for IndexEntry {
    type Envelope = AABB<[f64; 2]>;
    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners(
            [self.bbox.west, self.bbox.south],
            [self.bbox.east, self.bbox.north],
        )
    }
}

#[derive(Serialize, Deserialize)]
struct Meta {
    sources: Vec<String>,
    provenance: Vec<Provenance>,
}

fn write_u32(file: &mut File, value: u32) -> Result<(), CanonicalError> {
    Ok(file.write_all(&value.to_le_bytes())?)
}
fn write_u64(file: &mut File, value: u64) -> Result<(), CanonicalError> {
    Ok(file.write_all(&value.to_le_bytes())?)
}
fn read_u32(file: &mut File) -> Result<u32, CanonicalError> {
    let mut bytes = [0; 4];
    file.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}
fn read_u64(file: &mut File) -> Result<u64, CanonicalError> {
    let mut bytes = [0; 8];
    file.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

/// Write a seekable, versioned, checksummed build database. Its source lineage stays here,
/// outside runtime vector tiles.
pub fn write_geodb(
    output: &Path,
    records: &[(CanonicalFeature, Provenance)],
) -> Result<(), CanonicalError> {
    let count =
        u32::try_from(records.len()).map_err(|_| CanonicalError::Corrupt("too many features"))?;
    if count > MAX_FEATURES {
        return Err(CanonicalError::Corrupt("too many features"));
    }
    let temporary = output.with_extension("mgeodb.tmp");
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temporary)?;
    file.write_all(&[0; HEADER_BYTES as usize])?;
    let mut index = Vec::with_capacity(records.len());
    let mut ids = BTreeSet::new();
    for (feature, provenance) in records {
        if feature.id != provenance.feature_id
            || !ids.insert(feature.id)
            || feature.bbox != feature.geometry.bbox()?
        {
            return Err(CanonicalError::Corrupt(
                "duplicate ID, lineage, or invalid geometry",
            ));
        }
        let offset = file.stream_position()?;
        let encoded = bincode::serialize(feature)?;
        let size = u32::try_from(encoded.len())
            .map_err(|_| CanonicalError::Corrupt("record too large"))?;
        if size > MAX_RECORD_BYTES {
            return Err(CanonicalError::Corrupt("record too large"));
        }
        write_u32(&mut file, size)?;
        file.write_all(&encoded)?;
        index.push(IndexEntry {
            id: feature.id,
            offset,
            bbox: feature.bbox,
        });
    }
    let index_offset = file.stream_position()?;
    for entry in &index {
        file.write_all(&entry.id.to_le_bytes())?;
        write_u64(&mut file, entry.offset)?;
        for coordinate in [
            entry.bbox.west,
            entry.bbox.south,
            entry.bbox.east,
            entry.bbox.north,
        ] {
            file.write_all(&coordinate.to_le_bytes())?;
        }
    }
    let provenance_offset = file.stream_position()?;
    let meta = Meta {
        sources: records
            .iter()
            .map(|(_, p)| p.source_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        provenance: records.iter().map(|(_, p)| p.clone()).collect(),
    };
    let encoded_meta = bincode::serialize(&meta)?;
    write_u64(&mut file, encoded_meta.len() as u64)?;
    file.write_all(&encoded_meta)?;
    let footer_offset = file.stream_position()?;
    file.seek(SeekFrom::Start(0))?;
    file.write_all(MAGIC)?;
    write_u32(&mut file, SCHEMA_VERSION)?;
    write_u32(&mut file, count)?;
    write_u64(&mut file, index_offset)?;
    write_u64(&mut file, provenance_offset)?;
    write_u64(&mut file, footer_offset)?;
    file.flush()?;
    file.seek(SeekFrom::Start(0))?;
    let mut hasher = Sha256::new();
    let mut remaining = footer_offset;
    let mut buffer = [0u8; 64 * 1024];
    while remaining > 0 {
        let limit = remaining.min(buffer.len() as u64) as usize;
        let read = file.read(&mut buffer[..limit])?;
        if read == 0 {
            return Err(CanonicalError::Corrupt("truncated write"));
        }
        hasher.update(&buffer[..read]);
        remaining -= read as u64;
    }
    file.write_all(&hasher.finalize())?;
    file.flush()?;
    fs::rename(temporary, output)?;
    Ok(())
}

pub struct GeoDb {
    file: File,
    index: RTree<IndexEntry>,
    pub sources: Vec<String>,
    pub provenance: Vec<Provenance>,
}

impl GeoDb {
    pub fn open(path: &Path) -> Result<Self, CanonicalError> {
        let mut file = File::open(path)?;
        let length = file.metadata()?.len();
        if length < HEADER_BYTES + 32 {
            return Err(CanonicalError::Corrupt("short file"));
        }
        let mut magic = [0; 8];
        file.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(CanonicalError::Corrupt("bad magic"));
        }
        if read_u32(&mut file)? != SCHEMA_VERSION {
            return Err(CanonicalError::Corrupt("unsupported version"));
        }
        let count = read_u32(&mut file)?;
        if count > MAX_FEATURES {
            return Err(CanonicalError::Corrupt("feature count limit"));
        }
        let index_offset = read_u64(&mut file)?;
        let provenance_offset = read_u64(&mut file)?;
        let footer_offset = read_u64(&mut file)?;
        if index_offset < HEADER_BYTES
            || provenance_offset != index_offset + u64::from(count) * INDEX_BYTES
            || provenance_offset + 8 > footer_offset
            || footer_offset + 32 != length
        {
            return Err(CanonicalError::Corrupt("invalid section offsets"));
        }
        file.seek(SeekFrom::Start(0))?;
        let mut hasher = Sha256::new();
        let mut remaining = footer_offset;
        let mut buffer = [0u8; 64 * 1024];
        while remaining > 0 {
            let limit = remaining.min(buffer.len() as u64) as usize;
            let read = file.read(&mut buffer[..limit])?;
            if read == 0 {
                return Err(CanonicalError::Corrupt("truncated payload"));
            }
            hasher.update(&buffer[..read]);
            remaining -= read as u64;
        }
        let mut digest = [0; 32];
        file.read_exact(&mut digest)?;
        if hasher.finalize().as_slice() != digest {
            return Err(CanonicalError::Corrupt("checksum mismatch"));
        }
        file.seek(SeekFrom::Start(index_offset))?;
        let mut entries = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let mut id_bytes = [0; 16];
            file.read_exact(&mut id_bytes)?;
            let id = u128::from_le_bytes(id_bytes);
            let offset = read_u64(&mut file)?;
            let mut values = [0.0; 4];
            for value in &mut values {
                let mut bytes = [0; 8];
                file.read_exact(&mut bytes)?;
                *value = f64::from_le_bytes(bytes);
            }
            if offset < HEADER_BYTES || offset >= index_offset || validate_bbox(values).is_err() {
                return Err(CanonicalError::Corrupt("invalid index entry"));
            }
            entries.push(IndexEntry {
                id,
                offset,
                bbox: BBox {
                    west: values[0],
                    south: values[1],
                    east: values[2],
                    north: values[3],
                },
            });
        }
        let metadata_size = read_u64(&mut file)?;
        if metadata_size != footer_offset - provenance_offset - 8 {
            return Err(CanonicalError::Corrupt("invalid provenance length"));
        }
        let mut metadata = vec![0; metadata_size as usize];
        file.read_exact(&mut metadata)?;
        let meta: Meta = bincode::deserialize(&metadata)?;
        if meta.provenance.len() != count as usize {
            return Err(CanonicalError::Corrupt("lineage count mismatch"));
        }
        Ok(Self {
            file,
            index: RTree::bulk_load(entries),
            sources: meta.sources,
            provenance: meta.provenance,
        })
    }

    pub fn feature_count(&self) -> usize {
        self.index.size()
    }

    pub fn query(&mut self, bounds: BBox) -> Result<Vec<CanonicalFeature>, CanonicalError> {
        let mut matches: Vec<_> = self
            .index
            .locate_in_envelope_intersecting(&AABB::from_corners(
                [bounds.west, bounds.south],
                [bounds.east, bounds.north],
            ))
            .copied()
            .collect();
        matches.sort_by_key(|entry| entry.id);
        let mut output = Vec::with_capacity(matches.len());
        for entry in matches {
            self.file.seek(SeekFrom::Start(entry.offset))?;
            let size = read_u32(&mut self.file)?;
            if size > MAX_RECORD_BYTES {
                return Err(CanonicalError::Corrupt("invalid record size"));
            }
            let mut bytes = vec![0; size as usize];
            self.file.read_exact(&mut bytes)?;
            let feature: CanonicalFeature = bincode::deserialize(&bytes)?;
            if feature.id != entry.id
                || feature.bbox != entry.bbox
                || feature.geometry.bbox()? != entry.bbox
            {
                return Err(CanonicalError::Corrupt("record/index mismatch"));
            }
            output.push(feature);
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "mappa-geodb-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn license_gate_rejects_unreviewed_source() {
        let manifest = temporary("sources.toml");
        fs::write(
            &manifest,
            r#"schema_version=1
proof_region="test"
proof_bbox_wgs84=[126.0,34.0,127.0,35.0]
[[source]]
id="x"
name="x"
provider="x"
source_version="1"
download_date="2026-09-24"
official_url="https://example.com"
download_page_url="https://example.com"
file="x"
sha256="0000000000000000000000000000000000000000000000000000000000000000"
crs="EPSG:4326"
format="GeoJSON"
coverage="test"
resolution="unknown"
update_frequency="unknown"
license_id="UNKNOWN"
license_status="NEEDS_REVIEW"
license_url="https://example.com"
commercial_use=false
modification=false
redistribution=false
attribution_required=false
share_alike=false
adapter="naju-road-centerline"
adapter_version=1"#,
        )
        .unwrap();
        assert!(matches!(
            SourceManifest::open(&manifest),
            Err(CanonicalError::License(_))
        ));
        let manifest_text = fs::read_to_string(&manifest).unwrap();
        fs::write(&manifest, manifest_text.replace("NEEDS_REVIEW", "APPROVED")).unwrap();
        assert!(matches!(
            SourceManifest::open(&manifest),
            Err(CanonicalError::License(_))
        ));
        fs::remove_file(manifest).unwrap();
    }

    #[test]
    fn adapter_rejects_a_source_checksum_change() {
        let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/sources.toml");
        let manifest = SourceManifest::open(&manifest_path).unwrap();
        let mut source = manifest.source[0].clone();
        source.sha256 = "0".repeat(64);
        let [west, south, east, north] = manifest.proof_bbox_wgs84;
        assert!(matches!(
            adapt_naju_roads(
                &source,
                BBox {
                    west,
                    south,
                    east,
                    north
                }
            ),
            Err(CanonicalError::SourceChecksum(_))
        ));
    }

    #[test]
    fn licensed_worldcover_cannot_drop_required_screen_credit() {
        let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/sources.toml");
        let temporary_path = temporary("missing-credit.toml");
        let content = fs::read_to_string(manifest_path).unwrap();
        let without_credit = content
            .lines()
            .filter(|line| !line.starts_with("attribution_text = "))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&temporary_path, without_credit).unwrap();
        assert!(matches!(
            SourceManifest::open(&temporary_path),
            Err(CanonicalError::License(_))
        ));
        fs::remove_file(temporary_path).unwrap();
    }

    #[test]
    fn pinned_microsoft_buildings_keep_geometry_and_lineage() {
        let manifest_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/ms_monaco_buildings.toml");
        let manifest = SourceManifest::open(&manifest_path).unwrap();
        let [west, south, east, north] = manifest.proof_bbox_wgs84;
        let region = BBox {
            west,
            south,
            east,
            north,
        };
        let (features, rejected) = adapt_microsoft_buildings(&manifest.source[0], region).unwrap();
        assert_eq!(features.len(), 908);
        assert!(rejected.is_empty());
        assert!(features.iter().all(|(feature, provenance)| {
            feature.kind == FeatureKind::Building
                && feature.id == provenance.feature_id
                && provenance.source_id == manifest.source[0].id
        }));
        let mut modified = manifest.source[0].clone();
        modified.sha256 = "0".repeat(64);
        assert!(matches!(
            adapt_microsoft_buildings(&modified, region),
            Err(CanonicalError::SourceChecksum(_))
        ));
    }

    #[test]
    fn pinned_us_census_roads_keep_classes_and_report_exclusions() {
        let manifest_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/us_tiger_manhattan_roads.toml");
        let manifest = SourceManifest::open(&manifest_path).unwrap();
        let [west, south, east, north] = manifest.proof_bbox_wgs84;
        let region = BBox {
            west,
            south,
            east,
            north,
        };
        let (features, rejected) = adapt_us_census_roads(&manifest.source[0], region).unwrap();
        assert_eq!(features.len(), 1893);
        assert_eq!(rejected.len(), 321);
        assert!(
            features
                .iter()
                .any(|(feature, _)| feature.kind == FeatureKind::RoadPrimary)
        );
        assert!(
            features
                .iter()
                .any(|(feature, _)| feature.kind == FeatureKind::RoadSecondary)
        );
        assert!(features.iter().all(|(feature, provenance)| {
            feature.id == provenance.feature_id && provenance.source_id == manifest.source[0].id
        }));
        let mut modified = manifest.source[0].clone();
        modified.sha256 = "0".repeat(64);
        assert!(matches!(
            adapt_us_census_roads(&modified, region),
            Err(CanonicalError::SourceChecksum(_))
        ));
    }

    #[test]
    fn official_district_names_are_inside_the_proof_region() {
        let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/sources.toml");
        let manifest = SourceManifest::open(&manifest_path).unwrap();
        let source = manifest
            .source
            .iter()
            .find(|source| source.adapter == "sgis-admin-district")
            .unwrap();
        let [west, south, east, north] = manifest.proof_bbox_wgs84;
        let region = BBox {
            west,
            south,
            east,
            north,
        };
        let features = adapt_sgis_districts(source, region).unwrap();
        assert_eq!(features.len(), 14);
        assert!(
            features
                .iter()
                .any(|(feature, _)| feature.name.as_deref() == Some("송월동"))
        );
        for (feature, provenance) in features {
            assert_eq!(feature.kind, FeatureKind::PlaceDistrict);
            assert!(region.intersects(feature.bbox));
            assert_eq!(provenance.feature_id, feature.id);
            assert_eq!(provenance.source_id, source.id);
        }
    }

    #[test]
    fn official_nyc_water_and_park_sources_keep_distinct_provenance() {
        let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let water = SourceManifest::open(&data.join("us_tiger_nyc_areawater.toml")).unwrap();
        let [west, south, east, north] = water.proof_bbox_wgs84;
        let region = BBox {
            west,
            south,
            east,
            north,
        };
        let mut water_total = 0;
        for source in &water.source {
            let (features, rejected) = adapt_us_census_areawater(source, region).unwrap();
            assert!(rejected.is_empty());
            assert!(features.iter().all(|(feature, provenance)| {
                feature.kind == FeatureKind::Water
                    && feature.id == provenance.feature_id
                    && provenance.source_id == source.id
            }));
            water_total += features.len();
        }
        assert_eq!(water_total, 299);

        let parks = SourceManifest::open(&data.join("us_tiger_nyc_parks.toml")).unwrap();
        let [west, south, east, north] = parks.proof_bbox_wgs84;
        let (features, rejected) = adapt_us_census_parks(
            &parks.source[0],
            BBox {
                west,
                south,
                east,
                north,
            },
        )
        .unwrap();
        assert_eq!(features.len(), 178);
        assert_eq!(rejected.len(), 4_583);
        assert_eq!(
            rejected
                .iter()
                .filter(|record| record.reason == "polygon has unpaired or ambiguous rings")
                .count(),
            5
        );
        assert!(features.iter().all(|(feature, provenance)| {
            feature.kind == FeatureKind::Park
                && feature.id == provenance.feature_id
                && provenance.source_id == parks.source[0].id
        }));
    }

    #[test]
    fn worldcover_classes_keep_water_and_tree_cover_distinct() {
        let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/sources.toml");
        let manifest = SourceManifest::open(&manifest_path).unwrap();
        let [west, south, east, north] = manifest.proof_bbox_wgs84;
        let region = BBox {
            west,
            south,
            east,
            north,
        };
        for (adapter, expected_kind, expected_count) in [
            ("esa-worldcover-water", FeatureKind::Water, 79),
            ("esa-worldcover-tree", FeatureKind::Vegetation, 2609),
        ] {
            let source = manifest
                .source
                .iter()
                .find(|source| source.adapter == adapter)
                .unwrap();
            let (features, rejected) = adapt_worldcover_polygons(source, region).unwrap();
            assert_eq!(features.len(), expected_count);
            assert!(rejected.is_empty());
            assert!(features.iter().all(|(feature, provenance)| {
                feature.kind == expected_kind
                    && feature.id == provenance.feature_id
                    && provenance.source_id == source.id
            }));
        }
    }

    #[test]
    fn geodb_roundtrip_query_and_corruption_gate() {
        let path = temporary("roundtrip.mgeodb");
        let geometry = Geometry::Line(vec![[126.7, 35.0], [126.701, 35.001]]);
        let bbox = geometry.bbox().unwrap();
        let feature = CanonicalFeature {
            id: 42,
            kind: FeatureKind::RoadResidential,
            geometry,
            bbox,
            importance: 200,
            min_zoom: 12,
            max_zoom: 15,
            name: None,
            revision: 1,
        };
        let provenance = Provenance {
            feature_id: 42,
            source_id: "fixture".into(),
            source_feature_id: "1".into(),
            source_revision: "v1".into(),
            adapter_version: 1,
            source_feature_sha256: [7; 32],
        };
        write_geodb(&path, &[(feature.clone(), provenance.clone())]).unwrap();
        let mut db = GeoDb::open(&path).unwrap();
        assert_eq!(db.sources, vec!["fixture"]);
        assert_eq!(db.provenance, vec![provenance]);
        assert_eq!(
            db.query(BBox {
                west: 126.6,
                south: 34.9,
                east: 126.8,
                north: 35.1
            })
            .unwrap(),
            vec![feature]
        );
        assert!(
            db.query(BBox {
                west: 125.0,
                south: 34.0,
                east: 125.1,
                north: 34.1
            })
            .unwrap()
            .is_empty()
        );
        drop(db);
        let mut bytes = fs::read(&path).unwrap();
        bytes[50] ^= 1;
        fs::write(&path, bytes).unwrap();
        assert!(matches!(
            GeoDb::open(&path),
            Err(CanonicalError::Corrupt("checksum mismatch"))
        ));
        fs::remove_file(path).unwrap();
    }
}
