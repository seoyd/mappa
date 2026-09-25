//! Build-only reader for Luxembourg's official BD-L-GeoBase TransportNetwork GeoPackage.

use super::*;
use proj4rs::{proj::Proj, transform::transform};
use rusqlite::{Connection, OpenFlags};

const LUREF_TM: &str = concat!(
    "+proj=tmerc +lat_0=49.8333333333333 +lon_0=6.16666666666667 ",
    "+k=1 +x_0=80000 +y_0=100000 +ellps=intl ",
    // EPSG LUREF→WGS84 (3): the coordinate-frame rotations are negated for
    // PROJ.4's position-vector +towgs84 convention.
    "+towgs84=-189.6806,18.3463,-42.7695,-0.33746,-3.09264,2.53861,0.4598 ",
    "+units=m +no_defs +type=crs"
);
const WGS84: &str = "+proj=longlat +ellps=WGS84 +datum=WGS84 +no_defs";

fn invalid(message: impl Into<String>) -> CanonicalError {
    CanonicalError::Feature(message.into())
}

struct Bytes<'a> {
    data: &'a [u8],
    offset: usize,
}

impl Bytes<'_> {
    fn take(&mut self, len: usize) -> Result<&[u8], CanonicalError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| invalid("geometry length overflow"))?;
        let bytes = self
            .data
            .get(self.offset..end)
            .ok_or_else(|| invalid("truncated GeoPackage geometry"))?;
        self.offset = end;
        Ok(bytes)
    }

    fn u32(&mut self) -> Result<u32, CanonicalError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("four bytes"),
        ))
    }

    fn f64(&mut self) -> Result<f64, CanonicalError> {
        Ok(f64::from_le_bytes(
            self.take(8)?.try_into().expect("eight bytes"),
        ))
    }
}

fn read_parts(blob: &[u8], from: &Proj, to: &Proj) -> Result<Vec<Vec<[f64; 2]>>, CanonicalError> {
    let mut input = Bytes {
        data: blob,
        offset: 0,
    };
    if input.take(2)? != b"GP" || input.take(1)? != [0] || input.take(1)? != [5] {
        return Err(invalid("expected little-endian GeoPackage XYZ envelope"));
    }
    if input.u32()? != 2169 {
        return Err(invalid("expected Luxembourg EPSG:2169 geometry"));
    }
    input.take(48)?;
    if input.take(1)? != [1] || input.u32()? != 3011 {
        return Err(invalid("expected little-endian MultiCurve ZM"));
    }
    let count = usize::try_from(input.u32()?).map_err(|_| invalid("part count overflow"))?;
    if count == 0 || count > 1024 {
        return Err(invalid("invalid road part count"));
    }
    let mut parts = Vec::with_capacity(count);
    for _ in 0..count {
        if input.take(1)? != [1] || input.u32()? != 3002 {
            return Err(invalid("expected little-endian LineString ZM part"));
        }
        let point_count =
            usize::try_from(input.u32()?).map_err(|_| invalid("point count overflow"))?;
        if !(2..=1_000_000).contains(&point_count) {
            return Err(invalid("invalid road point count"));
        }
        let mut points = Vec::with_capacity(point_count);
        for _ in 0..point_count {
            let mut point = (input.f64()?, input.f64()?, 0.0);
            input.take(16)?; // Source Z and measure are not used by a 2D map.
            transform(from, to, &mut point).map_err(|error| invalid(error.to_string()))?;
            let projected = [point.0.to_degrees(), point.1.to_degrees()];
            if !projected.iter().all(|coordinate| coordinate.is_finite()) {
                return Err(invalid("non-finite transformed road point"));
            }
            points.push(projected);
        }
        parts.push(points);
    }
    if input.offset != blob.len() {
        return Err(invalid("trailing GeoPackage geometry bytes"));
    }
    Ok(parts)
}

