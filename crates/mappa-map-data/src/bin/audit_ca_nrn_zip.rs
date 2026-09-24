//! Inspect a Canadian NRN province ZIP before approving a road adapter.

use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs::File,
    io::Read,
    path::Path,
};
use zip::ZipArchive;

fn member(archive: &mut ZipArchive<File>, name: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut item = archive.by_name(name)?;
    if item.size() > 512 * 1024 * 1024 {
        return Err(format!("NRN member exceeds 512 MiB: {name}").into());
    }
    let mut bytes = Vec::with_capacity(item.size() as usize);
    item.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: audit_ca_nrn_zip SOURCE.zip PROVINCE_CODE".into());
    }
    let province = &args[2];
    if province.len() != 2 || !province.bytes().all(|byte| byte.is_ascii_uppercase()) {
        return Err("province code must be two uppercase letters".into());
    }
    let mut file = File::open(Path::new(&args[1]))?;
    let mut sha = Sha256::new();
    std::io::copy(&mut file, &mut sha)?;
    let source_sha256 = format!("{:x}", sha.finalize());
    let mut archive = ZipArchive::new(File::open(Path::new(&args[1]))?)?;
    let zip_members = archive.len();
    for index in 0..zip_members {
        let mut item = archive.by_index(index)?;
        std::io::copy(&mut item, &mut std::io::sink())?;
    }
    let suffix = format!("_SHAPE_en/NRN_{province}_");
    let stems: Vec<String> = archive
        .file_names()
        .filter(|name| name.contains(&suffix) && name.ends_with("_ROADSEG.shp"))
        .map(|name| name.trim_end_matches(".shp").to_owned())
        .collect();
    let [stem] = stems.as_slice() else {
        return Err("expected one English NRN ROADSEG Shapefile in ZIP".into());
    };
    let prj = member(&mut archive, &format!("{stem}.prj"))?;
    let prj = std::str::from_utf8(&prj)?;
    let shp = member(&mut archive, &format!("{stem}.shp"))?;
    let dbf = member(&mut archive, &format!("{stem}.dbf"))?;
    let shape_reader = shapefile::ShapeReader::new(std::io::Cursor::new(shp))?;
    let attr_reader = shapefile::dbase::Reader::new(std::io::Cursor::new(dbf))?;
    let mut reader = shapefile::Reader::new(shape_reader, attr_reader);
    let mut classes = BTreeMap::<String, usize>::new();
    let mut shape_types = BTreeMap::<String, usize>::new();
    let mut count = 0usize;
    let mut nids = BTreeSet::new();
    let mut roadseg_ids = BTreeSet::new();
    for row in reader.iter_shapes_and_records() {
        let (shape, attributes) = row?;
        *classes
            .entry(format!("{:?}", attributes.get("ROADCLASS")))
            .or_default() += 1;
        *shape_types
            .entry(format!("{:?}", shape.shapetype()))
            .or_default() += 1;
        nids.insert(format!("{:?}", attributes.get("NID")));
        roadseg_ids.insert(format!("{:?}", attributes.get("ROADSEGID")));
        count += 1;
    }
    println!("source_sha256={source_sha256} province={province} records={count}");
    println!("zip_members={zip_members} crc_failures=0");
    println!("roadseg_stem={stem}");
    println!("prj={prj}");
    println!("shape_types={shape_types:?}");
    println!(
        "unique_nids={} unique_roadseg_ids={}",
        nids.len(),
        roadseg_ids.len()
    );
    if roadseg_ids.len() != count {
        return Err("NRN ROADSEGID is not unique in source ROADSEG".into());
    }
    println!("road_classes={classes:?}");
    Ok(())
}
