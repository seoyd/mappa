//! Build-only adapter for Victoria's DTP Managed Roads GeoJSON snapshot.

use super::*;

pub fn adapt_au_vic_dtp_roads(
    source: &SourceRecord,
) -> Result<(AdaptedFeatures, Vec<RejectedFeature>), CanonicalError> {
    if source.adapter != "au-vic-dtp-managed-roads" {
        return Err(CanonicalError::Feature("wrong Victoria DTP adapter".into()));
    }
    if source_hash(Path::new(&source.file))? != source.sha256.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(source.id.clone()));
    }
    let collection: geojson::FeatureCollection =
        serde_json::from_reader(BufReader::new(File::open(&source.file)?))
            .map_err(|error| CanonicalError::Feature(error.to_string()))?;
    let mut output = Vec::with_capacity(collection.features.len());
    let mut rejected = Vec::new();
    let mut raw_ids = BTreeSet::new();
    for raw in collection.features {
        let source_feature_id = raw
            .property("OBJECTID")
            .and_then(|value| value.as_u64())
            .filter(|id| *id > 0)
            .ok_or_else(|| CanonicalError::Feature("missing Victoria OBJECTID".into()))?
            .to_string();
        if !raw_ids.insert(source_feature_id.clone()) {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id,
                reason: "duplicate OBJECTID".into(),
            });
            continue;
        }
        let class = raw.property("CLASSN").and_then(|value| value.as_str());
        // The portal names this field a legal road classification but gives no
        // code legend. These are Mappa display rules, not legal interpretations;
        // unhandled codes remain in the rejection record.
        let (kind, importance, min_zoom) = match class {
            Some("FW" | "HW") => (FeatureKind::RoadPrimary, 700, 10),
            Some("MR") => (FeatureKind::RoadSecondary, 450, 11),
            Some("TR" | "FR") => (FeatureKind::RoadResidential, 180, 12),
            other => {
                rejected.push(RejectedFeature {
                    source_id: source.id.clone(),
                    source_feature_id,
                    reason: format!("unmapped CLASSN code: {other:?}"),
                });
                continue;
            }
        };
        let name = raw
            .property("RD_NAME")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty() && value.len() <= 128)
            .map(str::to_owned);
        let Some(geojson::Geometry {
            value: GeometryValue::LineString { coordinates },
            ..
        }) = raw.geometry.as_ref()
        else {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id,
                reason: "expected LineString".into(),
            });
            continue;
        };
        let mut points = Vec::with_capacity(coordinates.len());
        let mut valid = true;
        for position in coordinates {
            let [lon, lat] = position.as_slice() else {
                valid = false;
                break;
            };
            points.push([*lon, *lat]);
        }
        if !valid {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id,
                reason: "non-2D coordinate".into(),
            });
            continue;
        }
        let geometry = Geometry::Line(points);
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
        let id = stable_id(&source.id, &source_feature_id);
        let mut hash = Sha256::new();
        hash.update(source_feature_id.as_bytes());
        hash.update(class.unwrap_or_default().as_bytes());
        hash.update(name.as_deref().unwrap_or_default().as_bytes());
        if let Geometry::Line(ref points) = geometry {
            for [lon, lat] in points {
                hash.update(lon.to_le_bytes());
                hash.update(lat.to_le_bytes());
            }
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
                name,
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
    output.sort_by_key(|(feature, _)| feature.id);
    Ok((output, rejected))
}
