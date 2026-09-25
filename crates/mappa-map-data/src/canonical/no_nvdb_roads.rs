//! Build-only reader for a captured NVDB V4 municipality road-link snapshot.

use super::*;
use proj4rs::{proj::Proj, transform::transform};
use serde_json::Value;
use std::collections::BTreeMap;

const UTM33: &str = "+proj=utm +zone=33 +ellps=GRS80 +units=m +no_defs";
const WGS84: &str = "+proj=longlat +datum=WGS84 +no_defs";
const SEGMENTED_ENDPOINT: &str =
    "https://nvdbapiles.atlas.vegvesen.no/vegnett/api/v4/veglenkesekvenser/segmentert";

#[derive(Deserialize)]
struct Snapshot {
    source_url: String,
    license_url: String,
    municipality: u32,
    expected_objects: usize,
    pages: Vec<Page>,
}

#[derive(Deserialize)]
struct Page {
    file: String,
    sha256: String,
    objects: usize,
}

#[derive(Clone)]
struct SegmentMeta {
    category: String,
    name: Option<String>,
    ambiguous_name: bool,
}

impl SegmentMeta {
    fn merge(&mut self, category: &str, name: Option<String>) -> Result<(), CanonicalError> {
        if self.category != category {
            return Err(invalid("conflicting NVDB road category within a link"));
        }
        if !self.ambiguous_name {
            match (&self.name, name) {
                (Some(existing), Some(next)) if *existing != next => {
                    self.name = None;
                    self.ambiguous_name = true;
                }
                (None, Some(next)) => self.name = Some(next),
                _ => {}
            }
        }
        Ok(())
    }
}

fn vehicle_style(category: &str) -> Option<(FeatureKind, u16, u8)> {
    match category {
        "E" | "R" => Some((FeatureKind::RoadPrimary, 800, 10)),
        "F" => Some((FeatureKind::RoadSecondary, 500, 11)),
        "K" | "P" => Some((FeatureKind::RoadResidential, 200, 12)),
        "S" => Some((FeatureKind::RoadResidential, 100, 13)),
        _ => None,
    }
}

fn segment_metadata(
    source: &SourceRecord,
    municipality: u32,
) -> Result<BTreeMap<(u64, u64), SegmentMeta>, CanonicalError> {
    let path = source
        .upstream_file
        .as_deref()
        .ok_or_else(|| invalid("enriched NVDB source lacks segmented snapshot"))?;
    let expected_hash = source
        .upstream_sha256
        .as_deref()
        .ok_or_else(|| invalid("enriched NVDB source lacks segmented checksum"))?;
    let path = Path::new(path);
    if source_hash(path)? != expected_hash.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(source.id.clone()));
    }
    let snapshot: Snapshot = toml::from_str(&fs::read_to_string(path)?)?;
    if snapshot.source_url != SEGMENTED_ENDPOINT
        || snapshot.license_url != source.license_url
        || snapshot.municipality != municipality
        || snapshot.expected_objects == 0
        || snapshot.pages.is_empty()
    {
        return Err(invalid("invalid NVDB segmented enrichment snapshot"));
    }
    let mut seen = BTreeSet::new();
    let mut output: BTreeMap<(u64, u64), SegmentMeta> = BTreeMap::new();
    for page in &snapshot.pages {
        let relative = Path::new(&page.file);
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            return Err(invalid("invalid NVDB enrichment page path"));
        }
        let bytes = fs::read(path.parent().expect("snapshot has parent").join(relative))?;
        if format!("{:x}", Sha256::digest(&bytes)) != page.sha256.to_lowercase() {
            return Err(CanonicalError::SourceChecksum(source.id.clone()));
        }
        let value: Value =
            serde_json::from_slice(&bytes).map_err(|error| invalid(error.to_string()))?;
        let objects = value
            .get("objekter")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("NVDB enrichment page has no objects"))?;
        if objects.len() != page.objects
            || value.pointer("/metadata/antall").and_then(Value::as_u64)
                != Some(snapshot.expected_objects as u64)
        {
            return Err(invalid("NVDB enrichment page count mismatch"));
        }
        for object in objects {
            let reference = object
                .get("referanse")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("NVDB enrichment object lacks reference"))?;
            if !seen.insert(reference.to_owned()) {
                return Err(invalid("duplicate NVDB enrichment reference"));
            }
            if object.get("type").and_then(Value::as_str) != Some("HOVED")
                || !matches!(
                    object.get("typeVeg_sosi").and_then(Value::as_str),
                    Some("enkelBilveg" | "kanalisertVeg" | "rundkjøring" | "rampe" | "gatetun")
                )
                || object
                    .pointer("/vegsystemreferanse/vegsystem/fase")
                    .and_then(Value::as_str)
                    != Some("V")
            {
                continue;
            }
            let Some(category) = object
                .pointer("/vegsystemreferanse/vegsystem/vegkategori")
                .and_then(Value::as_str)
                .filter(|category| vehicle_style(category).is_some())
            else {
                continue;
            };
            let sequence = object
                .get("veglenkesekvensid")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid("NVDB enrichment lacks sequence ID"))?;
            let number = object
                .get("veglenkenummer")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid("NVDB enrichment lacks link number"))?;
            let name = object
                .pointer("/adresse/navn")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|name| !name.is_empty() && name.len() <= 128)
                .map(str::to_owned);
            let entry = output
                .entry((sequence, number))
                .or_insert_with(|| SegmentMeta {
                    category: category.to_owned(),
                    name: name.clone(),
                    ambiguous_name: false,
                });
            entry.merge(category, name)?;
        }
    }
    if seen.len() != snapshot.expected_objects {
        return Err(invalid("NVDB enrichment snapshot incomplete"));
    }
    Ok(output)
}

