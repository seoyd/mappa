//! Projection, camera, and visible-tile selection. No IO or renderer types live here.

use std::f64::consts::PI;
use thiserror::Error;

pub const MAX_LAT: f64 = 85.051_128_78;
pub const TILE_PX: f64 = 512.0;
pub const MAX_ZOOM: f64 = 16.0;
const WEB_MERCATOR_RADIUS_M: f64 = 6_378_137.0;

#[derive(Debug, Error, PartialEq)]
pub enum MapError {
    #[error("non-finite coordinate, viewport, or zoom")]
    NonFinite,
    #[error("invalid viewport or scale factor")]
    InvalidViewport,
    #[error("invalid tile coordinate")]
    InvalidTile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TileKey {
    pub z: u8,
    pub x: u32,
    pub y: u32,
}

impl TileKey {
    pub fn new(z: u8, x: u32, y: u32) -> Result<Self, MapError> {
        if z > 15 || x >= (1u32 << z) || y >= (1u32 << z) {
            return Err(MapError::InvalidTile);
        }
        Ok(Self { z, x, y })
    }
}

/// Unit-square Web Mercator coordinate; x grows east, y grows south.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldPoint {
    pub x: f64,
    pub y: f64,
}

pub fn project(lon: f64, lat: f64) -> Result<WorldPoint, MapError> {
    if !lon.is_finite() || !lat.is_finite() {
        return Err(MapError::NonFinite);
    }
    let lat = lat.clamp(-MAX_LAT, MAX_LAT).to_radians();
    let x = (lon + 180.0).rem_euclid(360.0) / 360.0;
    let y = (1.0 - (lat.tan() + 1.0 / lat.cos()).ln() / PI) / 2.0;
    Ok(WorldPoint {
        x,
        y: y.clamp(0.0, 1.0),
    })
}

pub fn unproject(point: WorldPoint) -> Result<(f64, f64), MapError> {
    if !point.x.is_finite() || !point.y.is_finite() {
        return Err(MapError::NonFinite);
    }
    let lon = point.x.rem_euclid(1.0) * 360.0 - 180.0;
    let lat = (PI * (1.0 - 2.0 * point.y.clamp(0.0, 1.0)))
        .sinh()
        .atan()
        .to_degrees();
    Ok((lon, lat))
}

#[derive(Debug, Clone, Copy)]
pub struct MapCamera {
    /// Unwrapped x permits smooth panning across the antimeridian.
    pub center: WorldPoint,
    pub zoom: f64,
    pub width_px: u32,
    pub height_px: u32,
    pub scale_factor: f64,
}

impl MapCamera {
    pub fn new(
        lon: f64,
        lat: f64,
        zoom: f64,
        width_px: u32,
        height_px: u32,
        scale_factor: f64,
    ) -> Result<Self, MapError> {
        if !zoom.is_finite() || !scale_factor.is_finite() {
            return Err(MapError::NonFinite);
        }
        if width_px == 0 || height_px == 0 || scale_factor <= 0.0 {
            return Err(MapError::InvalidViewport);
        }
        Ok(Self {
            center: project(lon, lat)?,
            zoom: zoom.clamp(0.0, MAX_ZOOM),
            width_px,
            height_px,
            scale_factor,
        })
    }

    pub fn world_size_px(&self) -> f64 {
        TILE_PX * self.scale_factor * 2.0_f64.powf(self.zoom)
    }

    /// Approximate ground meters per physical screen pixel at the camera center.
    /// Web Mercator scale varies with latitude, so this is a local value.
    pub fn meters_per_pixel_at_center(&self) -> f64 {
        let latitude = (PI * (1.0 - 2.0 * self.center.y)).sinh().atan();
        2.0 * PI * WEB_MERCATOR_RADIUS_M * latitude.cos() / self.world_size_px()
    }

    pub fn world_to_screen(&self, point: WorldPoint) -> (f64, f64) {
        let dx = (point.x - self.center.x + 0.5).rem_euclid(1.0) - 0.5;
        (
            self.width_px as f64 / 2.0 + dx * self.world_size_px(),
            self.height_px as f64 / 2.0 + (point.y - self.center.y) * self.world_size_px(),
        )
    }

