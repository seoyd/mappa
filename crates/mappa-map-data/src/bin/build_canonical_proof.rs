use flate2::{Compression, write::GzEncoder};
use mappa_map_data::canonical::{
    BBox, GeoDb, SourceManifest, adapt_microsoft_buildings, adapt_naju_road_surfaces,
    adapt_naju_roads, adapt_sgis_districts, adapt_us_census_areawater, adapt_us_census_parks,
    adapt_us_census_roads, adapt_worldcover_polygons, write_geodb,
};
use std::{error::Error, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 && (args.len() != 4 || args[3] != "--gzip-rejections") {
        return Err(
            "usage: build_canonical_proof data/sources.toml OUTPUT.mgeodb [--gzip-rejections]"
                .into(),
        );
    }
    let manifest = SourceManifest::open(Path::new(&args[1]))?;
    let [west, south, east, north] = manifest.proof_bbox_wgs84;
    let region = BBox {
        west,
        south,
        east,
        north,
    };
    let mut records = Vec::new();
    let mut rejected = Vec::new();
    for source in &manifest.source {
        match source.adapter.as_str() {
            "naju-road-centerline" => records.extend(adapt_naju_roads(source, region)?),
            "naju-road-surface" => {
                let (accepted, invalid) = adapt_naju_road_surfaces(source, region)?;
                records.extend(accepted);
                rejected.extend(invalid);
            }
            "sgis-admin-district" => records.extend(adapt_sgis_districts(source, region)?),
            "esa-worldcover-water" | "esa-worldcover-tree" => {
                let (accepted, invalid) = adapt_worldcover_polygons(source, region)?;
                records.extend(accepted);
                rejected.extend(invalid);
            }
            "microsoft-ml-building-footprints" => {
                let (accepted, invalid) = adapt_microsoft_buildings(source, region)?;
                records.extend(accepted);
                rejected.extend(invalid);
            }
            "us-census-tiger-roads" => {
                let (accepted, invalid) = adapt_us_census_roads(source, region)?;
                records.extend(accepted);
                rejected.extend(invalid);
            }
            "us-census-tiger-areawater" => {
                let (accepted, invalid) = adapt_us_census_areawater(source, region)?;
                records.extend(accepted);
                rejected.extend(invalid);
            }
            "us-census-tiger-arealm-parks" => {
                let (accepted, invalid) = adapt_us_census_parks(source, region)?;
                records.extend(accepted);
                rejected.extend(invalid);
            }
            _ => return Err("unsupported source adapter".into()),
        }
    }
    records.sort_by_key(|(feature, _)| feature.id);
    if args.len() == 4 {
        let writer = GzEncoder::new(
            std::fs::File::create(Path::new(&args[2]).with_extension("rejected.json.gz"))?,
            Compression::default(),
        );
        let mut writer = writer;
        serde_json::to_writer(&mut writer, &rejected)?;
        writer.finish()?;
    } else {
        std::fs::write(
            Path::new(&args[2]).with_extension("rejected.json"),
            serde_json::to_vec_pretty(&rejected)?,
        )?;
    }
    write_geodb(Path::new(&args[2]), &records)?;
    let mut database = GeoDb::open(Path::new(&args[2]))?;
    let queried = database.query(region)?;
    if queried.len() != records.len() {
        return Err("MappaGeoDB region query omitted a source feature".into());
    }
    println!(
        "region={} sources={} features={} provenance={} rejected={} geodb_bytes={}",
        manifest.proof_region,
        database.sources.len(),
        database.feature_count(),
        database.provenance.len(),
        rejected.len(),
        std::fs::metadata(&args[2])?.len(),
    );
    Ok(())
}
