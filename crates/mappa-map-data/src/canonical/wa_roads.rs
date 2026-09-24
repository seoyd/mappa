//! Build-only adapter for the Main Roads Western Australia Road Network archive.

use super::*;
use std::collections::BTreeMap;

pub type WaRoadRegions = BTreeMap<String, AdaptedFeatures>;

/// GDA94 geographic degrees are retained numerically. No independent WGS84
/// position or road connectivity claim follows from this conversion.
pub fn adapt_wa_road_network(
    source: &SourceRecord,
) -> Result<(WaRoadRegions, Vec<RejectedFeature>), CanonicalError> {
    if source.adapter != "au-wa-road-network" {
        return Err(CanonicalError::Feature("wrong WA road adapter".into()));
    }
    if source_hash(Path::new(&source.file))? != source.sha256.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(source.id.clone()));
    }
    let mut archive = zip::ZipArchive::new(File::open(&source.file)?)
        .map_err(|error| CanonicalError::Feature(error.to_string()))?;
    let temporary = tempfile::tempdir()?;
    let mut paths = Vec::new();
    for extension in ["prj", "shp", "dbf"] {
        let name = format!("Road_Network.{extension}");
        let mut member = archive
            .by_name(&name)
            .map_err(|error| CanonicalError::Feature(error.to_string()))?;
        if member.size() > 2 * 1024 * 1024 * 1024 {
            return Err(CanonicalError::Feature(
                "WA source member exceeds 2 GiB".into(),
            ));
        }
        let path = temporary.path().join(&name);
        let mut output = File::create(&path)?;
        std::io::copy(&mut member, &mut output)?;
        paths.push(path);
    }
    let prj = fs::read_to_string(&paths[0])?;
    if !prj.contains("GCS_GDA_1994") || !prj.contains("UNIT[\"Degree\"") {
        return Err(CanonicalError::Feature("unexpected WA source CRS".into()));
    }
    let shape_reader = shapefile::ShapeReader::new(File::open(&paths[1])?)?;
    let attribute_reader = shapefile::dbase::Reader::new(File::open(&paths[2])?)?;
    let mut reader = shapefile::Reader::new(shape_reader, attribute_reader);
    let mut regions: WaRoadRegions = BTreeMap::new();
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
        let source_feature_id = string_field("GlobalID")
            .filter(|id| !id.is_empty())
            .ok_or_else(|| CanonicalError::Feature("missing WA GlobalID".into()))?
            .to_owned();
        if !raw_ids.insert(source_feature_id.clone()) {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id,
                reason: "duplicate GlobalID".into(),
            });
            continue;
        }
        let class = string_field("NETWORK_TY");
        let (kind, importance, min_zoom) = match class {
            Some("State Road") => (FeatureKind::RoadPrimary, 700, 10),
            Some("Local Road") => (FeatureKind::RoadResidential, 200, 12),
            Some("Miscellaneous Road") => (FeatureKind::RoadResidential, 160, 12),
            other => {
                rejected.push(RejectedFeature {
                    source_id: source.id.clone(),
                    source_feature_id,
                    reason: format!("non-road or unsupported NETWORK_TYPE: {other:?}"),
                });
                continue;
            }
        };
        let region = string_field("RA_NAME")
            .filter(|name| !name.is_empty())
            .unwrap_or("Unassigned");
        let name = string_field("ROAD_NAME")
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
            let id = stable_id(&source.id, &part_id);
            let mut hash = Sha256::new();
            hash.update(part_id.as_bytes());
            hash.update(class.unwrap_or_default().as_bytes());
            hash.update(name.as_deref().unwrap_or_default().as_bytes());
            for point in part {
                hash.update(point.x.to_le_bytes());
                hash.update(point.y.to_le_bytes());
            }
            regions.entry(region.to_owned()).or_default().push((
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
    for records in regions.values_mut() {
        records.sort_by_key(|(feature, _)| feature.id);
    }
    Ok((regions, rejected))
}