fn invalid(message: impl Into<String>) -> CanonicalError {
    CanonicalError::Feature(message.into())
}

fn parse_wkt(wkt: &str, from: &Proj, to: &Proj) -> Result<Geometry, CanonicalError> {
    let body = wkt
        .strip_prefix("LINESTRING Z (")
        .and_then(|line| line.strip_suffix(')'))
        .ok_or_else(|| invalid("expected NVDB LINESTRING Z geometry"))?;
    let mut points = Vec::new();
    for raw in body.split(',') {
        if points.len() >= 100_000 {
            return Err(invalid("NVDB line exceeds point limit"));
        }
        let parts: Vec<&str> = raw.split_whitespace().collect();
        if parts.len() != 3 {
            return Err(invalid("invalid NVDB WKT vertex"));
        }
        let x: f64 = parts[0]
            .parse()
            .map_err(|_| invalid("invalid NVDB easting"))?;
        let y: f64 = parts[1]
            .parse()
            .map_err(|_| invalid("invalid NVDB northing"))?;
        let height: f64 = parts[2]
            .parse()
            .map_err(|_| invalid("invalid NVDB height"))?;
        if !x.is_finite() || !y.is_finite() || !height.is_finite() {
            return Err(invalid("non-finite NVDB vertex"));
        }
        let mut point = (x, y, 0.0);
        transform(from, to, &mut point).map_err(|error| invalid(error.to_string()))?;
        points.push([point.0.to_degrees(), point.1.to_degrees()]);
    }
    let geometry = Geometry::Line(points);
    geometry.bbox()?;
    Ok(geometry)
}

