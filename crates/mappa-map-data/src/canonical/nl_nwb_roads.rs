//! Build-only reader for Rijkswaterstaat's monthly NWB-Wegen road sections.

use super::*;
use proj4rs::{proj::Proj, transform::transform};
use rusqlite::{Connection, OpenFlags, params};

const RD_NEW: &str = concat!(
    "+proj=sterea +lat_0=52.1561605555556 +lon_0=5.38763888888889 ",
    "+k=0.9999079 +x_0=155000 +y_0=463000 +ellps=bessel ",
    "+towgs84=565.4171,50.3319,465.5524,-0.398957388243134,",
    "0.343987817378283,-1.87740163998045,4.0725 +units=m +no_defs +type=crs"
);
const WGS84: &str = "+proj=longlat +ellps=WGS84 +datum=WGS84 +no_defs";
const CELL_METERS: i64 = 100_000;

fn invalid(message: impl Into<String>) -> CanonicalError {
    CanonicalError::Feature(message.into())
}

fn connection(source: &SourceRecord) -> Result<Connection, CanonicalError> {
    if source.adapter != "nl-nwb-wegen" {
        return Err(invalid("wrong Dutch NWB adapter"));
    }
    if source_hash(Path::new(&source.file))? != source.sha256.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(source.id.clone()));
    }
    let connection = Connection::open_with_flags(&source.file, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| invalid(error.to_string()))?;
    let (kind, srid): (String, i64) = connection
        .query_row(
            "SELECT geometry_type_name, srs_id FROM gpkg_geometry_columns WHERE table_name='Wegvakken' AND column_name='geom'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| invalid(error.to_string()))?;
    if kind != "LINESTRING" || srid != 28992 {
        return Err(invalid("unexpected Dutch NWB geometry or CRS"));
    }
    Ok(connection)
}

pub fn nl_nwb_regions(source: &SourceRecord) -> Result<Vec<(i64, i64)>, CanonicalError> {
    let connection = connection(source)?;
    let mut query = connection
        .prepare("SELECT CAST((minx + maxx) / (2 * ?1) AS INTEGER), CAST((miny + maxy) / (2 * ?1) AS INTEGER) FROM rtree_Wegvakken_geom GROUP BY 1, 2 ORDER BY 1, 2")
        .map_err(|error| invalid(error.to_string()))?;
    let rows = query
        .query_map([CELL_METERS], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|error| invalid(error.to_string()))?;
    rows.map(|row| row.map_err(|error| invalid(error.to_string())))
        .collect()
}

fn read_line(blob: &[u8], from: &Proj, to: &Proj) -> Result<Vec<[f64; 2]>, CanonicalError> {
    if blob.len() < 49 || blob[..4] != *b"GP\0\x03" || blob[4..8] != 28992u32.to_le_bytes() {
        return Err(invalid("unexpected Dutch GeoPackage header"));
    }
    if blob[40] != 1 || blob[41..45] != 2u32.to_le_bytes() {
        return Err(invalid("expected little-endian 2D LineString"));
    }
    let count = u32::from_le_bytes(blob[45..49].try_into().expect("four bytes")) as usize;
    if !(2..=1_000_000).contains(&count) || blob.len() != 49 + count * 16 {
        return Err(invalid("invalid Dutch road point count or geometry length"));
    }
    let mut points = Vec::with_capacity(count);
    for point in blob[49..].as_chunks::<16>().0 {
        let x = f64::from_le_bytes(point[..8].try_into().expect("eight bytes"));
        let y = f64::from_le_bytes(point[8..].try_into().expect("eight bytes"));
        let mut coordinate = (x, y, 0.0);
        transform(from, to, &mut coordinate).map_err(|error| invalid(error.to_string()))?;
        let lonlat = [coordinate.0.to_degrees(), coordinate.1.to_degrees()];
        if !lonlat.iter().all(|value| value.is_finite()) {
            return Err(invalid("non-finite Dutch road coordinate"));
        }
        points.push(lonlat);
    }
    Ok(points)
}

