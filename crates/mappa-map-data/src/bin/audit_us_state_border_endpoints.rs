//! Measure road endpoint agreement near a boundary shared by two official states.
//! This is a diagnostic: coincident endpoints do not prove routable connectivity.

use mappa_map_data::canonical::{BBox, FeatureKind, GeoDb, Geometry};
use rstar::{AABB, RTree, RTreeObject};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs::File,
    io::Read,
    path::Path,
};

type Point = [f64; 2];
type Edge = (Point, Point);
const METERS_PER_DEGREE: f64 = 111_319.49;
const SEARCH_DEGREES: f64 = 0.0005;
const BORDER_METERS: f64 = 20.0;

#[derive(Clone, Copy)]
struct Border(Edge);

impl RTreeObject for Border {
    type Envelope = AABB<Point>;

    fn envelope(&self) -> Self::Envelope {
        let (a, b) = self.0;
        AABB::from_corners(
            [a[0].min(b[0]), a[1].min(b[1])],
            [a[0].max(b[0]), a[1].max(b[1])],
        )
    }
}

fn edge_key((a, b): Edge) -> ((u64, u64), (u64, u64)) {
    let a = (a[0].to_bits(), a[1].to_bits());
    let b = (b[0].to_bits(), b[1].to_bits());
    if a <= b { (a, b) } else { (b, a) }
}

fn square(point: Point, radius: f64) -> AABB<Point> {
    AABB::from_corners(
        [point[0] - radius, point[1] - radius],
        [point[0] + radius, point[1] + radius],
    )
}