pub fn adapt_lu_geobase_roads(
    source: &SourceRecord,
) -> Result<(AdaptedFeatures, Vec<RejectedFeature>), CanonicalError> {
    if source.adapter != "lu-geobase-road" {
        return Err(invalid("wrong Luxembourg road adapter"));
    }
    if source_hash(Path::new(&source.file))? != source.sha256.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(source.id.clone()));
    }
    let archive = source
        .upstream_file
        .as_ref()
        .ok_or_else(|| invalid("missing source archive"))?;
    let archive_sha256 = source
        .upstream_sha256
        .as_ref()
        .ok_or_else(|| invalid("missing archive hash"))?;
    if source_hash(Path::new(archive))? != archive_sha256.to_lowercase() {
        return Err(CanonicalError::SourceChecksum(archive.clone()));
    }
    let connection = Connection::open_with_flags(&source.file, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| invalid(error.to_string()))?;
    let srid: i64 = connection
        .query_row("SELECT srs_id FROM gpkg_geometry_columns WHERE table_name='TransportNetwork_TN_Road' AND column_name='geom'", [], |row| row.get(0))
        .map_err(|error| invalid(error.to_string()))?;
    if srid != 2169 {
        return Err(invalid("unexpected Luxembourg source CRS"));
    }
    let from = Proj::from_proj_string(LUREF_TM).map_err(|error| invalid(error.to_string()))?;
    let to = Proj::from_proj_string(WGS84).map_err(|error| invalid(error.to_string()))?;
    let mut query = connection
        .prepare("SELECT id, geom, TN_RoadID, Name, FormOfWay, ConditionOfFacility FROM TransportNetwork_TN_Road ORDER BY id")
        .map_err(|error| invalid(error.to_string()))?;
    let mut rows = query
        .query([])
        .map_err(|error| invalid(error.to_string()))?;
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    while let Some(row) = rows.next().map_err(|error| invalid(error.to_string()))? {
        let row_id: i64 = row.get(0).map_err(|error| invalid(error.to_string()))?;
        let blob: Vec<u8> = row.get(1).map_err(|error| invalid(error.to_string()))?;
        let road_id: Option<String> = row.get(2).map_err(|error| invalid(error.to_string()))?;
        let raw_name: Option<String> = row.get(3).map_err(|error| invalid(error.to_string()))?;
        let form: Option<String> = row.get(4).map_err(|error| invalid(error.to_string()))?;
        let condition: Option<String> = row.get(5).map_err(|error| invalid(error.to_string()))?;
        let source_feature_id = format!("{row_id}:{}", road_id.as_deref().unwrap_or(""));
        let road_style = match (form.as_deref(), condition.as_deref()) {
            (Some("Freeway"), Some("Functional")) => Some((FeatureKind::RoadPrimary, 800, 10)),
            (Some("NationalRoad"), Some("Functional")) => Some((FeatureKind::RoadPrimary, 650, 10)),
            (Some("StateRoad"), Some("Functional")) => Some((FeatureKind::RoadSecondary, 450, 11)),
            (Some("MunicipalRoad"), Some("Functional")) => {
                Some((FeatureKind::RoadResidential, 200, 12))
            }
            _ => None,
        };
        let Some((kind, importance, min_zoom)) = road_style else {
            rejected.push(RejectedFeature {
                source_id: source.id.clone(),
                source_feature_id,
                reason: format!("non-vehicle, unknown, or non-functional road: FormOfWay={form:?}, ConditionOfFacility={condition:?}"),
            });
            continue;
        };
        let name = raw_name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty() && name.len() <= 128)
            .map(str::to_owned);
        let parts = match read_parts(&blob, &from, &to) {
            Ok(parts) => parts,
            Err(error) => {
                rejected.push(RejectedFeature {
                    source_id: source.id.clone(),
                    source_feature_id,
                    reason: error.to_string(),
                });
                continue;
            }
        };
        for (part_index, points) in parts.into_iter().enumerate() {
            let part_id = format!("{source_feature_id}:{part_index}");
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
            let mut hash = Sha256::new();
            hash.update(row_id.to_le_bytes());
            hash.update(&blob);
            hash.update(form.as_deref().unwrap_or("").as_bytes());
            hash.update(condition.as_deref().unwrap_or("").as_bytes());
            hash.update(raw_name.as_deref().unwrap_or("").as_bytes());
            hash.update(part_index.to_le_bytes());
            accepted.push((
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
    accepted.sort_by_key(|(feature, _)| feature.id);
    Ok((accepted, rejected))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsupported_geometry_layout() {
        let from = Proj::from_proj_string(LUREF_TM).unwrap();
        let to = Proj::from_proj_string(WGS84).unwrap();
        assert!(read_parts(b"not a geopackage", &from, &to).is_err());
    }

    #[test]
    fn transforms_source_point_near_independent_proj_result() {
        let from = Proj::from_proj_string(LUREF_TM).unwrap();
        let to = Proj::from_proj_string(WGS84).unwrap();
        let mut blob = b"GP\0\x05".to_vec();
        blob.extend_from_slice(&2169u32.to_le_bytes());
        blob.extend_from_slice(&[0; 48]);
        blob.push(1);
        blob.extend_from_slice(&3011u32.to_le_bytes());
        blob.extend_from_slice(&1u32.to_le_bytes());
        blob.push(1);
        blob.extend_from_slice(&3002u32.to_le_bytes());
        blob.extend_from_slice(&2u32.to_le_bytes());
        for (x, y) in [(75022.8582_f64, 72370.5895_f64), (75026.3639, 72371.1848)] {
            for ordinate in [x, y, 0.0, 0.0] {
                blob.extend_from_slice(&ordinate.to_le_bytes());
            }
        }
        let parts = read_parts(&blob, &from, &to).unwrap();
        let [lon, lat] = parts[0][0];
        // Independent GDAL/PROJ EPSG:2169 → EPSG:4326 result for the same source point.
        assert!((lon - 6.099291756708264).abs() < 0.000001, "lon={lon}");
        assert!((lat - 49.585968507157801).abs() < 0.000001, "lat={lat}");
    }
}
