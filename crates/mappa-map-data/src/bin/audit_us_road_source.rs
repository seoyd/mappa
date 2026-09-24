//! Inspect a publisher ZIP before recording its observed bounds and checksum.

use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fs::File,
    io::{Read, Seek},
    path::Path,
};
use zip::ZipArchive;

fn member(archive: &mut ZipArchive<File>, suffix: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let matches: Vec<_> = archive
        .file_names()
        .filter(|name| name.ends_with(suffix))
        .map(str::to_owned)
        .collect();
    let [name] = matches.as_slice() else {
        return Err(format!("expected one {suffix} member").into());
    };
    let mut member = archive.by_name(name)?;
    if member.size() > 64 * 1024 * 1024 {
        return Err(format!("{suffix} exceeds inspection limit").into());
    }
    let mut bytes = Vec::with_capacity(member.size() as usize);
    member.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn main() -> Result<(), Box<dyn Error>> {
    let paths: Vec<_> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        return Err("usage: audit_us_road_source ROADS_OR_AREAWATER_ZIP [ZIP ...]".into());
    }
    let mut union = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    let mut total = 0u64;
    for path in paths {
        let path = Path::new(&path);
        let mut file = File::open(path)?;
        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher)?;
        file.rewind()?;
        let mut archive = ZipArchive::new(file)?;
        let shape = member(&mut archive, ".shp")?;
        let dbf = member(&mut archive, ".dbf")?;
        let prj = member(&mut archive, ".prj")?;
        if shape.len() < 100 || dbf.len() < 32 {
            return Err(format!("short TIGER Shapefile header: {}", path.display()).into());
        }
        let shape_type = u32::from_le_bytes(shape[32..36].try_into()?);
        if u32::from_be_bytes(shape[0..4].try_into()?) != 9994
            || u32::from_le_bytes(shape[28..32].try_into()?) != 1000
            || !matches!(shape_type, 3 | 5)
            || !std::str::from_utf8(&prj)?.contains("GCS_North_American_1983")
        {
            return Err(
                format!("unexpected TIGER Shapefile header/CRS: {}", path.display()).into(),
            );
        }
        let mut bounds = [0.0; 4];
        for (index, value) in bounds.iter_mut().enumerate() {
            let start = 36 + index * 8;
            *value = f64::from_le_bytes(shape[start..start + 8].try_into()?);
        }
        if !bounds.iter().all(|value| value.is_finite())
            || bounds[0] > bounds[2]
            || bounds[1] > bounds[3]
            || bounds[0] < -180.0
            || bounds[2] > 180.0
            || bounds[1] < -90.0
            || bounds[3] > 90.0
        {
            return Err(format!("invalid TIGER bounds: {}", path.display()).into());
        }
        let records = u32::from_le_bytes(dbf[4..8].try_into()?);
        total += u64::from(records);
        union[0] = union[0].min(bounds[0]);
        union[1] = union[1].min(bounds[1]);
        union[2] = union[2].max(bounds[2]);
        union[3] = union[3].max(bounds[3]);
        println!(
            "{} sha256={:x} shape_type={shape_type} source_crs=EPSG:4269 bbox_nad83={bounds:?} dbf_records={records}",
            path.display(),
            hasher.finalize()
        );
    }
    println!("source_numeric_union={union:?} dbf_records={total}");
    Ok(())
}