fn segment_distance_m(point: Point, (a, b): Edge) -> f64 {
    let x_scale = METERS_PER_DEGREE * point[1].to_radians().cos();
    let xy = |other: Point| {
        [
            (other[0] - point[0]) * x_scale,
            (other[1] - point[1]) * METERS_PER_DEGREE,
        ]
    };
    let a = xy(a);
    let b = xy(b);
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let length = dx * dx + dy * dy;
    let t = if length > 0.0 {
        (-(a[0] * dx + a[1] * dy) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a[0] + t * dx).hypot(a[1] + t * dy)
}

fn member(archive: &mut zip::ZipArchive<File>, name: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut file = archive.by_name(name)?;
    let mut bytes = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn state_edges(
    path: &Path,
    expected_sha: &str,
    states: [&str; 2],
) -> Result<[Vec<Edge>; 2], Box<dyn Error>> {
    let bytes = std::fs::read(path)?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    if digest != expected_sha {
        return Err("state boundary ZIP SHA-256 differs from the pinned source".into());
    }
    let mut archive = zip::ZipArchive::new(File::open(path)?)?;
    let prj = member(&mut archive, "tl_2025_us_state.prj")?;
    if !std::str::from_utf8(&prj)?.contains("GCS_North_American_1983") {
        return Err("unexpected state boundary CRS".into());
    }
    let shp = member(&mut archive, "tl_2025_us_state.shp")?;
    let dbf = member(&mut archive, "tl_2025_us_state.dbf")?;
    let shape_reader = shapefile::ShapeReader::new(std::io::Cursor::new(shp))?;
    let attribute_reader = shapefile::dbase::Reader::new(std::io::Cursor::new(dbf))?;
    let mut reader = shapefile::Reader::new(shape_reader, attribute_reader);
    let mut output = [Vec::new(), Vec::new()];
    let mut found = [false; 2];
    for item in reader.iter_shapes_and_records() {
        let (shape, attributes) = item?;
        let Some(shapefile::dbase::FieldValue::Character(Some(code))) = attributes.get("STATEFP")
        else {
            return Err("state boundary lacks STATEFP".into());
        };
        let Some(index) = states.iter().position(|state| *state == code.trim()) else {
            continue;
        };
        if found[index] {
            return Err("duplicate STATEFP in boundary file".into());
        }
        found[index] = true;
        let shapefile::Shape::Polygon(polygon) = shape else {
            return Err("state boundary is not Polygon".into());
        };
        for ring in polygon.rings() {
            output[index].extend(
                ring.points()
                    .windows(2)
                    .map(|pair| ([pair[0].x, pair[0].y], [pair[1].x, pair[1].y])),
            );
        }
    }
    if !found.into_iter().all(|value| value) {
        return Err("requested STATEFP missing from boundary file".into());
    }
    Ok(output)
}

struct RoadSample {
    endpoints: Vec<Point>,
    endpoint_features: HashMap<(u64, u64), Vec<u128>>,
    feature_details: HashMap<u128, (FeatureKind, Option<String>, String, String)>,
    segments: RTree<Border>,
}

fn border_roads(
    path: &Path,
    border: &RTree<Border>,
    bounds: BBox,
) -> Result<RoadSample, Box<dyn Error>> {
    let mut db = GeoDb::open(path)?;
    let mut unique = HashSet::new();
    let mut endpoint_features: HashMap<(u64, u64), Vec<u128>> = HashMap::new();
    let mut feature_fields = HashMap::new();
    let mut segments = Vec::new();
    for feature in db.query(bounds)? {
        let Geometry::Line(line) = feature.geometry else {
            return Err("road GeoDB contains non-line geometry".into());
        };
        segments.extend(line.windows(2).map(|pair| Border((pair[0], pair[1]))));
        for point in [line[0], *line.last().ok_or("empty road line")?] {
            if border
                .locate_in_envelope_intersecting(&square(point, SEARCH_DEGREES))
                .any(|edge| segment_distance_m(point, edge.0) <= BORDER_METERS)
            {
                let key = (point[0].to_bits(), point[1].to_bits());
                unique.insert(key);
                endpoint_features.entry(key).or_default().push(feature.id);
                feature_fields.insert(feature.id, (feature.kind, feature.name.clone()));
            }
        }
    }
    let feature_details: HashMap<_, _> = db
        .provenance
        .iter()
        .filter_map(|provenance| {
            feature_fields
                .get(&provenance.feature_id)
                .map(|(kind, name)| {
                    (
                        provenance.feature_id,
                        (
                            *kind,
                            name.clone(),
                            provenance.source_id.clone(),
                            provenance.source_feature_id.clone(),
                        ),
                    )
                })
        })
        .collect();
    if feature_details.len() != feature_fields.len() {
        return Err("border road feature lacks GeoDB provenance".into());
    }
    let mut points: Vec<_> = unique
        .into_iter()
        .map(|(x, y)| [f64::from_bits(x), f64::from_bits(y)])
        .collect();
    points.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Ok(RoadSample {
        endpoints: points,
        endpoint_features,
        feature_details,
        segments: RTree::bulk_load(segments),
    })
}

fn report(
    label: &str,
    sample: &RoadSample,
    opposite: &RTree<Point>,
    lines: &RTree<Border>,
    details: bool,
) {
    let mut bins = [0usize; 5];
    let mut line_bins = [0usize; 5];
    let mut samples = Vec::new();
    let mut candidate_points = Vec::new();
    for &point in &sample.endpoints {
        let nearest = opposite
            .locate_in_envelope_intersecting(&square(point, SEARCH_DEGREES))
            .map(|&other| segment_distance_m(point, (other, other)))
            .fold(f64::INFINITY, f64::min);
        let bin = if nearest <= 0.1 {
            0
        } else if nearest <= 1.0 {
            1
        } else if nearest <= 5.0 {
            2
        } else if nearest <= 20.0 {
            3
        } else {
            4
        };
        bins[bin] += 1;
        if bin == 4 {
            let nearest_line = lines
                .locate_in_envelope_intersecting(&square(point, SEARCH_DEGREES))
                .map(|line| segment_distance_m(point, line.0))
                .fold(f64::INFINITY, f64::min);
            let line_bin = if nearest_line <= 0.1 {
                0
            } else if nearest_line <= 1.0 {
                1
            } else if nearest_line <= 5.0 {
                2
            } else if nearest_line <= 20.0 {
                3
            } else {
                4
            };
            line_bins[line_bin] += 1;
            if line_bin == 4 {
                candidate_points.push(point);
                if samples.len() < 12 {
                    samples.push(format!("[{:.6},{:.6}]", point[0], point[1]));
                }
            }
        }
    }
    println!(
        "{label} endpoints={} matches_0_1m={} matches_1m={} matches_5m={} matches_20m={} no_endpoint_20m={} on_opposite_line_0_1m={} on_opposite_line_1m={} on_opposite_line_5m={} on_opposite_line_20m={} no_opposite_line_20m={} candidate_gap_samples={}",
        sample.endpoints.len(),
        bins[0],
        bins[1],
        bins[2],
        bins[3],
        bins[4],
        line_bins[0],
        line_bins[1],
        line_bins[2],
        line_bins[3],
        line_bins[4],
        samples.join(" ")
    );
    if details {
        for point in candidate_points {
            let key = (point[0].to_bits(), point[1].to_bits());
            if let Some(ids) = sample.endpoint_features.get(&key) {
                for id in ids {
                    if let Some((kind, name, source_id, source_feature_id)) =
                        sample.feature_details.get(id)
                    {
                        println!(
                            "candidate_gap state={label} lon={:.6} lat={:.6} kind={kind:?} name={name:?} source_id={source_id} source_feature_id={source_feature_id}",
                            point[0], point[1]
                        );
                    }
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 7 && (args.len() != 8 || args[7] != "--details") {
        return Err("usage: audit_us_state_border_endpoints STATE.zip SHA256 FIRST_STATEFP FIRST.mgeodb SECOND_STATEFP SECOND.mgeodb [--details]".into());
    }
    let [first, second] = state_edges(Path::new(&args[1]), &args[2], [&args[3], &args[5]])?;
    let first: HashSet<_> = first.into_iter().map(edge_key).collect();
    let mut common: Vec<_> = second
        .into_iter()
        .filter(|edge| first.contains(&edge_key(*edge)))
        .collect();
    if common.is_empty() {
        return Err("state polygons have no identical shared boundary segments".into());
    }
    common.sort_by_key(|edge| edge_key(*edge));
    common.dedup_by_key(|edge| edge_key(*edge));
    let mut bounds = BBox {
        west: f64::INFINITY,
        south: f64::INFINITY,
        east: f64::NEG_INFINITY,
        north: f64::NEG_INFINITY,
    };
    for &(a, b) in &common {
        for [lon, lat] in [a, b] {
            bounds.west = bounds.west.min(lon);
            bounds.south = bounds.south.min(lat);
            bounds.east = bounds.east.max(lon);
            bounds.north = bounds.north.max(lat);
        }
    }
    bounds.west -= SEARCH_DEGREES;
    bounds.south -= SEARCH_DEGREES;
    bounds.east += SEARCH_DEGREES;
    bounds.north += SEARCH_DEGREES;
    let border = RTree::bulk_load(common.iter().copied().map(Border).collect());
    let a = border_roads(Path::new(&args[4]), &border, bounds)?;
    let b = border_roads(Path::new(&args[6]), &border, bounds)?;
    println!(
        "shared_border_segments={} first_state={} second_state={}",
        common.len(),
        args[3],
        args[5]
    );
    report(
        &args[3],
        &a,
        &RTree::bulk_load(b.endpoints.clone()),
        &b.segments,
        args.len() == 8,
    );
    report(
        &args[5],
        &b,
        &RTree::bulk_load(a.endpoints.clone()),
        &a.segments,
        args.len() == 8,
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reversed_border_edges_have_the_same_identity() {
        let edge = ([0.0, 0.0], [1.0, 1.0]);
        assert_eq!(edge_key(edge), edge_key((edge.1, edge.0)));
    }

    #[test]
    fn point_distance_uses_segment_interior_and_endpoints() {
        let edge = ([0.0, 0.0], [0.01, 0.0]);
        assert!(segment_distance_m([0.005, 0.0], edge) < 0.01);
        assert!((segment_distance_m([0.02, 0.0], edge) - 1113.1949).abs() < 0.1);
    }
}
