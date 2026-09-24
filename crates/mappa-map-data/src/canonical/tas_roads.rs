//! Build-only adapter for the LIST Transport Segments snapshot.

use super::*;
use serde_json::Value;
use std::{collections::BTreeMap, path::Component};

pub type TasRoadRegions = BTreeMap<String, AdaptedFeatures>;

fn region_for_bbox(bbox: BBox) -> String {
    let lon = (bbox.west + bbox.east) * 0.5;
    let lat = ((bbox.south + bbox.north) * 0.5).clamp(-85.051_128_78, 85.051_128_78);
    let x = (((lon + 180.0) / 360.0) * 256.0).floor().clamp(0.0, 255.0) as u32;
    let lat_radians = lat.to_radians();
    let y = ((1.0 - (lat_radians.tan() + 1.0 / lat_radians.cos()).ln() / std::f64::consts::PI)
        * 128.0)
        .floor()
        .clamp(0.0, 255.0) as u32;
    format!("z8-{x:03}-{y:03}")
}

#[derive(Deserialize)]
struct Snapshot {
    source_layer_url: String,
    source_dataset_url: String,
    license_url: String,
    source_sr: u32,
    output_sr: u32,
    expected_objectids: usize,
    ids_sha256: String,
    page: Vec<PageRecord>,
}

#[derive(Deserialize)]
struct PageRecord {
    first_objectid: u64,
    last_objectid: u64,
    features: usize,
    bytes: usize,
    sha256: String,
    file: String,
}

fn fail(message: impl Into<String>) -> CanonicalError {
    CanonicalError::Feature(message.into())
}

fn source_ids(bytes: &[u8]) -> Result<Vec<u64>, CanonicalError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|error| fail(error.to_string()))?;
    let raw = value
        .get("objectIds")
        .and_then(Value::as_array)
        .ok_or_else(|| fail("LIST source has no objectIds array"))?;
    let mut ids = raw
        .iter()
        .map(|id| id.as_u64().ok_or_else(|| fail("invalid LIST objectId")))
        .collect::<Result<Vec<_>, _>>()?;
    ids.sort_unstable();
    if ids.is_empty() || ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(fail("empty or duplicate LIST source IDs"));
    }
    Ok(ids)
}

