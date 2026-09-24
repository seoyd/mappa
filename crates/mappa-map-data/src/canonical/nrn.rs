//! Build-only Canadian National Road Network Shapefile adapter.

use super::*;

/// Retain NAD83(CSRS) geographic degrees numerically. Independent WGS84
/// accuracy checks remain required before making a metre-level claim.
pub fn adapt_ca_nrn_roads(
    source: &SourceRecord,
    province: &str,
) -> Result<(AdaptedFeatures, Vec<RejectedFeature>), CanonicalError> {
    if source.adapter != "ca-nrn-roadseg"
        || province.len() != 2
        || !province.bytes().all(|byte| byte.is_ascii_uppercase())
    {
        return Err(CanonicalError::Feature(
            "invalid NRN adapter or province".into(),
        ));
    }
    if source_hash(Path::new(&source.file))? != source.sha256.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(source.id.clone()));
    }
    let mut archive = zip::ZipArchive::new(File::open(&source.file)?)
        .map_err(|error| CanonicalError::Feature(error.to_string()))?;
    let version = source.source_version.replace('.', "_");
    let expected_suffix =
        format!("/NRN_{province}_{version}_SHAPE_en/NRN_{province}_{version}_ROADSEG.shp");
    let members: Vec<_> = archive
        .file_names()
        .filter(|name| name.ends_with(&expected_suffix))
        .map(str::to_owned)
        .collect();
    let [shp_name] = members.as_slice() else {
        return Err(CanonicalError::Feature(
            "expected exactly one versioned NRN ROADSEG Shapefile".into(),
        ));
    };
    let stem = shp_name.trim_end_matches(".shp");
    let read_member = |archive: &mut zip::ZipArchive<File>, extension: &str| {
        let mut member = archive
            .by_name(&format!("{stem}.{extension}"))
            .map_err(|error| CanonicalError::Feature(error.to_string()))?;
        if member.size() > 512 * 1024 * 1024 {
            return Err(CanonicalError::Feature("NRN member exceeds 512 MiB".into()));
        }
        let mut bytes = Vec::with_capacity(member.size() as usize);
        member.read_to_end(&mut bytes)?;
        Ok::<_, CanonicalError>(bytes)
    };
    let prj = read_member(&mut archive, "prj")?;
    if !std::str::from_utf8(&prj).is_ok_and(|text| {
        text.contains("GCS_North_American_1983_CSRS") && text.contains("UNIT[\"Degree\"")
    }) {
        return Err(CanonicalError::Feature("unexpected NRN source CRS".into()));
    }
    let shp = read_member(&mut archive, "shp")?;
    let dbf = read_member(&mut archive, "dbf")?;
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
        // NID is a network element identifier and repeats in this release;
        // ROADSEGID is unique within the province's Road Segment table.
        let source_feature_id = match attributes.get("ROADSEGID") {
            Some(shapefile::dbase::FieldValue::Numeric(Some(value)))
                if value.is_finite() && *value > 0.0 && value.fract() == 0.0 && *value < 9e15 =>
            {
                format!("{value:.0}")
            }
            _ => return Err(CanonicalError::Feature("invalid NRN ROADSEGID".into())),
        };
        if !raw_ids.insert(source_feature_id.clone()) {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id,
                reason: "duplicate ROADSEGID in province ROADSEG".into(),
            });
            continue;
        }
        let road_class = string_field("ROADCLASS");
        let (kind, importance, min_zoom) = match road_class {
            Some("Freeway") => (FeatureKind::RoadPrimary, 800, 10),
            Some("Expressway / Highway") => (FeatureKind::RoadPrimary, 750, 10),
            Some("Arterial") => (FeatureKind::RoadPrimary, 700, 10),
            Some("Ramp") => (FeatureKind::RoadPrimary, 650, 11),
            Some("Collector") => (FeatureKind::RoadSecondary, 500, 11),
            Some(
                "Local / Street"
                | "Local / Strata"
                | "Local / Unknown"
                | "Resource / Recreation"
                | "Service Lane"
                | "Alleyway / Lane"
                | "Rapid Transit"
                | "Winter"
                | "Unknown",
            ) => (FeatureKind::RoadResidential, 200, 12),
            value => {
                rejected.push(RejectedFeature {
                    source_id: source.id.clone(),
                    source_feature_id,
                    reason: format!("unmapped NRN road class: {value:?}"),
                });
                continue;
            }
        };
        let name = ["L_STNAME_C", "R_STNAME_C", "STRUNAMEEN", "RTENAME1EN"]
            .into_iter()
            .filter_map(string_field)
            .find(|name| !name.is_empty() && *name != "Unknown" && name.len() <= 128)
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
            hash.update(road_class.unwrap_or_default().as_bytes());
            hash.update(name.as_deref().unwrap_or_default().as_bytes());
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