pub fn adapt_no_nvdb_roads(
    source: &SourceRecord,
) -> Result<(AdaptedFeatures, Vec<RejectedFeature>), CanonicalError> {
    if !matches!(
        source.adapter.as_str(),
        "no-nvdb-v4-road-links" | "no-nvdb-v4-road-links-enriched"
    ) {
        return Err(invalid("wrong Norway NVDB adapter"));
    }
    let snapshot_path = Path::new(&source.file);
    if source_hash(snapshot_path)? != source.sha256.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(source.id.clone()));
    }
    let snapshot: Snapshot = toml::from_str(&fs::read_to_string(snapshot_path)?)?;
    if snapshot.source_url != source.official_url
        || snapshot.license_url != source.license_url
        || snapshot.municipality == 0
        || snapshot.pages.is_empty()
        || snapshot.expected_objects == 0
    {
        return Err(invalid("invalid Norway NVDB snapshot metadata"));
    }
    let enrichment = if source.adapter == "no-nvdb-v4-road-links-enriched" {
        Some(segment_metadata(source, snapshot.municipality)?)
    } else {
        None
    };
    let from = Proj::from_proj_string(UTM33).map_err(|error| invalid(error.to_string()))?;
    let to = Proj::from_proj_string(WGS84).map_err(|error| invalid(error.to_string()))?;
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    let mut sequence_ids = BTreeSet::new();
    let mut object_count = 0;
    for page in &snapshot.pages {
        let relative = Path::new(&page.file);
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            return Err(invalid("invalid NVDB page path"));
        }
        let bytes = fs::read(
            snapshot_path
                .parent()
                .expect("snapshot has parent")
                .join(relative),
        )?;
        if format!("{:x}", Sha256::digest(&bytes)) != page.sha256.to_lowercase() {
            return Err(CanonicalError::SourceChecksum(source.id.clone()));
        }
        let value: Value =
            serde_json::from_slice(&bytes).map_err(|error| invalid(error.to_string()))?;
        let objects = value
            .get("objekter")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("NVDB page has no objects"))?;
        if objects.len() != page.objects
            || value.pointer("/metadata/antall").and_then(Value::as_u64)
                != Some(snapshot.expected_objects as u64)
        {
            return Err(invalid("NVDB snapshot count mismatch"));
        }
        object_count += objects.len();
        for object in objects {
            let sequence = object
                .get("veglenkesekvensid")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid("NVDB object missing sequence ID"))?;
            if !sequence_ids.insert(sequence) {
                return Err(invalid("duplicate NVDB road-link sequence"));
            }
            let links = object
                .get("veglenker")
                .and_then(Value::as_array)
                .ok_or_else(|| invalid("NVDB object missing links"))?;
            let mut link_ids = BTreeSet::new();
            for link in links {
                let number = link
                    .get("veglenkenummer")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| invalid("NVDB link missing number"))?;
                if !link_ids.insert(number) {
                    return Err(invalid("duplicate NVDB link number"));
                }
                let source_feature_id = format!("{sequence}:{number}");
                let road_type = link.get("typeVeg_sosi").and_then(Value::as_str);
                let link_type = link.get("type").and_then(Value::as_str);
                let style = if link_type == Some("HOVED") {
                    match road_type {
                        Some("enkelBilveg" | "gatetun") => {
                            Some((FeatureKind::RoadResidential, 200, 12))
                        }
                        Some("kanalisertVeg" | "rundkjøring" | "rampe") => {
                            Some((FeatureKind::RoadSecondary, 450, 11))
                        }
                        _ => None,
                    }
                } else {
                    None
                };
                let Some((mut kind, mut importance, mut min_zoom)) = style else {
                    rejected.push(RejectedFeature {
                        source_id: source.id.clone(),
                        source_feature_id,
                        reason: format!("not a physical vehicle road: type={link_type:?}, typeVeg_sosi={road_type:?}"),
                    });
                    continue;
                };
                let enriched = enrichment
                    .as_ref()
                    .and_then(|map| map.get(&(sequence, number)));
                if let Some(info) = enriched {
                    (kind, importance, min_zoom) =
                        vehicle_style(&info.category).expect("validated enrichment category");
                }
                let geometry_record = link
                    .get("geometri")
                    .ok_or_else(|| invalid("NVDB link missing geometry object"))?;
                let geometry = if geometry_record.get("srid").and_then(Value::as_u64) == Some(5973)
                {
                    geometry_record
                        .get("wkt")
                        .and_then(Value::as_str)
                        .ok_or_else(|| invalid("NVDB link missing WKT"))
                        .and_then(|wkt| parse_wkt(wkt, &from, &to))
                } else {
                    Err(invalid("unexpected NVDB geometry SRID"))
                };
                let geometry = match geometry {
                    Ok(geometry) => geometry,
                    Err(error) => {
                        rejected.push(RejectedFeature {
                            source_id: source.id.clone(),
                            source_feature_id,
                            reason: error.to_string(),
                        });
                        continue;
                    }
                };
                let bbox = geometry.bbox()?;
                let id = stable_id(&source.id, &source_feature_id);
                let raw = serde_json::to_vec(link).map_err(|error| invalid(error.to_string()))?;
                let mut hash = Sha256::new();
                hash.update(&raw);
                if let Some(info) = enriched {
                    hash.update(info.category.as_bytes());
                    hash.update([0]);
                    hash.update(info.name.as_deref().unwrap_or("").as_bytes());
                }
                accepted.push((
                    CanonicalFeature {
                        id,
                        kind,
                        geometry,
                        bbox,
                        importance,
                        min_zoom,
                        max_zoom: 15,
                        name: enriched.and_then(|info| info.name.clone()),
                        revision: 1,
                    },
                    Provenance {
                        feature_id: id,
                        source_id: source.id.clone(),
                        source_feature_id,
                        source_revision: source.source_version.clone(),
                        adapter_version: source.adapter_version,
                        source_feature_sha256: hash.finalize().into(),
                    },
                ));
            }
        }
    }
    if object_count != snapshot.expected_objects {
        return Err(invalid("NVDB snapshot incomplete"));
    }
    accepted.sort_by_key(|(feature, _)| feature.id);
    Ok((accepted, rejected))
}

