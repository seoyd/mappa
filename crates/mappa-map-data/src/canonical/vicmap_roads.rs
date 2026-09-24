//! Build-only adapter for an audited Vicmap Transport Road Line WFS snapshot.

use super::*;
use serde_json::Value;
use std::{collections::BTreeMap, path::Component};

pub type VicmapRegions = BTreeMap<String, AdaptedFeatures>;

#[derive(Deserialize)]
struct Snapshot {
    source_wfs_url: String,
    source_type_name: String,
    output_crs: String,
    expected_ufi_count: usize,
    ufi_sha256: String,
    page: Vec<PageRecord>,
}

#[derive(Deserialize)]
struct PageRecord {
    first_ufi: u64,
    last_ufi: u64,
    features: usize,
    bytes: usize,
    sha256: String,
    file: String,
}

fn fail(message: impl Into<String>) -> CanonicalError {
    CanonicalError::Feature(message.into())
}

fn read_ids(
    directory: &Path,
    count: usize,
    expected_hash: &str,
) -> Result<Vec<u64>, CanonicalError> {
    let mut ids = Vec::with_capacity(count);
    let mut hash = Sha256::new();
    for page_index in 0..count.div_ceil(5_000) {
        let bytes = fs::read(directory.join(format!("ids/page-{page_index:04}.json")))?;
        let value: Value =
            serde_json::from_slice(&bytes).map_err(|error| fail(error.to_string()))?;
        let records = value
            .get("features")
            .and_then(Value::as_array)
            .ok_or_else(|| fail("Vicmap ID page has no features"))?;
        let expected_len = 5_000.min(count - page_index * 5_000);
        if records.len() != expected_len
            || value.get("numberMatched").and_then(Value::as_u64) != Some(count as u64)
        {
            return Err(fail("Vicmap ID page count differs from snapshot"));
        }
        for record in records {
            let id = record
                .get("properties")
                .and_then(|properties| properties.get("ufi"))
                .and_then(Value::as_u64)
                .ok_or_else(|| fail("Vicmap ID page has invalid UFI"))?;
            if ids.last().is_some_and(|previous| *previous >= id) {
                return Err(fail("Vicmap ID pages are unordered or contain duplicates"));
            }
            hash.update(id.to_le_bytes());
            ids.push(id);
        }
    }
    if ids.len() != count || format!("{:x}", hash.finalize()) != expected_hash {
        return Err(CanonicalError::SourceChecksum("Vicmap UFI index".into()));
    }
    Ok(ids)
}

