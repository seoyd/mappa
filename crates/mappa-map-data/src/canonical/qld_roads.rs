//! Build-only adapter for the Queensland Roads and Tracks source snapshot.

use super::*;
use std::{collections::BTreeMap, path::Component};

pub type QldRoadRegions = BTreeMap<String, AdaptedFeatures>;

#[derive(Deserialize)]
struct Snapshot {
    source_layer_url: String,
    out_sr: u32,
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
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|error| fail(error.to_string()))?;
    let raw = value
        .get("objectIds")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| fail("Queensland source has no objectIds array"))?;
    let mut ids = raw
        .iter()
        .map(|id| {
            id.as_u64()
                .ok_or_else(|| fail("invalid Queensland objectId"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    ids.sort_unstable();
    if ids.is_empty() || ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(fail("empty or duplicate Queensland source IDs"));
    }
    Ok(ids)
}

pub fn adapt_au_qld_qrt_roads(
    source: &SourceRecord,
) -> Result<(QldRoadRegions, Vec<RejectedFeature>), CanonicalError> {
    if source.adapter != "au-qld-qrt-roads" {
        return Err(fail("wrong Queensland roads adapter"));
    }
    let snapshot_path = Path::new(&source.file);
    if source_hash(snapshot_path)? != source.sha256.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(source.id.clone()));
    }
    let snapshot: Snapshot = toml::from_str(&fs::read_to_string(snapshot_path)?)?;
    if snapshot.out_sr != 4326
        || snapshot.expected_objectids == 0
        || snapshot.page.is_empty()
        || source.official_url != snapshot.source_layer_url
    {
        return Err(fail("invalid Queensland source snapshot metadata"));
    }
    let directory = snapshot_path
        .parent()
        .ok_or_else(|| fail("Queensland snapshot has no parent"))?;
    let ids_bytes = fs::read(directory.join("ids.json"))?;
    if format!("{:x}", Sha256::digest(&ids_bytes)) != snapshot.ids_sha256 {
        return Err(CanonicalError::SourceChecksum("Queensland ids.json".into()));
    }
    let ids = source_ids(&ids_bytes)?;
    if ids.len() != snapshot.expected_objectids {
        return Err(fail("Queensland source ID count differs from snapshot"));
    }
    let mut regions: QldRoadRegions = BTreeMap::new();
    let mut rejected = Vec::new();
    let mut offset: usize = 0;
    for page in snapshot.page {
        let page_path = Path::new(&page.file);
        if !page.file.starts_with("pages/")
            || !page_path
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
        {
            return Err(fail("unsafe Queensland source page path"));
        }
        let end = offset
            .checked_add(page.features)
            .filter(|end| *end <= ids.len())
            .ok_or_else(|| fail("Queensland page count exceeds source IDs"))?;
        let expected = &ids[offset..end];
        if expected.first().copied() != Some(page.first_objectid)
            || expected.last().copied() != Some(page.last_objectid)
        {
            return Err(fail("Queensland page ranges differ from source IDs"));
        }
        let bytes = fs::read(directory.join(page_path))?;
        if bytes.len() != page.bytes || format!("{:x}", Sha256::digest(&bytes)) != page.sha256 {
            return Err(CanonicalError::SourceChecksum(page.file));
        }
        let collection: geojson::FeatureCollection =
            serde_json::from_slice(&bytes).map_err(|error| fail(error.to_string()))?;
        if collection.features.len() != expected.len() {
            return Err(fail(
                "Queensland page feature count differs from source IDs",
            ));
        }
        let mut actual = Vec::with_capacity(collection.features.len());
        for raw in collection.features {
            let objectid = raw
                .property("objectid")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| fail("Queensland feature has no objectid"))?;
            actual.push(objectid);
            let source_feature_id = objectid.to_string();
            let property = |name: &str| raw.property(name).and_then(serde_json::Value::as_str);
            let class = property("class");
            let (kind, importance, min_zoom) = match class {
                Some("Motorway") => (FeatureKind::RoadPrimary, 750, 10),
                Some("Highway") => (FeatureKind::RoadPrimary, 700, 10),
                Some("Secondary") => (FeatureKind::RoadSecondary, 520, 11),
                Some("Connector") => (FeatureKind::RoadSecondary, 400, 11),
                Some("Local") => (FeatureKind::RoadResidential, 200, 12),
                other => {
                    rejected.push(RejectedFeature {
                        source_id: source.id.clone(),
                        source_feature_id,
                        reason: format!("non-general-road or unmapped class: {other:?}"),
                    });
                    continue;
                }
            };
            if property("record_status") != Some("C")
                || property("op_status_ind") != Some("Operational")
            {
                rejected.push(RejectedFeature {
                    source_id: source.id.clone(),
                    source_feature_id,
                    reason: format!(
                        "non-current or non-operational: record_status={:?} op_status_ind={:?}",
                        property("record_status"),
                        property("op_status_ind")
                    ),
                });
                continue;
            }
            if matches!(property("user_access"), Some("Private" | "Restricted"))
                || property("sub_class") == Some("Crossover")
            {
                rejected.push(RejectedFeature {
                    source_id: source.id.clone(),
                    source_feature_id,
                    reason: format!(
                        "access or crossover excluded: user_access={:?} sub_class={:?}",
                        property("user_access"),
                        property("sub_class")
                    ),
                });
                continue;
            }
            let region = property("lga_name_left")
                .or_else(|| property("lga_name_right"))
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .unwrap_or("Unassigned")
                .to_owned();
            let name = property("road_name_full")
                .map(str::trim)
                .filter(|value| !value.is_empty() && value.len() <= 128)
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
                            return Err(fail("Queensland road has non-2D coordinates"));
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
                regions.entry(region.clone()).or_default().push((
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
            return Err(fail("Queensland page IDs differ from source snapshot"));
        }
        offset = end;
    }
    if offset != ids.len() {
        return Err(fail("Queensland snapshot omits source IDs"));
    }
    for records in regions.values_mut() {
        records.sort_by_key(|(feature, _)| feature.id);
    }
    Ok((regions, rejected))
}
