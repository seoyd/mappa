//! Official IGN BD TOPO roads, surface water, and real passenger stations.

use super::*;

fn member_to_file(
    archive: &mut zip::ZipArchive<File>,
    stem: &str,
    extension: &str,
    destination: &Path,
) -> Result<(), CanonicalError> {
    let name = format!("{stem}.{extension}");
    let mut member = archive
        .by_name(&name)
        .map_err(|error| CanonicalError::Feature(error.to_string()))?;
    if member.size() > 4 * 1024 * 1024 * 1024 {
        return Err(CanonicalError::Feature(format!("IGN {name} exceeds 4 GiB")));
    }
    let mut output = File::create(destination)?;
    if std::io::copy(&mut member, &mut output)? != member.size() {
        return Err(CanonicalError::Feature(format!("short IGN {name}")));
    }
    Ok(())
}

pub fn adapt_ign_bdtopo_roads(
    source: &SourceRecord,
    region: BBox,
) -> Result<(AdaptedFeatures, Vec<RejectedFeature>), CanonicalError> {
    if source.adapter != "ign-bdtopo-road-segment" {
        return Err(CanonicalError::Feature("wrong IGN road adapter".into()));
    }
    if source_hash(Path::new(&source.file))? != source.sha256.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(source.id.clone()));
    }
    let (Some(upstream_file), Some(upstream_hash)) =
        (&source.upstream_file, &source.upstream_sha256)
    else {
        return Err(CanonicalError::Feature(
            "IGN published 7z provenance missing".into(),
        ));
    };
    if source_hash(Path::new(upstream_file))? != upstream_hash.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(format!("{} 7z", source.id)));
    }
    let mut archive = zip::ZipArchive::new(File::open(&source.file)?)
        .map_err(|error| CanonicalError::Feature(error.to_string()))?;
    let temporary = tempfile::tempdir()?;
    for extension in ["shp", "dbf", "prj"] {
        member_to_file(
            &mut archive,
            "road",
            extension,
            &temporary.path().join(format!("road.{extension}")),
        )?;
    }
    let prj = fs::read_to_string(temporary.path().join("road.prj"))?;
    if !prj.contains("Lambert_Conformal_Conic")
        || !prj.contains("RGF_1993")
        || !prj.contains("700000")
        || !prj.contains("6600000")
    {
        return Err(CanonicalError::Feature(
            "unexpected IGN Lambert-93 CRS".into(),
        ));
    }
    let shape_reader = shapefile::ShapeReader::new(File::open(temporary.path().join("road.shp"))?)?;
    let attribute_reader =
        shapefile::dbase::Reader::new(File::open(temporary.path().join("road.dbf"))?)?;
    let mut reader = shapefile::Reader::new(shape_reader, attribute_reader);
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    let mut raw_ids = BTreeSet::new();
    for (index, record) in reader.iter_shapes_and_records().enumerate() {
        let (shape, attributes) = record?;
        let string_field = |field: &str| -> Option<&str> {
            match attributes.get(field) {
                Some(shapefile::dbase::FieldValue::Character(Some(value))) => Some(value.trim()),
                _ => None,
            }
        };
        let id = string_field("ID")
            .filter(|value| !value.is_empty())
            .ok_or_else(|| CanonicalError::Feature("missing IGN road ID".into()))?
            .to_owned();
        if !raw_ids.insert(id.clone()) {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id: id,
                reason: "duplicate IGN road ID".into(),
            });
            continue;
        }
        let real_geometry = match attributes.get("FICTIF") {
            Some(shapefile::dbase::FieldValue::Logical(Some(false))) => true,
            Some(shapefile::dbase::FieldValue::Character(Some(value))) => value.trim() == "Non",
            _ => false,
        };
        let reject_reason = if string_field("ETAT") != Some("En service") {
            Some(format!("road not in service: {:?}", string_field("ETAT")))
        } else if !real_geometry {
            Some(format!(
                "fictitious or unknown geometry: {:?}",
                attributes.get("FICTIF")
            ))
        } else if !matches!(
            string_field("ACCES_VL"),
            Some("Libre" | "A péage" | "Restreint aux ayants droit")
        ) {
            Some(format!(
                "vehicle access excluded: {:?}",
                string_field("ACCES_VL")
            ))
        } else {
            None
        };
        if let Some(reason) = reject_reason {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id: id,
                reason,
            });
            continue;
        }
        let nature = string_field("NATURE");
        let importance = string_field("IMPORTANCE");
        let road_kind = match (nature, importance) {
            (Some("Type autoroutier" | "Route à 2 chaussées"), _) => {
                Some((FeatureKind::RoadPrimary, 750, 10))
            }
            (Some("Bretelle"), _) => Some((FeatureKind::RoadPrimary, 600, 11)),
            (Some("Route à 1 chaussée" | "Rond-point"), Some("1" | "2" | "3")) => {
                Some((FeatureKind::RoadPrimary, 650, 10))
            }
            (Some("Route à 1 chaussée" | "Rond-point"), Some("4")) => {
                Some((FeatureKind::RoadSecondary, 450, 11))
            }
            (
                Some("Route à 1 chaussée" | "Rond-point" | "Route empierrée" | "Chemin"),
                Some("5" | "6"),
            ) => Some((FeatureKind::RoadResidential, 200, 12)),
            _ => None,
        };
        let Some((kind, priority, min_zoom)) = road_kind else {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id: id,
                reason: format!("unmapped IGN road nature={nature:?} importance={importance:?}"),
            });
            continue;
        };
        let name = ["NOM_COLL_G", "NOM_COLL_D"]
            .into_iter()
            .filter_map(string_field)
            .find(|value| !value.is_empty() && value.len() <= 128)
            .map(str::to_owned);
        let shapefile::Shape::PolylineZ(line) = shape else {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id: id,
                reason: format!("row {}: expected PolylineZ", index + 1),
            });
            continue;
        };
        for (part_index, part) in line.parts().iter().enumerate() {
            let part_id = format!("{id}:{part_index}");
            let mut points = Vec::with_capacity(part.len());
            let mut hash = Sha256::new();
            hash.update(part_id.as_bytes());
            hash.update(nature.unwrap_or_default().as_bytes());
            hash.update(importance.unwrap_or_default().as_bytes());
            let mut transform_error = None;
            for point in part {
                hash.update(point.x.to_le_bytes());
                hash.update(point.y.to_le_bytes());
                hash.update(point.z.to_le_bytes());
                match inverse_lambert93(point.x, point.y) {
                    Ok(lonlat) => points.push(lonlat),
                    Err(error) => {
                        transform_error = Some(error.to_string());
                        break;
                    }
                }
            }
            if let Some(error) = transform_error {
                rejected.push(RejectedFeature {
                    source_id: source.id.clone(),
                    source_feature_id: part_id,
                    reason: error,
                });
                continue;
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
            if !bbox.intersects(region) {
                continue;
            }
            let feature_id = stable_id(&source.id, &part_id);
            accepted.push((
                CanonicalFeature {
                    id: feature_id,
                    kind,
                    geometry,
                    bbox,
                    importance: priority,
                    min_zoom,
                    max_zoom: 15,
                    name: name.clone(),
                    revision: 1,
                },
                Provenance {
                    feature_id,
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

pub fn adapt_ign_bdtopo_water(
    source: &SourceRecord,
    region: BBox,
) -> Result<(AdaptedFeatures, Vec<RejectedFeature>), CanonicalError> {
    if source.adapter != "ign-bdtopo-surface-water" {
        return Err(CanonicalError::Feature("wrong IGN water adapter".into()));
    }
    if source_hash(Path::new(&source.file))? != source.sha256.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(source.id.clone()));
    }
    let (Some(upstream_file), Some(upstream_hash)) =
        (&source.upstream_file, &source.upstream_sha256)
    else {
        return Err(CanonicalError::Feature(
            "IGN published 7z provenance missing".into(),
        ));
    };
    if source_hash(Path::new(upstream_file))? != upstream_hash.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(format!("{} 7z", source.id)));
    }
    let mut archive = zip::ZipArchive::new(File::open(&source.file)?)
        .map_err(|error| CanonicalError::Feature(error.to_string()))?;
    let temporary = tempfile::tempdir()?;
    for extension in ["shp", "dbf", "prj"] {
        member_to_file(
            &mut archive,
            "water",
            extension,
            &temporary.path().join(format!("water.{extension}")),
        )?;
    }
    let prj = fs::read_to_string(temporary.path().join("water.prj"))?;
    if !prj.contains("Lambert_Conformal_Conic")
        || !prj.contains("RGF_1993")
        || !prj.contains("700000")
        || !prj.contains("6600000")
    {
        return Err(CanonicalError::Feature(
            "unexpected IGN Lambert-93 CRS".into(),
        ));
    }
    let shape_reader =
        shapefile::ShapeReader::new(File::open(temporary.path().join("water.shp"))?)?;
    let attribute_reader =
        shapefile::dbase::Reader::new(File::open(temporary.path().join("water.dbf"))?)?;
    let mut reader = shapefile::Reader::new(shape_reader, attribute_reader);
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    let mut raw_ids = BTreeSet::new();
    for (index, record) in reader.iter_shapes_and_records().enumerate() {
        let (shape, attributes) = record?;
        let string_field = |field: &str| -> Option<&str> {
            match attributes.get(field) {
                Some(shapefile::dbase::FieldValue::Character(Some(value))) => Some(value.trim()),
                _ => None,
            }
        };
        let id = string_field("ID")
            .filter(|value| !value.is_empty())
            .ok_or_else(|| CanonicalError::Feature("missing IGN water ID".into()))?
            .to_owned();
        if !raw_ids.insert(id.clone()) {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id: id,
                reason: "duplicate IGN water ID".into(),
            });
            continue;
        }
        if string_field("ETAT") != Some("En service") {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id: id,
                reason: format!("water not in service: {:?}", string_field("ETAT")),
            });
            continue;
        }
        if string_field("PERSISTANC") != Some("Permanent") {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id: id,
                reason: format!("water not permanent: {:?}", string_field("PERSISTANC")),
            });
            continue;
        }
        let shapefile::Shape::PolygonZ(polygon) = shape else {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id: id,
                reason: format!("row {}: expected PolygonZ", index + 1),
            });
            continue;
        };
        let mut groups: Vec<Vec<Vec<[f64; 2]>>> = Vec::new();
        let mut holes = Vec::new();
        for ring in polygon.rings() {
            let mut points = Vec::with_capacity(ring.points().len());
            let mut invalid = None;
            for point in ring.points() {
                match inverse_lambert93(point.x, point.y) {
                    Ok(lonlat) => points.push(lonlat),
                    Err(error) => {
                        invalid = Some(error.to_string());
                        break;
                    }
                }
            }
            if let Some(reason) = invalid {
                rejected.push(RejectedFeature {
                    source_id: source.id.clone(),
                    source_feature_id: id.clone(),
                    reason,
                });
                groups.clear();
                holes.clear();
                break;
            }
            match ring {
                shapefile::PolygonRing::Outer(_) => groups.push(vec![points]),
                shapefile::PolygonRing::Inner(_) => holes.push(points),
            }
        }
        if groups.is_empty() {
            if !rejected.iter().any(|row| row.source_feature_id == id) {
                rejected.push(RejectedFeature {
                    source_id: source.id.clone(),
                    source_feature_id: id,
                    reason: "water polygon has no exterior ring".into(),
                });
            }
            continue;
        }
        let mut unpaired = false;
        for hole in holes {
            let Some(first) = hole.first() else {
                unpaired = true;
                break;
            };
            let point = Point::new(first[0], first[1]);
            let matches = groups
                .iter()
                .enumerate()
                .filter(|(_, group)| {
                    let exterior: LineString<f64> = group[0]
                        .iter()
                        .map(|p| Coord { x: p[0], y: p[1] })
                        .collect();
                    Polygon::new(exterior, vec![]).contains(&point)
                })
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            if let [group_index] = matches.as_slice() {
                groups[*group_index].push(hole);
            } else {
                unpaired = true;
                break;
            }
        }
        if unpaired {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id: id,
                reason: "water polygon has unpaired or ambiguous rings".into(),
            });
            continue;
        }
        let name = ["NOM_P_EAU", "NOM_C_EAU"]
            .into_iter()
            .filter_map(string_field)
            .find(|value| !value.is_empty() && value.len() <= 128)
            .map(str::to_owned);
        for (part_index, rings) in groups.into_iter().enumerate() {
            let part_id = format!("{id}:{part_index}");
            let geometry = Geometry::Polygon(rings);
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
            if !bbox.intersects(region) {
                continue;
            }
            let feature_id = stable_id(&source.id, &part_id);
            let mut hash = Sha256::new();
            hash.update(part_id.as_bytes());
            hash.update(string_field("NATURE").unwrap_or_default().as_bytes());
            hash.update(string_field("PERSISTANC").unwrap_or_default().as_bytes());
            if let Geometry::Polygon(rings) = &geometry {
                for ring in rings {
                    for [lon, lat] in ring {
                        hash.update(lon.to_le_bytes());
                        hash.update(lat.to_le_bytes());
                    }
                }
            }
            accepted.push((
                CanonicalFeature {
                    id: feature_id,
                    kind: FeatureKind::Water,
                    geometry,
                    bbox,
                    importance: 400,
                    min_zoom: 10,
                    max_zoom: 15,
                    name: name.clone(),
                    revision: 1,
                },
                Provenance {
                    feature_id,
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

pub fn adapt_ign_bdtopo_stations(
    source: &SourceRecord,
    region: BBox,
) -> Result<(AdaptedFeatures, Vec<RejectedFeature>), CanonicalError> {
    if source.adapter != "ign-bdtopo-passenger-station" {
        return Err(CanonicalError::Feature("wrong IGN station adapter".into()));
    }
    if source_hash(Path::new(&source.file))? != source.sha256.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(source.id.clone()));
    }
    let (Some(upstream_file), Some(upstream_hash)) =
        (&source.upstream_file, &source.upstream_sha256)
    else {
        return Err(CanonicalError::Feature(
            "IGN published 7z provenance missing".into(),
        ));
    };
    if source_hash(Path::new(upstream_file))? != upstream_hash.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(format!("{} 7z", source.id)));
    }
    let mut archive = zip::ZipArchive::new(File::open(&source.file)?)
        .map_err(|error| CanonicalError::Feature(error.to_string()))?;
    let temporary = tempfile::tempdir()?;
    for extension in ["shp", "dbf", "prj"] {
        member_to_file(
            &mut archive,
            "transport",
            extension,
            &temporary.path().join(format!("transport.{extension}")),
        )?;
    }
    let prj = fs::read_to_string(temporary.path().join("transport.prj"))?;
    if !prj.contains("Lambert_Conformal_Conic")
        || !prj.contains("RGF_1993")
        || !prj.contains("700000")
        || !prj.contains("6600000")
    {
        return Err(CanonicalError::Feature(
            "unexpected IGN Lambert-93 CRS".into(),
        ));
    }
    let shape_reader =
        shapefile::ShapeReader::new(File::open(temporary.path().join("transport.shp"))?)?;
    let attribute_reader =
        shapefile::dbase::Reader::new(File::open(temporary.path().join("transport.dbf"))?)?;
    let mut reader = shapefile::Reader::new(shape_reader, attribute_reader);
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    let mut raw_ids = BTreeSet::new();
    for (index, record) in reader.iter_shapes_and_records().enumerate() {
        let (shape, attributes) = record?;
        let string_field = |field: &str| -> Option<&str> {
            match attributes.get(field) {
                Some(shapefile::dbase::FieldValue::Character(Some(value))) => Some(value.trim()),
                _ => None,
            }
        };
        let id = string_field("ID")
            .filter(|value| !value.is_empty())
            .ok_or_else(|| CanonicalError::Feature("missing IGN transport ID".into()))?
            .to_owned();
        if !raw_ids.insert(id.clone()) {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id: id,
                reason: "duplicate IGN transport ID".into(),
            });
            continue;
        }
        let nature = string_field("NATURE");
        let station = matches!(
            nature,
            Some(
                "Station de métro"
                    | "Station de tramway"
                    | "Gare voyageurs uniquement"
                    | "Gare voyageurs et fret"
                    | "Gare routière"
                    | "Arrêt voyageurs"
            )
        );
        let name = string_field("TOPONYME")
            .filter(|value| !value.is_empty() && value.len() <= 128)
            .map(str::to_owned);
        let reason = if !station {
            Some("not a passenger station".to_owned())
        } else if string_field("ETAT") != Some("En service") {
            Some(format!(
                "station not in service: {:?}",
                string_field("ETAT")
            ))
        } else if string_field("FICTIF") != Some("Non") {
            Some(format!(
                "station has fictitious geometry: {:?}",
                string_field("FICTIF")
            ))
        } else if name.is_none() {
            Some("station has no published name".to_owned())
        } else {
            None
        };
        if let Some(reason) = reason {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id: id,
                reason,
            });
            continue;
        }
        let shapefile::Shape::PolygonZ(polygon) = shape else {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id: id,
                reason: format!("row {}: expected PolygonZ", index + 1),
            });
            continue;
        };
        let mut outer = Vec::new();
        let mut holes = Vec::new();
        let mut hash = Sha256::new();
        hash.update(id.as_bytes());
        hash.update(nature.unwrap_or_default().as_bytes());
        hash.update(name.as_deref().unwrap_or_default().as_bytes());
        for ring in polygon.rings() {
            let line: LineString<f64> = ring
                .points()
                .iter()
                .map(|point| {
                    hash.update(point.x.to_le_bytes());
                    hash.update(point.y.to_le_bytes());
                    hash.update(point.z.to_le_bytes());
                    Coord {
                        x: point.x,
                        y: point.y,
                    }
                })
                .collect();
            match ring {
                shapefile::PolygonRing::Outer(_) => outer.push(line),
                shapefile::PolygonRing::Inner(_) => holes.push(line),
            }
        }
        let mut groups: Vec<_> = outer.into_iter().map(|line| (line, Vec::new())).collect();
        let mut invalid = groups.is_empty();
        for hole in holes {
            let Some(first) = hole.0.first() else {
                invalid = true;
                break;
            };
            let point = Point::new(first.x, first.y);
            let matches = groups
                .iter()
                .enumerate()
                .filter(|(_, (exterior, _))| {
                    Polygon::new(exterior.clone(), vec![]).contains(&point)
                })
                .map(|(group_index, _)| group_index)
                .collect::<Vec<_>>();
            if let [group_index] = matches.as_slice() {
                groups[*group_index].1.push(hole);
            } else {
                invalid = true;
                break;
            }
        }
        if invalid {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id: id,
                reason: "station has ambiguous polygon rings".into(),
            });
            continue;
        }
        let polygons = MultiPolygon(
            groups
                .into_iter()
                .map(|(exterior, interiors)| Polygon::new(exterior, interiors))
                .collect(),
        );
        if polygons.check_validation().is_err() {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id: id,
                reason: "station has invalid polygon topology".into(),
            });
            continue;
        }
        let Some(point) = polygons.interior_point() else {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id: id,
                reason: "station has no interior point".into(),
            });
            continue;
        };
        let lonlat = match inverse_lambert93(point.x(), point.y()) {
            Ok(point) => point,
            Err(error) => {
                rejected.push(RejectedFeature {
                    source_id: source.id.clone(),
                    source_feature_id: id,
                    reason: error.to_string(),
                });
                continue;
            }
        };
        let geometry = Geometry::Point(lonlat);
        let bbox = geometry.bbox()?;
        if !bbox.intersects(region) {
            continue;
        }
        let feature_id = stable_id(&source.id, &id);
        accepted.push((
            CanonicalFeature {
                id: feature_id,
                kind: FeatureKind::PlaceStation,
                geometry,
                bbox,
                importance: 600,
                min_zoom: 13,
                max_zoom: 15,
                name,
                revision: 1,
            },
            Provenance {
                feature_id,
                source_id: source.id.clone(),
                source_feature_id: id,
                source_revision: source.source_version.clone(),
                adapter_version: source.adapter_version,
                source_feature_sha256: hash.finalize().into(),
            },
        ));
    }
    accepted.sort_by_key(|(feature, _)| feature.id);
    Ok((accepted, rejected))
}