    pub fn unwrapped_to_screen(&self, point: WorldPoint) -> (f64, f64) {
        (
            self.width_px as f64 / 2.0 + (point.x - self.center.x) * self.world_size_px(),
            self.height_px as f64 / 2.0 + (point.y - self.center.y) * self.world_size_px(),
        )
    }

    pub fn screen_to_world(&self, x_px: f64, y_px: f64) -> Result<WorldPoint, MapError> {
        if !x_px.is_finite() || !y_px.is_finite() {
            return Err(MapError::NonFinite);
        }
        Ok(WorldPoint {
            x: self.center.x + (x_px - self.width_px as f64 / 2.0) / self.world_size_px(),
            y: (self.center.y + (y_px - self.height_px as f64 / 2.0) / self.world_size_px())
                .clamp(0.0, 1.0),
        })
    }

    pub fn screen_to_coordinate(&self, x_px: f64, y_px: f64) -> Result<(f64, f64), MapError> {
        unproject(self.screen_to_world(x_px, y_px)?)
    }

    pub fn pan_physical(&mut self, dx_px: f64, dy_px: f64) -> Result<(), MapError> {
        if !dx_px.is_finite() || !dy_px.is_finite() {
            return Err(MapError::NonFinite);
        }
        self.center.x -= dx_px / self.world_size_px();
        self.center.y = (self.center.y - dy_px / self.world_size_px()).clamp(0.0, 1.0);
        Ok(())
    }

    /// Zoom around a pointer while keeping its geographic point stationary.
    pub fn zoom_at(&mut self, delta: f64, x_px: f64, y_px: f64) -> Result<(), MapError> {
        if !delta.is_finite() {
            return Err(MapError::NonFinite);
        }
        let before = self.screen_to_world(x_px, y_px)?;
        self.zoom = (self.zoom + delta).clamp(0.0, MAX_ZOOM);
        let after = self.screen_to_world(x_px, y_px)?;
        self.center.x += before.x - after.x;
        self.center.y = (self.center.y + before.y - after.y).clamp(0.0, 1.0);
        Ok(())
    }

    pub fn resize(
        &mut self,
        width_px: u32,
        height_px: u32,
        scale_factor: f64,
    ) -> Result<(), MapError> {
        if !scale_factor.is_finite() {
            return Err(MapError::NonFinite);
        }
        if width_px == 0 || height_px == 0 || scale_factor <= 0.0 {
            return Err(MapError::InvalidViewport);
        }
        self.width_px = width_px;
        self.height_px = height_px;
        self.scale_factor = scale_factor;
        Ok(())
    }

