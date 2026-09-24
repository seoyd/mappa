//! Compare official RoadLink identifiers and planar geometry across source grids.

use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeSet, HashMap},
    error::Error,
    fs::File,
    io::Read,
    path::Path,
};
use zip::ZipArchive;

type Fingerprints = HashMap<String, ([u8; 32], String, String)>;

fn grid_records(
    archive: &mut ZipArchive<File>,
    grid: &str,
) -> Result<Fingerprints, Box<dyn Error>> {
    if grid.len() != 2 || !grid.bytes().all(|byte| byte.is_ascii_uppercase()) {
        return Err("grid must be two uppercase letters".into());
    }
    let mut read_member = |suffix: &str| -> Result<Vec<u8>, Box<dyn Error>> {
        let mut entry = archive.by_name(&format!("data/{grid}_RoadLink{suffix}"))?;
        if entry.size() > 512 * 1024 * 1024 {
            return Err("RoadLink member exceeds 512 MiB".into());
        }
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut bytes)?;
        Ok(bytes)
    };
    let shp = read_member(".shp")?;
    let dbf = read_member(".dbf")?;
    let shape_reader = shapefile::ShapeReader::new(std::io::Cursor::new(shp))?;
    let attribute_reader = shapefile::dbase::Reader::new(std::io::Cursor::new(dbf))?;
    let mut reader = shapefile::Reader::new(shape_reader, attribute_reader);
    let mut records = HashMap::new();
    for record in reader.iter_shapes_and_records() {
        let (shape, attributes) = record?;
        let field = |name: &str| -> Result<&str, Box<dyn Error>> {
            match attributes.get(name) {
                Some(shapefile::dbase::FieldValue::Character(Some(value))) => Ok(value.trim()),
                _ => Err(format!("missing RoadLink {name}").into()),
            }
        };
        let id = field("identifier")?.to_owned();
        let function = field("function")?.to_owned();
        let name = match attributes.get("name1") {
            Some(shapefile::dbase::FieldValue::Character(Some(value))) => value.trim().to_owned(),
            _ => String::new(),
        };
        let shapefile::Shape::PolylineZ(line) = shape else {
            return Err("RoadLink is not PolylineZ".into());
        };
        let mut hash = Sha256::new();
        for part in line.parts() {
            hash.update((part.len() as u64).to_le_bytes());
            for point in part {
                hash.update(point.x.to_le_bytes());
                hash.update(point.y.to_le_bytes());
            }
        }
        if records
            .insert(id, (hash.finalize().into(), function, name))
            .is_some()
        {
            return Err(format!("duplicate RoadLink identifier within {grid}").into());
        }
    }
    Ok(records)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: audit_os_open_roads_grid_overlap SOURCE.zip FIRST_GRID SECOND_GRID | SOURCE.zip --all OWNER.tsv".into());
    }
    let mut file = File::open(Path::new(&args[1]))?;
    let mut source_hash = Sha256::new();
    std::io::copy(&mut file, &mut source_hash)?;
    let source_sha256 = format!("{:x}", source_hash.finalize());
    let mut archive = ZipArchive::new(File::open(Path::new(&args[1]))?)?;
    if args[2] == "--all" {
        let grids: BTreeSet<_> = archive
            .file_names()
            .filter_map(|name| {
                Some(
                    name.strip_prefix("data/")?
                        .strip_suffix("_RoadLink.shp")?
                        .to_owned(),
                )
            })
            .collect();
        if grids.is_empty() {
            return Err("national ZIP has no RoadLink grids".into());
        }
        let mut owners: HashMap<String, ([u8; 32], String, String, String)> = HashMap::new();
        let mut duplicate_rows = Vec::new();
        let mut total_records = 0usize;
        let mut changed_name = 0usize;
        for grid in &grids {
            let records = grid_records(&mut archive, grid)?;
            total_records += records.len();
            for (id, (geometry, function, name)) in records {
                if let Some((previous_geometry, previous_function, previous_name, owner)) =
                    owners.get(&id)
                {
                    if previous_geometry != &geometry || previous_function != &function {
                        return Err(format!(
                            "RoadLink {id} differs between {owner} and {grid}; cannot deduplicate"
                        )
                        .into());
                    }
                    changed_name += usize::from(previous_name != &name);
                    duplicate_rows.push((id, owner.clone(), grid.clone()));
                } else {
                    owners.insert(id, (geometry, function, name, grid.clone()));
                }
            }
        }
        if changed_name != 0 {
            return Err(
                "RoadLink names differ across duplicate grid records; cannot deduplicate".into(),
            );
        }
        duplicate_rows.sort();
        let mut output =
            format!("# source_sha256={source_sha256}\nidentifier\towner_grid\tduplicate_grid\n");
        for (id, owner, duplicate) in &duplicate_rows {
            output.push_str(&format!("{id}\t{owner}\t{duplicate}\n"));
        }
        std::fs::write(&args[3], output)?;
        println!(
            "source_sha256={source_sha256} grids={} source_records={total_records} unique_ids={} duplicate_records={} changed_geometry_or_function=0 changed_name={changed_name}",
            grids.len(),
            owners.len(),
            duplicate_rows.len(),
        );
        return Ok(());
    }
    let first = grid_records(&mut archive, &args[2])?;
    let second = grid_records(&mut archive, &args[3])?;
    let mut shared = 0usize;
    let mut same_geometry_and_function = 0usize;
    let mut changed_geometry = 0usize;
    let mut changed_function = 0usize;
    let mut changed_name = 0usize;
    for (id, (second_geometry, second_function, second_name)) in &second {
        if let Some((first_geometry, first_function, first_name)) = first.get(id) {
            shared += 1;
            if first_geometry == second_geometry && first_function == second_function {
                same_geometry_and_function += 1;
            }
            changed_geometry += usize::from(first_geometry != second_geometry);
            changed_function += usize::from(first_function != second_function);
            changed_name += usize::from(first_name != second_name);
        }
    }
    println!(
        "source_sha256={source_sha256} first_grid={} first_ids={} second_grid={} second_ids={} shared_ids={} same_geometry_and_function={} changed_geometry={} changed_function={} changed_name={changed_name}",
        args[2],
        first.len(),
        args[3],
        second.len(),
        shared,
        same_geometry_and_function,
        changed_geometry,
        changed_function,
    );
    Ok(())
}