pub fn adapt_au_tas_list_transport_segments(
    source: &SourceRecord,
) -> Result<(TasRoadRegions, Vec<RejectedFeature>), CanonicalError> {
    if source.adapter != "au-tas-list-transport-segments" {
        return Err(fail("wrong LIST roads adapter"));
    }
    let snapshot_path = Path::new(&source.file);
    if source_hash(snapshot_path)? != source.sha256.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(source.id.clone()));
    }
    let snapshot: Snapshot = toml::from_str(&fs::read_to_string(snapshot_path)?)?;
    if snapshot.source_layer_url != source.official_url
        || snapshot.source_dataset_url != source.download_page_url
        || snapshot.license_url != source.license_url
        || snapshot.source_sr != 3857
        || snapshot.output_sr != 4326
        || snapshot.expected_objectids == 0
        || snapshot.page.is_empty()
    {
        return Err(fail("invalid LIST source snapshot metadata"));
    }
    let directory = snapshot_path
        .parent()
        .ok_or_else(|| fail("snapshot has no parent"))?;
    let ids_bytes = fs::read(directory.join("ids.json"))?;
    if format!("{:x}", Sha256::digest(&ids_bytes)) != snapshot.ids_sha256 {
        return Err(CanonicalError::SourceChecksum("LIST ids.json".into()));
    }
    let ids = source_ids(&ids_bytes)?;
    if ids.len() != snapshot.expected_objectids {
        return Err(fail("LIST ID count differs from snapshot"));
    }
    let mut regions: TasRoadRegions = BTreeMap::new();
    let mut rejected = Vec::new();
    let mut offset: usize = 0;
    let mut source_ufis = BTreeSet::new();
    for page in snapshot.page {
        let path = Path::new(&page.file);
        if !page.file.starts_with("pages/")
            || !path
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
        {
            return Err(fail("unsafe LIST page path"));
        }
        let end = offset
            .checked_add(page.features)
            .filter(|end| *end <= ids.len())
            .ok_or_else(|| fail("LIST page count exceeds IDs"))?;
        let expected = &ids[offset..end];
        if expected.first().copied() != Some(page.first_objectid)
            || expected.last().copied() != Some(page.last_objectid)
        {
            return Err(fail("LIST page range differs from IDs"));
        }
        let bytes = fs::read(directory.join(path))?;
        if bytes.len() != page.bytes || format!("{:x}", Sha256::digest(&bytes)) != page.sha256 {
            return Err(CanonicalError::SourceChecksum(page.file));
        }
        let collection: geojson::FeatureCollection =
            serde_json::from_slice(&bytes).map_err(|error| fail(error.to_string()))?;
        if collection.features.len() != expected.len() {
            return Err(fail("LIST page feature count differs from IDs"));
        }
        let mut actual = Vec::with_capacity(collection.features.len());
        for raw in collection.features {
            let objectid = raw
                .property("OBJECTID")
                .and_then(Value::as_u64)
                .ok_or_else(|| fail("LIST road has no OBJECTID"))?;
            actual.push(objectid);
            let source_feature_id = raw
                .property("UFI")
                .and_then(Value::as_str)
                .ok_or_else(|| fail(format!("LIST OBJECTID {objectid} has no UFI")))?
                .to_owned();
            if !source_ufis.insert(source_feature_id.clone()) {
                return Err(fail(format!("duplicate LIST UFI {source_feature_id}")));
            }
            let property = |name: &str| raw.property(name).and_then(Value::as_str);
            let transport_class = property("TRAN_CLASS");
            let (kind, importance, min_zoom) = match transport_class {
                Some("National/State Highway" | "Arterial Road") => {
                    (FeatureKind::RoadPrimary, 750, 10)
                }
                Some("Sub Arterial Road" | "Collector Road") => {
                    (FeatureKind::RoadSecondary, 450, 11)
                }
                Some("Local Road" | "Access Road") => (FeatureKind::RoadResidential, 200, 12),
                Some("Vehicular Track") => (FeatureKind::RoadResidential, 120, 13),
                other => {
                    rejected.push(RejectedFeature {
                        source_id: source.id.clone(),
                        source_feature_id,
                        reason: format!("non-road or unmapped TRAN_CLASS: {other:?}"),
                    });
                    continue;
                }
            };
            if property("TRANS_TYPE") != Some("Road")
                || property("STATUS") != Some("Open")
                || property("USER_TYPE") != Some("Public")
            {
                rejected.push(RejectedFeature {
                    source_id: source.id.clone(),
                    source_feature_id,
                    reason: format!(
                        "not an open public road: type={:?} status={:?} user={:?}",
                        property("TRANS_TYPE"),
                        property("STATUS"),
                        property("USER_TYPE")
                    ),
                });
                continue;
            }
            let name = property("PRI_NAME")
                .map(str::trim)
                .filter(|name| !name.is_empty() && name.len() <= 128)
                .map(str::to_owned);
            let raw_bytes = serde_json::to_vec(&raw).map_err(|error| fail(error.to_string()))?;
            let parts: Vec<&Vec<geojson::Position>> =
                match raw.geometry.as_ref().map(|geometry| &geometry.value) {
                    Some(GeometryValue::LineString { coordinates }) => vec![coordinates],
                    Some(GeometryValue::MultiLineString { coordinates }) => {
                        coordinates.iter().collect()
                    }
                    _ => {
                        rejected.push(RejectedFeature {
                            source_id: source.id.clone(),
                            source_feature_id,
                            reason: "expected LineString or MultiLineString".into(),
                        });
                        continue;
                    }
                };
            for (part_index, part) in parts.into_iter().enumerate() {
                let part_id = format!("{source_feature_id}:{part_index}");
                let points = part
                    .iter()
                    .map(|position| {
                        let [lon, lat] = position.as_slice() else {
                            return Err(fail("LIST road has non-2D coordinates"));
                        };
                        Ok([*lon, *lat])
                    })
                    .collect::<Result<Vec<_>, _>>();
                let geometry = match points {
                    Ok(points) => Geometry::Line(points),
                    Err(error) => {
                        rejected.push(RejectedFeature {
                            source_id: source.id.clone(),
                            source_feature_id: part_id,
                            reason: error.to_string(),
                        });
                        continue;
                    }
                };
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
                hash.update(&raw_bytes);
                hash.update(part_index.to_le_bytes());
                regions.entry(region_for_bbox(bbox)).or_default().push((
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
        actual.sort_unstable();
        if actual != expected {
            return Err(fail("LIST page IDs differ from snapshot"));
        }
        offset = end;
    }
    if offset != ids.len() {
        return Err(fail("LIST snapshot omits source IDs"));
    }
    for records in regions.values_mut() {
        records.sort_by_key(|(feature, _)| feature.id);
    }
    Ok((regions, rejected))
}
