//! Build-only adapter for Ireland's official Road Management Office local-road schedule.

use super::*;
use proj4rs::{proj::Proj, transform::transform};

const IRISH_TM: &str = concat!(
    "+proj=tmerc +lat_0=53.5 +lon_0=-8 +k=0.99982 ",
    "+x_0=600000 +y_0=750000 +ellps=GRS80 +units=m +no_defs +type=crs"
);
const WGS84: &str = "+proj=longlat +ellps=WGS84 +datum=WGS84 +no_defs";
const STEM: &str = "Local_Road_Schedule_Open_data_Shapefile";

fn invalid(message: impl Into<String>) -> CanonicalError {
    CanonicalError::Feature(message.into())
}

pub fn adapt_ie_rmo_local_roads(
    source: &SourceRecord,
) -> Result<(AdaptedFeatures, Vec<RejectedFeature>), CanonicalError> {
    if source.adapter != "ie-rmo-local-roads" {
        return Err(invalid("wrong Irish RMO local-roads adapter"));
    }
    if source_hash(Path::new(&source.file))? != source.sha256.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(source.id.clone()));
    }
    let mut archive = zip::ZipArchive::new(File::open(&source.file)?)
        .map_err(|error| invalid(error.to_string()))?;
    let temporary = tempfile::tempdir()?;
    let mut paths = Vec::new();
    for extension in ["prj", "shp", "dbf"] {
        let name = format!("{STEM}.{extension}");
        let mut member = archive
            .by_name(&name)
            .map_err(|error| invalid(error.to_string()))?;
        if member.size() > 512 * 1024 * 1024 {
            return Err(invalid("Irish source member exceeds 512 MiB"));
        }
        let path = temporary.path().join(name);
        std::io::copy(&mut member, &mut File::create(&path)?)?;
        paths.push(path);
    }
    let prj = fs::read_to_string(&paths[0])?;
    if !prj.contains("IRENET95_Irish_Transverse_Mercator")
        || !prj.contains("600000.0")
        || !prj.contains("750000.0")
        || !prj.contains("0.99982")
        || !prj.contains("GRS_1980")
    {
        return Err(invalid("unexpected Irish Transverse Mercator projection"));
    }
    let from = Proj::from_proj_string(IRISH_TM).map_err(|error| invalid(error.to_string()))?;
    let to = Proj::from_proj_string(WGS84).map_err(|error| invalid(error.to_string()))?;
    let shapes = shapefile::ShapeReader::new(File::open(&paths[1])?)?;
    let attributes = shapefile::dbase::Reader::new(File::open(&paths[2])?)?;
    let mut reader = shapefile::Reader::new(shapes, attributes);
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    for (row_index, record) in reader.iter_shapes_and_records().enumerate() {
        let (shape, attributes) = record?;
        let text = |field: &str| -> Option<&str> {
            match attributes.get(field) {
                Some(shapefile::dbase::FieldValue::Character(Some(value))) => Some(value.trim()),
                _ => None,
            }
        };
        let route = text("FIRST_Rout").unwrap_or("");
        let council = text("FIRST_Full").unwrap_or("");
        let road_class = text("FIRST_Road");
        let source_feature_id = format!("{row_index}:{council}:{route}");
        let (kind, importance, min_zoom) = match road_class {
            Some("Local Primary") => (FeatureKind::RoadSecondary, 450, 11),
            Some("Local Secondary") => (FeatureKind::RoadResidential, 240, 12),
            Some("Local Tertiary") => (FeatureKind::RoadResidential, 180, 12),
            other => {
                rejected.push(RejectedFeature {
                    source_id: source.id.clone(),
                    source_feature_id,
                    reason: format!("unsupported local road class: {other:?}"),
                });
                continue;
            }
        };
        let shapefile::Shape::Polyline(line) = shape else {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id,
                reason: "expected Polyline".into(),
            });
            continue;
        };
        for (part_index, part) in line.parts().iter().enumerate() {
            let part_id = format!("{source_feature_id}:{part_index}");
            let mut points = Vec::with_capacity(part.len());
            let mut hash = Sha256::new();
            hash.update(part_id.as_bytes());
            hash.update(road_class.unwrap_or_default().as_bytes());
            for point in part {
                hash.update(point.x.to_le_bytes());
                hash.update(point.y.to_le_bytes());
                let mut projected = (point.x, point.y, 0.0);
                transform(&from, &to, &mut projected)
                    .map_err(|error| invalid(error.to_string()))?;
                points.push([projected.0.to_degrees(), projected.1.to_degrees()]);
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
            accepted.push((
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
                    source_feature_id: part_id,
                    source_revision: source.source_version.clone(),
                    adapter_version: source.adapter_version,
                    source_feature_sha256: hash.finalize().into(),
                },
            ));
        }
    }
    accepted.sort_by_key(|(feature, _)| feature.id);
    Ok((accepted, rejected))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn irish_tm_matches_independent_proj_point() {
        let from = Proj::from_proj_string(IRISH_TM).unwrap();
        let to = Proj::from_proj_string(WGS84).unwrap();
        let mut point = (568779.001, 571424.814, 0.0);
        transform(&from, &to, &mut point).unwrap();
        // GDAL/PROJ EPSG:2157 → EPSG:4326 for the first official source vertex.
        assert!((point.0.to_degrees() - -8.453613044873586).abs() < 0.000001);
        assert!((point.1.to_degrees() - 51.894107317160014).abs() < 0.000001);
    }
}