pub fn adapt_no_nvdb_segmented_roads(
    source: &SourceRecord,
) -> Result<(AdaptedFeatures, Vec<RejectedFeature>), CanonicalError> {
    if source.adapter != "no-nvdb-v4-segmented-road-links" {
        return Err(invalid("wrong Norway NVDB segmented adapter"));
    }
    let snapshot_path = Path::new(&source.file);
    if source_hash(snapshot_path)? != source.sha256.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(source.id.clone()));
    }
    let snapshot: Snapshot = toml::from_str(&fs::read_to_string(snapshot_path)?)?;
    if snapshot.source_url != source.official_url
        || snapshot.license_url != source.license_url
        || snapshot.municipality == 0
        || snapshot.pages.is_empty()
        || snapshot.expected_objects == 0
    {
        return Err(invalid("invalid Norway NVDB segmented snapshot metadata"));
    }
    let from = Proj::from_proj_string(UTM33).map_err(|error| invalid(error.to_string()))?;
    let to = Proj::from_proj_string(WGS84).map_err(|error| invalid(error.to_string()))?;
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    let mut references = BTreeSet::new();
    let mut object_count = 0;
    for page in &snapshot.pages {
        let relative = Path::new(&page.file);
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            return Err(invalid("invalid NVDB segmented page path"));
        }
        let bytes = fs::read(
            snapshot_path
                .parent()
                .expect("snapshot has parent")
                .join(relative),
        )?;
        if format!("{:x}", Sha256::digest(&bytes)) != page.sha256.to_lowercase() {
            return Err(CanonicalError::SourceChecksum(source.id.clone()));
        }
        let value: Value =
            serde_json::from_slice(&bytes).map_err(|error| invalid(error.to_string()))?;
        let objects = value
            .get("objekter")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("NVDB segmented page has no objects"))?;
        if objects.len() != page.objects
            || value.pointer("/metadata/antall").and_then(Value::as_u64)
                != Some(snapshot.expected_objects as u64)
        {
            return Err(invalid("NVDB segmented snapshot count mismatch"));
        }
        object_count += objects.len();
        for object in objects {
            let reference = object
                .get("referanse")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("NVDB segment missing reference"))?;
            if !references.insert(reference.to_owned()) {
                return Err(invalid("duplicate NVDB segment reference"));
            }
            let source_feature_id = reference.to_owned();
            let road_type = object.get("typeVeg_sosi").and_then(Value::as_str);
            let link_type = object.get("type").and_then(Value::as_str);
            let category = object
                .pointer("/vegsystemreferanse/vegsystem/vegkategori")
                .and_then(Value::as_str);
            let phase = object
                .pointer("/vegsystemreferanse/vegsystem/fase")
                .and_then(Value::as_str);
            let vehicle_road = matches!(
                road_type,
                Some("enkelBilveg" | "kanalisertVeg" | "rundkjøring" | "rampe" | "gatetun")
            );
            let style = if link_type == Some("HOVED") && vehicle_road && phase == Some("V") {
                match category {
                    Some("E" | "R") => Some((FeatureKind::RoadPrimary, 800, 10)),
                    Some("F") => Some((FeatureKind::RoadSecondary, 500, 11)),
                    Some("K" | "P") => Some((FeatureKind::RoadResidential, 200, 12)),
                    Some("S") => Some((FeatureKind::RoadResidential, 100, 13)),
                    _ => None,
                }
            } else {
                None
            };
            let Some((kind, importance, min_zoom)) = style else {
                rejected.push(RejectedFeature {
                    source_id: source.id.clone(),
                    source_feature_id,
                    reason: format!("not a classified existing physical vehicle road: type={link_type:?}, typeVeg_sosi={road_type:?}, category={category:?}, phase={phase:?}"),
                });
                continue;
            };
            let geometry_record = object
                .get("geometri")
                .ok_or_else(|| invalid("NVDB segment missing geometry object"))?;
            let geometry = if geometry_record.get("srid").and_then(Value::as_u64) == Some(5973) {
                geometry_record
                    .get("wkt")
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid("NVDB segment missing WKT"))
                    .and_then(|wkt| parse_wkt(wkt, &from, &to))
            } else {
                Err(invalid("unexpected NVDB segment SRID"))
            };
            let geometry = match geometry {
                Ok(geometry) => geometry,
                Err(error) => {
                    rejected.push(RejectedFeature {
                        source_id: source.id.clone(),
                        source_feature_id,
                        reason: error.to_string(),
                    });
                    continue;
                }
            };
            let bbox = geometry.bbox()?;
            let name = object
                .pointer("/adresse/navn")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|name| !name.is_empty() && name.len() <= 128)
                .map(str::to_owned);
            let id = stable_id(&source.id, &source_feature_id);
            let raw = serde_json::to_vec(object).map_err(|error| invalid(error.to_string()))?;
            accepted.push((
                CanonicalFeature {
                    id,
                    kind,
                    geometry,
                    bbox,
                    importance,
                    min_zoom,
                    max_zoom: 15,
                    name,
                    revision: 1,
                },
                Provenance {
                    feature_id: id,
                    source_id: source.id.clone(),
                    source_feature_id,
                    source_revision: source.source_version.clone(),
                    adapter_version: source.adapter_version,
                    source_feature_sha256: Sha256::digest(&raw).into(),
                },
            ));
        }
    }
    if object_count != snapshot.expected_objects {
        return Err(invalid("NVDB segmented snapshot incomplete"));
    }
    accepted.sort_by_key(|(feature, _)| feature.id);
    Ok((accepted, rejected))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utm33_matches_independent_proj_control_point() {
        let from = Proj::from_proj_string(UTM33).unwrap();
        let to = Proj::from_proj_string(WGS84).unwrap();
        let Geometry::Line(points) = parse_wkt(
            "LINESTRING Z (266994.82 7044204.36 3.643,266993.94 7044203.88 3.643)",
            &from,
            &to,
        )
        .unwrap() else {
            panic!("expected line");
        };
        assert!((points[0][0] - 10.3248025706).abs() < 0.000001);
        assert!((points[0][1] - 63.4496897360).abs() < 0.000001);
    }

    #[test]
    fn conflicting_segment_names_do_not_invent_one_link_name() {
        let mut info = SegmentMeta {
            category: "K".into(),
            name: Some("Kong Inges gate".into()),
            ambiguous_name: false,
        };
        info.merge("K", Some("Paul Fjermstads veg".into())).unwrap();
        info.merge("K", Some("Kong Inges gate".into())).unwrap();
        assert!(info.name.is_none());
        assert!(info.ambiguous_name);
        assert!(info.merge("F", None).is_err());
    }
}