pub fn adapt_nl_nwb_roads(
    source: &SourceRecord,
    region: (i64, i64),
) -> Result<(AdaptedFeatures, Vec<RejectedFeature>), CanonicalError> {
    let connection = connection(source)?;
    let from = Proj::from_proj_string(RD_NEW).map_err(|error| invalid(error.to_string()))?;
    let to = Proj::from_proj_string(WGS84).map_err(|error| invalid(error.to_string()))?;
    let mut query = connection
        .prepare("SELECT w.WVK_ID, w.geom, w.FRC, w.BST_CODE, w.STT_NAAM, w.WEGNUMMER FROM Wegvakken w JOIN rtree_Wegvakken_geom r ON r.id=w.id WHERE (r.minx+r.maxx)/2 >= ?1 AND (r.minx+r.maxx)/2 < ?2 AND (r.miny+r.maxy)/2 >= ?3 AND (r.miny+r.maxy)/2 < ?4 ORDER BY w.id")
        .map_err(|error| invalid(error.to_string()))?;
    let mut rows = query
        .query(params![
            region.0 * CELL_METERS,
            (region.0 + 1) * CELL_METERS,
            region.1 * CELL_METERS,
            (region.1 + 1) * CELL_METERS
        ])
        .map_err(|error| invalid(error.to_string()))?;
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    while let Some(row) = rows.next().map_err(|error| invalid(error.to_string()))? {
        let road_id: i64 = row.get(0).map_err(|error| invalid(error.to_string()))?;
        let blob: Vec<u8> = row.get(1).map_err(|error| invalid(error.to_string()))?;
        let frc: Option<String> = row.get(2).map_err(|error| invalid(error.to_string()))?;
        let lane: Option<String> = row.get(3).map_err(|error| invalid(error.to_string()))?;
        let raw_name: Option<String> = row.get(4).map_err(|error| invalid(error.to_string()))?;
        let number: Option<String> = row.get(5).map_err(|error| invalid(error.to_string()))?;
        let source_feature_id = road_id.to_string();
        // FRC 7 is explicitly closed to general car traffic. These lane codes
        // describe bicycle, pedestrian, transit-only, or emergency facilities.
        let excluded_lane = matches!(
            lane.as_deref(),
            Some("FP" | "VP" | "VZ" | "BUS" | "OVB" | "CADO")
        );
        let style = if excluded_lane {
            None
        } else {
            match frc.as_deref() {
                Some("0" | "1" | "2") => Some((FeatureKind::RoadPrimary, 800, 10)),
                Some("3") => Some((FeatureKind::RoadPrimary, 650, 10)),
                Some("4" | "5") => Some((FeatureKind::RoadSecondary, 450, 11)),
                Some("6") => Some((FeatureKind::RoadResidential, 200, 12)),
                _ => None,
            }
        };
        let Some((kind, importance, min_zoom)) = style else {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id,
                reason: format!("not a general vehicle road: FRC={frc:?}, BST_CODE={lane:?}"),
            });
            continue;
        };
        let geometry = match read_line(&blob, &from, &to) {
            Ok(points) => Geometry::Line(points),
            Err(error) => {
                rejected.push(RejectedFeature {
                    source_id: source.id.clone(),
                    source_feature_id,
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
                    source_feature_id,
                    reason: error.to_string(),
                });
                continue;
            }
        };
        let name = raw_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty() && value.len() <= 128)
            .map(str::to_owned);
        let id = stable_id(&source.id, &source_feature_id);
        let mut hash = Sha256::new();
        hash.update(road_id.to_le_bytes());
        hash.update(&blob);
        hash.update(frc.as_deref().unwrap_or("").as_bytes());
        hash.update(lane.as_deref().unwrap_or("").as_bytes());
        hash.update(raw_name.as_deref().unwrap_or("").as_bytes());
        hash.update(number.as_deref().unwrap_or("").as_bytes());
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
                source_feature_sha256: hash.finalize().into(),
            },
        ));
    }
    accepted.sort_by_key(|(feature, _)| feature.id);
    Ok((accepted, rejected))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rd_new_matches_independent_proj_point() {
        let from = Proj::from_proj_string(RD_NEW).unwrap();
        let to = Proj::from_proj_string(WGS84).unwrap();
        let mut point = (118183.47441707, 439954.593883019, 0.0);
        transform(&from, &to, &mut point).unwrap();
        // PROJ EPSG:28992 → EPSG:4326 for a source road vertex.
        assert!((point.0.to_degrees() - 4.851713166334).abs() < 0.000001);
        assert!((point.1.to_degrees() - 51.946819749874).abs() < 0.000001);
    }

    #[test]
    fn rejects_unexpected_geometry() {
        let from = Proj::from_proj_string(RD_NEW).unwrap();
        let to = Proj::from_proj_string(WGS84).unwrap();
        assert!(read_line(b"not a geopackage", &from, &to).is_err());
    }
}