    /// A tile's canonical storage key and unwrapped world copy placement.
    pub fn visible_tiles(&self, max_data_zoom: u8, margin: i32) -> Vec<VisibleTile> {
        let z = (self.zoom.floor() as u8).min(max_data_zoom).min(15);
        let count = 1i32 << z;
        let world = self.world_size_px();
        let left = self.center.x - self.width_px as f64 / (2.0 * world);
        let right = self.center.x + self.width_px as f64 / (2.0 * world);
        let top = self.center.y - self.height_px as f64 / (2.0 * world);
        let bottom = self.center.y + self.height_px as f64 / (2.0 * world);
        let x0 = (left * count as f64).floor() as i32 - margin;
        let x1 = (right * count as f64).floor() as i32 + margin;
        let y0 = ((top * count as f64).floor() as i32 - margin).clamp(0, count - 1);
        let y1 = ((bottom * count as f64).floor() as i32 + margin).clamp(0, count - 1);
        let mut out = Vec::new();
        for y in y0..=y1 {
            for world_x in x0..=x1 {
                out.push(VisibleTile {
                    key: TileKey {
                        z,
                        x: world_x.rem_euclid(count) as u32,
                        y: y as u32,
                    },
                    world_x,
                });
            }
        }
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisibleTile {
    pub key: TileKey,
    pub world_x: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projection_and_poles() {
        for (lon, lat) in [(-179.9, -85.0), (0.0, 0.0), (127.0, 37.5), (179.9, 85.0)] {
            let (got_lon, got_lat) = unproject(project(lon, lat).unwrap()).unwrap();
            assert!((got_lon - lon).abs() < 1e-8);
            assert!((got_lat - lat).abs() < 1e-8);
        }
        assert_eq!(project(0.0, 90.0).unwrap().y, 0.0);
        assert_eq!(project(0.0, -90.0).unwrap().y, 1.0);
        assert!(project(f64::NAN, 0.0).is_err());
    }
    #[test]
    fn camera_roundtrip_zoom_wrap_resize() {
        let mut c = MapCamera::new(179.0, 35.0, 2.75, 1200, 800, 2.0).unwrap();
        let p = project(-179.0, 37.0).unwrap();
        let (x, y) = c.world_to_screen(p);
        let (lon, lat) = c.screen_to_coordinate(x, y).unwrap();
        assert!((lon + 179.0).abs() < 1e-7 && (lat - 37.0).abs() < 1e-7);
        let anchor = c.screen_to_coordinate(300.0, 300.0).unwrap();
        c.zoom_at(0.35, 300.0, 300.0).unwrap();
        let anchor2 = c.screen_to_coordinate(300.0, 300.0).unwrap();
        assert!((anchor.0 - anchor2.0).abs() < 1e-7);
        c.pan_physical(-200.0, 0.0).unwrap();
        c.resize(2400, 1600, 2.0).unwrap();
        assert_eq!((c.width_px, c.height_px), (2400, 1600));
        assert!(
            c.visible_tiles(4, 1)
                .iter()
                .all(|t| t.key.x < (1 << t.key.z))
        );
        assert_eq!(TileKey::new(0, 0, 0).unwrap().z, 0);
    }
    #[test]
    fn repeated_pan_wraps_tile_keys_without_jump() {
        let mut camera = MapCamera::new(179.5, 0.0, 3.25, 800, 600, 1.0).unwrap();
        let before = camera.visible_tiles(4, 0);
        camera.pan_physical(-camera.world_size_px(), 0.0).unwrap();
        let after = camera.visible_tiles(4, 0);
        assert_eq!(
            before.iter().map(|t| t.key).collect::<Vec<_>>(),
            after.iter().map(|t| t.key).collect::<Vec<_>>()
        );
        assert!(
            after
                .iter()
                .zip(&before)
                .all(|(a, b)| a.world_x == b.world_x + (1 << a.key.z))
        );
    }

    #[test]
    fn overzoom_keeps_the_pointer_and_uses_available_tiles() {
        let mut camera = MapCamera::new(126.866_778, 37.302_813, 14.0, 1200, 720, 1.0).unwrap();
        let anchor = camera.screen_to_coordinate(250.0, 180.0).unwrap();
        camera.zoom_at(3.0, 250.0, 180.0).unwrap();
        assert_eq!(camera.zoom, MAX_ZOOM);
        let after = camera.screen_to_coordinate(250.0, 180.0).unwrap();
        assert!((anchor.0 - after.0).abs() < 1e-7);
        assert!((anchor.1 - after.1).abs() < 1e-7);
        assert!(
            camera
                .visible_tiles(12, 1)
                .iter()
                .all(|tile| tile.key.z == 12)
        );
    }

    #[test]
    fn ground_resolution_tracks_zoom_latitude_and_pixel_scale() {
        let equator = MapCamera::new(0.0, 0.0, 0.0, 800, 600, 1.0).unwrap();
        let zoomed = MapCamera::new(0.0, 0.0, 1.0, 800, 600, 1.0).unwrap();
        let latitude = MapCamera::new(0.0, 60.0, 0.0, 800, 600, 1.0).unwrap();
        let retina = MapCamera::new(0.0, 0.0, 0.0, 1600, 1200, 2.0).unwrap();
        assert!((equator.meters_per_pixel_at_center() - 78_271.516_96).abs() < 0.001);
        assert!(
            (zoomed.meters_per_pixel_at_center() / equator.meters_per_pixel_at_center() - 0.5)
                .abs()
                < 1e-12
        );
        assert!(
            (latitude.meters_per_pixel_at_center() / equator.meters_per_pixel_at_center() - 0.5)
                .abs()
                < 1e-12
        );
        assert!(
            (retina.meters_per_pixel_at_center() / equator.meters_per_pixel_at_center() - 0.5)
                .abs()
                < 1e-12
        );
    }
}
