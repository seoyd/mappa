//! Inverse EPSG:2154 for build-time French source ingestion.
//!
//! Projection constants come from IGN's Lambert-93 technical parameters. The
//! returned geographic angles remain RGF93; a WGS84 datum accuracy claim
//! requires an independently checked coordinate operation and control points.

use super::CanonicalError;

/// Invert the RGF93 / Lambert-93 projected coordinates to geographic degrees.
pub fn inverse_lambert93(easting: f64, northing: f64) -> Result<[f64; 2], CanonicalError> {
    if !easting.is_finite() || !northing.is_finite() {
        return Err(CanonicalError::Feature(
            "non-finite Lambert-93 coordinate".into(),
        ));
    }
    // IGN NTG_71: cone constant, scale constant, and false pole (metres).
    const N: f64 = 0.725_607_765_0;
    const C: f64 = 11_754_255.426;
    const XS: f64 = 700_000.0;
    const YS: f64 = 12_655_612.050;
    // GRS80 ellipsoid used by RGF93.
    const INV_FLATTENING: f64 = 298.257_222_101;
    let flattening = 1.0 / INV_FLATTENING;
    let eccentricity = (2.0 * flattening - flattening * flattening).sqrt();
    let dx = easting - XS;
    let dy = YS - northing;
    let radius = dx.hypot(dy);
    if radius <= 0.0 {
        return Err(CanonicalError::Feature(
            "Lambert-93 coordinate at projection pole".into(),
        ));
    }
    let longitude = 3.0 + (dx.atan2(dy) / N).to_degrees();
    let isometric_latitude = -(radius / C).ln() / N;
    let mut latitude = 2.0 * isometric_latitude.exp().atan() - std::f64::consts::FRAC_PI_2;
    for _ in 0..12 {
        let es = eccentricity * latitude.sin();
        let next = 2.0
            * (isometric_latitude.exp() * ((1.0 + es) / (1.0 - es)).powf(eccentricity / 2.0))
                .atan()
            - std::f64::consts::FRAC_PI_2;
        if (next - latitude).abs() < 1e-14 {
            latitude = next;
            break;
        }
        latitude = next;
    }
    let latitude = latitude.to_degrees();
    if !longitude.is_finite()
        || !latitude.is_finite()
        || !(-180.0..=180.0).contains(&longitude)
        || !(-85.051_128_78..=85.051_128_78).contains(&latitude)
    {
        return Err(CanonicalError::Feature(
            "Lambert-93 inverse outside Web Mercator bounds".into(),
        ));
    }
    Ok([longitude, latitude])
}

#[cfg(test)]
mod tests {
    use super::inverse_lambert93;

    #[test]
    fn matches_projection_origin_and_independent_proj_samples() {
        // Expected values were obtained from PROJ cs2cs EPSG:2154 -> EPSG:4326.
        for (x, y, lon, lat) in [
            (700_000.0, 6_600_000.0, 3.0, 46.5),
            (652_469.0, 6_862_035.0, 2.352_199_719_5, 48.856_597_665_4),
            (1_000_000.0, 6_500_000.0, 6.845_273_477_4, 45.533_728_844_6),
        ] {
            let [actual_lon, actual_lat] = inverse_lambert93(x, y).unwrap();
            assert!((actual_lon - lon).abs() < 1e-7, "longitude {x},{y}");
            assert!((actual_lat - lat).abs() < 1e-7, "latitude {x},{y}");
        }
    }

    #[test]
    fn invalid_coordinates_fail_closed() {
        assert!(inverse_lambert93(f64::NAN, 6_600_000.0).is_err());
        assert!(inverse_lambert93(700_000.0, 12_655_612.050).is_err());
    }
}
