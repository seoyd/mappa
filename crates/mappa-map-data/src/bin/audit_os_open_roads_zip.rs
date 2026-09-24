//! Inventory a pinned OS Open Roads national ZIP before selecting a grid square.

use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fs::File,
    io::{Read, Seek},
    path::Path,
};
use zip::ZipArchive;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err(
            "usage: audit_os_open_roads_zip SOURCE.zip SHAPEFILES.tsv ROADLINKS.tsv".into(),
        );
    }
    let mut file = File::open(&args[1])?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let sha256 = format!("{:x}", hasher.finalize());
    file.rewind()?;
    let mut archive = ZipArchive::new(file)?;
    let mut output = String::from("member\tcompressed_bytes\tuncompressed_bytes\n");
    let mut shapefiles = 0usize;
    let mut roadlinks = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if !entry.is_dir() {
            std::io::copy(&mut entry, &mut std::io::sink())?;
        }
        if entry.is_dir() || !entry.name().to_ascii_lowercase().ends_with(".shp") {
            continue;
        }
        if entry.name().contains(['\t', '\n', '\r']) || entry.name().contains("..") {
            return Err("unsafe ZIP member name".into());
        }
        shapefiles += 1;
        if entry.name().to_ascii_lowercase().contains("roadlink") {
            roadlinks.push(entry.name().to_owned());
        }
        output.push_str(&format!(
            "{}\t{}\t{}\n",
            entry.name(),
            entry.compressed_size(),
            entry.size()
        ));
    }
    std::fs::write(Path::new(&args[2]), output)?;
    let mut road_output =
        String::from("member\tdbf_records\teasting_min\tnorthing_min\teasting_max\tnorthing_max\n");
    let mut road_records = 0u64;
    for roadlink in &roadlinks {
        let stem = roadlink
            .strip_suffix(".shp")
            .ok_or("invalid RoadLink filename")?;
        let mut shp = [0u8; 100];
        archive.by_name(roadlink)?.read_exact(&mut shp)?;
        let mut dbf = [0u8; 32];
        archive
            .by_name(&format!("{stem}.dbf"))?
            .read_exact(&mut dbf)?;
        let mut prj = String::new();
        archive
            .by_name(&format!("{stem}.prj"))?
            .read_to_string(&mut prj)?;
        if u32::from_be_bytes(shp[..4].try_into()?) != 9994
            || u32::from_le_bytes(shp[28..32].try_into()?) != 1000
            || !matches!(u32::from_le_bytes(shp[32..36].try_into()?), 3 | 13)
            || !prj.contains("British_National_Grid")
            || !prj.contains("AUTHORITY[\"EPSG\",27700]")
        {
            return Err(format!("unexpected RoadLink type or CRS: {roadlink}").into());
        }
        let records = u32::from_le_bytes(dbf[4..8].try_into()?);
        road_records += u64::from(records);
        let bounds: Vec<_> = (0..4)
            .map(|index| {
                let start = 36 + index * 8;
                f64::from_le_bytes(
                    shp[start..start + 8]
                        .try_into()
                        .expect("fixed shape header"),
                )
            })
            .collect();
        if !bounds.iter().all(|value| value.is_finite())
            || bounds[0] >= bounds[2]
            || bounds[1] >= bounds[3]
        {
            return Err(format!("invalid RoadLink bounds: {roadlink}").into());
        }
        road_output.push_str(&format!(
            "{roadlink}\t{records}\t{}\t{}\t{}\t{}\n",
            bounds[0], bounds[1], bounds[2], bounds[3]
        ));
    }
    std::fs::write(Path::new(&args[3]), road_output)?;
    println!(
        "zip_bytes={} sha256={sha256} members={} shapefiles={shapefiles} roadlink_shapefiles={} roadlink_dbf_records={road_records}",
        std::fs::metadata(&args[1])?.len(),
        archive.len(),
        roadlinks.len(),
    );
    Ok(())
}
