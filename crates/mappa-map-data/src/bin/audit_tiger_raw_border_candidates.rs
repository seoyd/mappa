//! Check whether excluded raw TIGER road classes explain canonical border gap candidates.
//! This compares source geometry only; it cannot establish physical road connectivity.

use mappa_map_data::canonical::SourceManifest;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, error::Error, fs, fs::File, io::Read, path::Path};

type Point = [f64; 2];
const METERS_PER_DEGREE: f64 = 111_319.49;

#[derive(Clone)]
struct Nearest {
    distance_m: f64,
    source_id: String,
    linearid: String,
    mtfcc: String,
    name: Option<String>,
}

struct Candidate {
    point: Point,
    nearest: Option<Nearest>,
    raw_within_radius: usize,
    accepted_within_radius: usize,
    excluded_within_radius: usize,
}

fn candidates(path: &Path, state: &str) -> Result<Vec<Candidate>, Box<dyn Error>> {
    let prefix = format!("candidate_gap state={state} ");
    let mut seen = BTreeSet::new();
    let mut output = Vec::new();
    for line in fs::read_to_string(path)?
        .lines()
        .filter(|line| line.starts_with(&prefix))
    {
        let lon = line
            .split_whitespace()
            .find_map(|part| part.strip_prefix("lon="))
            .ok_or("candidate is missing longitude")?
            .parse::<f64>()?;
        let lat = line
            .split_whitespace()
            .find_map(|part| part.strip_prefix("lat="))
            .ok_or("candidate is missing latitude")?
            .parse::<f64>()?;
        if !lon.is_finite() || !lat.is_finite() {
            return Err("candidate coordinate is not finite".into());
        }
        if seen.insert((lon.to_bits(), lat.to_bits())) {
            output.push(Candidate {
                point: [lon, lat],
                nearest: None,
                raw_within_radius: 0,
                accepted_within_radius: 0,
                excluded_within_radius: 0,
            });
        }
    }
    if output.is_empty() {
        return Err("no candidate points for requested state".into());
    }
    output.sort_by(|a, b| a.point.partial_cmp(&b.point).unwrap());
    Ok(output)
}