fn region_for_bbox(bbox: BBox) -> String {
    // Web Mercator z8 cells keep packs local without an external boundary dataset.
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

pub fn adapt_au_vic_vicmap_roads(
    source: &SourceRecord,
) -> Result<(VicmapRegions, Vec<RejectedFeature>), CanonicalError> {
    if source.adapter != "au-vic-vicmap-roads" {
        return Err(fail("wrong Vicmap roads adapter"));
    }
    let snapshot_path = Path::new(&source.file);
    if source_hash(snapshot_path)? != source.sha256.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(source.id.clone()));
    }
    let snapshot: Snapshot = toml::from_str(&fs::read_to_string(snapshot_path)?)?;
    if snapshot.source_wfs_url != source.official_url
        || snapshot.source_type_name != "open-data-platform:tr_road"
        || snapshot.output_crs != "EPSG:4326"
        || snapshot.expected_ufi_count == 0
        || snapshot.page.is_empty()
    {
        return Err(fail("invalid Vicmap source snapshot metadata"));
    }
    let directory = snapshot_path
        .parent()
        .ok_or_else(|| fail("snapshot has no parent"))?;
    let ids = read_ids(directory, snapshot.expected_ufi_count, &snapshot.ufi_sha256)?;
    let mut regions: VicmapRegions = BTreeMap::new();
    let mut rejected = Vec::new();
    let mut offset: usize = 0;
    for page in snapshot.page {
        let path = Path::new(&page.file);
        if !page.file.starts_with("pages/")
            || !path
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
        {
            return Err(fail("unsafe Vicmap page path"));
        }
        let end = offset
            .checked_add(page.features)
            .filter(|end| *end <= ids.len())
            .ok_or_else(|| fail("Vicmap page count exceeds ID snapshot"))?;
        let expected = &ids[offset..end];
        if expected.first().copied() != Some(page.first_ufi)
            || expected.last().copied() != Some(page.last_ufi)
        {
            return Err(fail("Vicmap page range differs from ID snapshot"));
        }
        let bytes = fs::read(directory.join(path))?;
        if bytes.len() != page.bytes || format!("{:x}", Sha256::digest(&bytes)) != page.sha256 {
            return Err(CanonicalError::SourceChecksum(page.file));
        }
        let collection: geojson::FeatureCollection =
            serde_json::from_slice(&bytes).map_err(|error| fail(error.to_string()))?;
        if collection.features.len() != expected.len() {
            return Err(fail("Vicmap page feature count differs from IDs"));
        }
        for (raw, expected_ufi) in collection.features.into_iter().zip(expected) {
            let ufi = raw
                .property("ufi")
                .and_then(Value::as_u64)
                .ok_or_else(|| fail("Vicmap road has no UFI"))?;
            if ufi != *expected_ufi {
                return Err(fail("Vicmap road UFI differs from ID snapshot"));
            }
            let source_feature_id = ufi.to_string();
            let property = |name: &str| raw.property(name).and_then(Value::as_str);
            let feature_type = property("feature_type_code");
            let class = raw.property("class_code").and_then(Value::as_i64);
            let supported_type = matches!(
                feature_type,
                Some(
                    "road"
                        | "connector"
                        | "bridge"
                        | "tunnel"
                        | "ford"
                        | "dip"
                        | "level crossing"
                        | "roundabout"
                )
            );
            let (kind, importance, min_zoom) = match class {
                Some(0 | 1) => (FeatureKind::RoadPrimary, 750, 10),
                Some(2 | 3) => (FeatureKind::RoadSecondary, 500, 11),
                Some(4) => (FeatureKind::RoadSecondary, 320, 11),
                Some(5 | 6) => (FeatureKind::RoadResidential, 200, 12),
                Some(7 | 8) => (FeatureKind::RoadResidential, 120, 13),
                _ => {
                    rejected.push(RejectedFeature {
                        source_id: source.id.clone(),
                        source_feature_id,
                        reason: format!("non-vehicular or unknown class_code: {class:?}"),
                    });
                    continue;
                }
            };
            if !supported_type
                || property("road_status") != Some("O")
                || !matches!(property("vehicular_access"), Some("1" | "2" | "3"))
                || matches!(property("physical_condition"), Some("2"))
                || property("restrictions") == Some("4")
            {
                rejected.push(RejectedFeature {
                    source_id: source.id.clone(), source_feature_id,
                    reason: format!(
                        "not an open public vehicular road: type={feature_type:?} status={:?} access={:?} condition={:?} restriction={:?}",
                        property("road_status"), property("vehicular_access"),
                        property("physical_condition"), property("restrictions")
                    ),
                });
                continue;
            }
            let name = property("ezi_road_name_label")
                .or_else(|| property("ezi_road_name"))
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
                            return Err(fail("Vicmap road has non-2D coordinates"));
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
        offset = end;
    }
    if offset != ids.len() {
        return Err(fail("Vicmap snapshot omits source IDs"));
    }
    for records in regions.values_mut() {
        records.sort_by_key(|(feature, _)| feature.id);
    }
    Ok((regions, rejected))
}

#[cfg(test)]
mod tests {
    use super::region_for_bbox;
    use crate::canonical::BBox;

    #[test]
    fn melbourne_and_geelong_have_distinct_z8_regions() {
        let melbourne = BBox {
            west: 144.96,
            south: -37.82,
            east: 144.97,
            north: -37.81,
        };
        let geelong = BBox {
            west: 144.35,
            south: -38.15,
            east: 144.36,
            north: -38.14,
        };
        assert_ne!(region_for_bbox(melbourne), region_for_bbox(geelong));
    }
}
