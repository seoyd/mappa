use mappa_domain::{CellId, Coordinate};
use thiserror::Error;

pub const CONTENT_CELL_ZOOM: u8 = 14;
pub const MERCATOR_MAX_LAT_E7: i32 = 850_511_287;
const AXIS_MASK: u64 = (1 << 29) - 1;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum SpatialError {
    #[error("latitude exceeds Web Mercator bounds")]
    OutsideMercator,
    #[error("invalid cell coordinate or packed id")]
    InvalidCell,
}

// Layout: [zoom:6][x:29][y:29]. Zoom 0..29; unused x/y bits must be zero.
pub fn xy_to_cell(zoom: u8, x: u32, y: u32) -> Result<CellId, SpatialError> {
    if zoom > 29 || x >= (1_u32 << zoom) || y >= (1_u32 << zoom) {
        return Err(SpatialError::InvalidCell);
    }
    Ok(CellId(
        (u64::from(zoom) << 58) | (u64::from(x) << 29) | u64::from(y),
    ))
}

pub fn cell_to_xy(cell: CellId) -> Result<(u8, u32, u32), SpatialError> {
    let zoom = (cell.0 >> 58) as u8;
    let x = ((cell.0 >> 29) & AXIS_MASK) as u32;
    let y = (cell.0 & AXIS_MASK) as u32;
    xy_to_cell(zoom, x, y)?;
    Ok((zoom, x, y))
}

pub fn coordinate_to_cell(coordinate: Coordinate) -> Result<CellId, SpatialError> {
    if coordinate.lat_e7.abs() > MERCATOR_MAX_LAT_E7 {
        return Err(SpatialError::OutsideMercator);
    }
    let n = f64::from(1_u32 << CONTENT_CELL_ZOOM);
    let lon = f64::from(coordinate.lon_e7) / 10_000_000.0;
    let lat_rad = (f64::from(coordinate.lat_e7) / 10_000_000.0).to_radians();
    let x = (((lon + 180.0) / 360.0) * n).floor().clamp(0.0, n - 1.0) as u32;
    let y = ((1.0 - lat_rad.tan().asinh() / std::f64::consts::PI) * n / 2.0)
        .floor()
        .clamp(0.0, n - 1.0) as u32;
    xy_to_cell(CONTENT_CELL_ZOOM, x, y)
}

pub fn neighbors_3x3(cell: CellId) -> Result<Vec<CellId>, SpatialError> {
    let (zoom, x, y) = cell_to_xy(cell)?;
    let n = i64::from(1_u32 << zoom);
    let mut result = Vec::with_capacity(9);
    for dy in -1_i64..=1 {
        let neighbor_y = i64::from(y) + dy;
        if !(0..n).contains(&neighbor_y) {
            continue;
        }
        for dx in -1_i64..=1 {
            let neighbor_x = (i64::from(x) + dx).rem_euclid(n);
            result.push(xy_to_cell(zoom, neighbor_x as u32, neighbor_y as u32)?);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packing_roundtrip_and_rejects_stray_bits() {
        for zoom in 0..=29 {
            let n = 1_u32 << zoom;
            for (x, y) in [(0, 0), (n - 1, n - 1)] {
                assert_eq!(
                    cell_to_xy(xy_to_cell(zoom, x, y).unwrap()).unwrap(),
                    (zoom, x, y)
                );
            }
        }
        assert!(cell_to_xy(CellId(u64::MAX)).is_err());
    }
    #[test]
    fn seoul_is_deterministic_and_nearby() {
        let a = Coordinate::new(375_665_000, 1_269_780_000).unwrap();
        let x = coordinate_to_cell(a).unwrap();
        assert_eq!(coordinate_to_cell(a).unwrap(), x);
        let b = Coordinate::new(375_666_000, 1_269_781_000).unwrap();
        assert!(
            neighbors_3x3(x)
                .unwrap()
                .contains(&coordinate_to_cell(b).unwrap())
        );
    }
    #[test]
    fn wrap_and_poles() {
        let west = coordinate_to_cell(Coordinate::new(0, -1_800_000_000).unwrap()).unwrap();
        let east = coordinate_to_cell(Coordinate::new(0, 1_800_000_000).unwrap()).unwrap();
        assert!(neighbors_3x3(west).unwrap().contains(&east));
        assert!(coordinate_to_cell(Coordinate::new(900_000_000, 0).unwrap()).is_err());
        assert_eq!(
            neighbors_3x3(xy_to_cell(14, 0, 0).unwrap()).unwrap().len(),
            6
        );
    }
}