fn segment_distance_m(point: Point, a: Point, b: Point) -> f64 {
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

fn member(archive: &mut zip::ZipArchive<File>, suffix: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let matches: Vec<_> = archive
        .file_names()
        .filter(|name| name.ends_with(suffix))
        .map(str::to_owned)
        .collect();
    let [name] = matches.as_slice() else {
        return Err(format!("expected exactly one {suffix} in TIGER ZIP").into());
    };
    let mut file = archive.by_name(name)?;
    if file.size() > 64 * 1024 * 1024 {
        return Err(format!("TIGER {suffix} exceeds 64 MiB audit limit").into());
    }
    let mut bytes = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn field<'a>(attributes: &'a shapefile::dbase::Record, name: &str) -> Option<&'a str> {
    match attributes.get(name) {
        Some(shapefile::dbase::FieldValue::Character(Some(value))) => Some(value.trim()),
        _ => None,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err("usage: audit_tiger_raw_border_candidates OPPOSITE_ROADS_MANIFEST.toml BORDER_DETAILS.log CANDIDATE_STATEFP RADIUS_METERS".into());
    }
    let manifest = SourceManifest::open(Path::new(&args[1]))?;
    if manifest
        .source
        .iter()
        .any(|source| source.adapter != "us-census-tiger-roads")
    {
        return Err("opposite manifest contains a non-TIGER road source".into());
    }
    let mut points = candidates(Path::new(&args[2]), &args[3])?;
    let radius_m: f64 = args[4].parse()?;
    if !radius_m.is_finite() || radius_m <= 0.0 || radius_m > 1000.0 {
        return Err("radius must be between zero and 1000 meters".into());
    }
    let mut inspected_sources = 0usize;
    let mut inspected_rows = 0usize;
    for source in &manifest.source {
        let mut archive = zip::ZipArchive::new(File::open(&source.file)?)?;
        let shp = member(&mut archive, ".shp")?;
        if shp.len() < 100 || u32::from_be_bytes(shp[0..4].try_into()?) != 9994 {
            return Err("invalid TIGER Shapefile header".into());
        }
        let mut bounds = [0.0; 4];
        for (index, value) in bounds.iter_mut().enumerate() {
            let start = 36 + index * 8;
            *value = f64::from_le_bytes(shp[start..start + 8].try_into()?);
        }
        let relevant: Vec<_> = points
            .iter()
            .enumerate()
            .filter(|(_, candidate)| {
                let [lon, lat] = candidate.point;
                let delta = radius_m / (METERS_PER_DEGREE * lat.to_radians().cos()) + 0.0001;
                lon >= bounds[0] - delta
                    && lon <= bounds[2] + delta
                    && lat >= bounds[1] - delta
                    && lat <= bounds[3] + delta
            })
            .map(|(index, _)| index)
            .collect();
        if relevant.is_empty() {
            continue;
        }
        let digest = format!("{:x}", Sha256::digest(fs::read(&source.file)?));
        if digest != source.sha256 {
            return Err(format!("source ZIP checksum mismatch: {}", source.id).into());
        }
        let prj = member(&mut archive, ".prj")?;
        if !std::str::from_utf8(&prj)?.contains("GCS_North_American_1983") {
            return Err("unexpected TIGER CRS".into());
        }
        let dbf = member(&mut archive, ".dbf")?;
        let shape_reader = shapefile::ShapeReader::new(std::io::Cursor::new(shp))?;
        let attribute_reader = shapefile::dbase::Reader::new(std::io::Cursor::new(dbf))?;
        let mut reader = shapefile::Reader::new(shape_reader, attribute_reader);
        inspected_sources += 1;
        for item in reader.iter_shapes_and_records() {
            let (shape, attributes) = item?;
            inspected_rows += 1;
            let shapefile::Shape::Polyline(line) = shape else {
                return Err("TIGER All Roads row is not a polyline".into());
            };
            let mtfcc = field(&attributes, "MTFCC").unwrap_or_default();
            let accepted = matches!(mtfcc, "S1100" | "S1200" | "S1400" | "S1630" | "S1640");
            for &index in &relevant {
                let candidate = &mut points[index];
                let distance_m = line
                    .parts()
                    .iter()
                    .flat_map(|part| part.windows(2))
                    .map(|segment| {
                        segment_distance_m(
                            candidate.point,
                            [segment[0].x, segment[0].y],
                            [segment[1].x, segment[1].y],
                        )
                    })
                    .fold(f64::INFINITY, f64::min);
                if distance_m
                    < candidate
                        .nearest
                        .as_ref()
                        .map_or(f64::INFINITY, |near| near.distance_m)
                {
                    candidate.nearest = Some(Nearest {
                        distance_m,
                        source_id: source.id.clone(),
                        linearid: field(&attributes, "LINEARID")
                            .unwrap_or_default()
                            .to_owned(),
                        mtfcc: mtfcc.to_owned(),
                        name: field(&attributes, "FULLNAME").map(str::to_owned),
                    });
                }
                if distance_m <= radius_m {
                    candidate.raw_within_radius += 1;
                    if accepted {
                        candidate.accepted_within_radius += 1;
                    } else {
                        candidate.excluded_within_radius += 1;
                    }
                }
            }
        }
    }
    println!(
        "candidate_points={} inspected_sources={} inspected_rows={} radius_m={radius_m}",
        points.len(),
        inspected_sources,
        inspected_rows
    );
    for candidate in &points {
        let [lon, lat] = candidate.point;
        if let Some(near) = &candidate.nearest {
            println!(
                "candidate lon={lon:.6} lat={lat:.6} raw_within_radius={} accepted_within_radius={} excluded_within_radius={} nearest_m={:.3} nearest_mtfcc={} nearest_name={:?} source_id={} source_feature_id={}",
                candidate.raw_within_radius,
                candidate.accepted_within_radius,
                candidate.excluded_within_radius,
                near.distance_m,
                near.mtfcc,
                near.name,
                near.source_id,
                near.linearid
            );
        } else {
            println!(
                "candidate lon={lon:.6} lat={lat:.6} raw_within_radius=0 accepted_within_radius=0 excluded_within_radius=0 nearest=none"
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_to_segment_distance_handles_interior_and_endpoint() {
        assert!(segment_distance_m([0.005, 0.0], [0.0, 0.0], [0.01, 0.0]) < 0.01);
        assert!((segment_distance_m([0.02, 0.0], [0.0, 0.0], [0.01, 0.0]) - 1113.1949).abs() < 0.1);
    }
}
