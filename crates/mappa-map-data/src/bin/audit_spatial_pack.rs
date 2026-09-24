//! Compare every quantized MSP feature against the canonical GeoDB source.
use mappa_map_data::{
    canonical::{FeatureKind, GeoDb, Geometry},
    spatial_pack::SpatialPack,
};
use std::{collections::BTreeMap, error::Error, path::Path};

fn points(geometry: &Geometry) -> Vec<[f64; 2]> {
    match geometry {
        Geometry::Point(point) => vec![*point],
        Geometry::Line(line) => line.clone(),
        Geometry::Polygon(rings) => rings.iter().flatten().copied().collect(),
    }
}

fn same_shape(a: &Geometry, b: &Geometry) -> bool {
    match (a, b) {
        (Geometry::Point(_), Geometry::Point(_)) => true,
        (Geometry::Line(a), Geometry::Line(b)) => a.len() == b.len(),
        (Geometry::Polygon(a), Geometry::Polygon(b)) => {
            a.iter().map(Vec::len).eq(b.iter().map(Vec::len))
        }
        _ => false,
    }
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: audit_spatial_pack INPUT.mgeodb FILE.msp BASELINE.pmtiles".into());
    }
    let mut db = GeoDb::open(Path::new(&args[0]))?;
    let pack = SpatialPack::open(Path::new(&args[1]))?;
    let baseline_size = std::fs::metadata(&args[2])?.len();
    let source = db.query(pack.region())?;
    if source.len() != pack.feature_count() {
        return Err("feature count mismatch".into());
    }
    let mut source_by_id = source
        .into_iter()
        .map(|f| (f.id, f))
        .collect::<BTreeMap<_, _>>();
    let mut kinds = BTreeMap::<&'static str, usize>::new();
    let mut max_error_m = 0f64;
    let mut points_checked = 0usize;
    for ordinal in 0..pack.feature_count() {
        let decoded = pack.decode(ordinal)?;
        let original = source_by_id
            .remove(&decoded.id)
            .ok_or("MSP contains an unknown or duplicated feature ID")?;
        if decoded.kind != original.kind
            || decoded.min_zoom != original.min_zoom
            || decoded.max_zoom != original.max_zoom
            || decoded.name != original.name
            || decoded.importance != original.importance
            || decoded.revision != original.revision
            || !same_shape(&decoded.geometry, &original.geometry)
        {
            return Err(format!("feature metadata or shape mismatch: {}", original.id).into());
        }
        let name = match decoded.kind {
            FeatureKind::RoadPrimary => "road_primary",
            FeatureKind::RoadSecondary => "road_secondary",
            FeatureKind::RoadResidential => "road_residential",
            FeatureKind::RoadSurface => "road_surface",
            FeatureKind::Building => "building",
            FeatureKind::Water => "water",
            FeatureKind::Park => "park",
            FeatureKind::Rail => "rail",
            FeatureKind::Place => "place",
            FeatureKind::PlaceDistrict => "place_district",
            FeatureKind::Vegetation => "vegetation",
        };
        *kinds.entry(name).or_default() += 1;
        for (a, b) in points(&original.geometry)
            .into_iter()
            .zip(points(&decoded.geometry))
        {
            let latitude = ((a[1] + b[1]) / 2.0).to_radians();
            let dx = (a[0] - b[0]).to_radians() * 6_378_137.0 * latitude.cos();
            let dy = (a[1] - b[1]).to_radians() * 6_378_137.0;
            max_error_m = max_error_m.max(dx.hypot(dy));
            points_checked += 1;
        }
    }
    if !source_by_id.is_empty() {
        return Err("MSP omitted canonical features".into());
    }
    let pack_size = std::fs::metadata(&args[1])?.len();
    println!(
        "features={} cells={} points={} max_quantization_error_m={max_error_m:.6}",
        pack.feature_count(),
        pack.cell_count(),
        points_checked
    );
    println!("kinds={kinds:?}");
    println!(
        "msp_bytes={pack_size} baseline_pmtiles_bytes={baseline_size} ratio={:.3}",
        pack_size as f64 / baseline_size as f64
    );
    Ok(())
}
