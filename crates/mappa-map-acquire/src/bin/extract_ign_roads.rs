//! Extract one IGN BD TOPO road class from a published 7z into a reproducible ZIP.

use sevenz_rust2::{ArchiveEntry, Error as SevenZError};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    error::Error,
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

fn sha256(path: &Path) -> Result<String, Box<dyn Error>> {
    let mut input = File::open(path)?;
    let mut hash = Sha256::new();
    std::io::copy(&mut input, &mut hash)?;
    Ok(format!("{:x}", hash.finalize()))
}

fn selected_extension(entry: &ArchiveEntry) -> Option<&'static str> {
    if entry.is_directory() || entry.size() > 4 * 1024 * 1024 * 1024 {
        return None;
    }
    let name = entry.name().to_ascii_lowercase();
    ["shp", "shx", "dbf", "prj", "cpg"]
        .into_iter()
        .find(|ext| name.ends_with(&format!("troncon_de_route.{ext}")))
}

/// The published DBF includes nullable date columns with `00000000` values.
/// Keep only road fields used by the canonical adapter, preserving their raw
/// values and record order. This avoids interpreting unrelated invalid dates.
fn compact_road_dbf(path: &Path) -> Result<u32, Box<dyn Error>> {
    const FIELDS: [&str; 8] = [
        "ID",
        "NATURE",
        "NOM_COLL_G",
        "NOM_COLL_D",
        "IMPORTANCE",
        "FICTIF",
        "ETAT",
        "ACCES_VL",
    ];
    let mut input = File::open(path)?;
    let mut header = [0u8; 32];
    input.read_exact(&mut header)?;
    let count = u32::from_le_bytes(header[4..8].try_into()?);
    let old_header_bytes = u16::from_le_bytes(header[8..10].try_into()?) as usize;
    let old_record_bytes = u16::from_le_bytes(header[10..12].try_into()?) as usize;
    if old_header_bytes < 33 || !(old_header_bytes - 33).is_multiple_of(32) || old_record_bytes < 2
    {
        return Err("unexpected IGN DBF header".into());
    }
    let mut descriptors = vec![0u8; old_header_bytes - 32];
    input.read_exact(&mut descriptors)?;
    if descriptors.last() != Some(&0x0d) {
        return Err("IGN DBF field terminator missing".into());
    }
    let mut selected = Vec::new();
    let mut seen = BTreeSet::new();
    let mut old_offset = 1usize;
    for bytes in descriptors[..descriptors.len() - 1].as_chunks::<32>().0 {
        let name_end = bytes[..11].iter().position(|byte| *byte == 0).unwrap_or(11);
        let name = std::str::from_utf8(&bytes[..name_end])?;
        let length = usize::from(bytes[16]);
        if FIELDS.contains(&name) {
            if !seen.insert(name.to_owned()) {
                return Err(format!("duplicate IGN DBF field {name}").into());
            }
            selected.push((bytes.to_vec(), old_offset, length));
        }
        old_offset += length;
    }
    if old_offset != old_record_bytes || seen.len() != FIELDS.len() {
        return Err(format!("IGN DBF schema differs: found {seen:?}").into());
    }
    let new_header_bytes = 32 + selected.len() * 32 + 1;
    let new_record_bytes = 1 + selected.iter().map(|(_, _, length)| length).sum::<usize>();
    header[8..10].copy_from_slice(&(new_header_bytes as u16).to_le_bytes());
    header[10..12].copy_from_slice(&(new_record_bytes as u16).to_le_bytes());
    let compact = path.with_extension("compact");
    let mut output = File::create(&compact)?;
    output.write_all(&header)?;
    let mut new_offset = 1u32;
    for (descriptor, _, length) in &mut selected {
        descriptor[12..16].copy_from_slice(&new_offset.to_le_bytes());
        output.write_all(descriptor)?;
        new_offset += *length as u32;
    }
    output.write_all(&[0x0d])?;
    let mut row = vec![0u8; old_record_bytes];
    for _ in 0..count {
        input.read_exact(&mut row)?;
        output.write_all(&row[..1])?;
        for (_, offset, length) in &selected {
            output.write_all(&row[*offset..*offset + *length])?;
        }
    }
    output.write_all(&[0x1a])?;
    output.sync_all()?;
    fs::rename(compact, path)?;
    Ok(count)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: extract_ign_roads PUBLISHED.7z ROAD_ONLY.zip".into());
    }
    let archive = Path::new(&args[1]);
    let output = Path::new(&args[2]);
    let upstream_sha256 = sha256(archive)?;
    let temporary = tempfile::tempdir()?;
    let mut found = BTreeSet::new();
    sevenz_rust2::decompress_file_with_extract_fn(
        archive,
        temporary.path(),
        |entry, reader, _| -> Result<bool, SevenZError> {
            let Some(extension) = selected_extension(entry) else {
                // Solid 7z blocks require every preceding member to be drained.
                // This also checks the archive's per-member CRC for skipped files.
                if entry.has_stream() {
                    std::io::copy(reader, &mut std::io::sink())?;
                }
                return Ok(true);
            };
            if !found.insert(extension) {
                return Err(SevenZError::Other(
                    format!("duplicate {extension} road member").into(),
                ));
            }
            let path = temporary.path().join(format!("road.{extension}"));
            let mut file = File::create(path)?;
            let bytes = std::io::copy(reader, &mut file)?;
            if bytes != entry.size() {
                return Err(SevenZError::Other(
                    format!("short {extension} road member").into(),
                ));
            }
            Ok(true)
        },
    )?;
    for required in ["shp", "shx", "dbf", "prj"] {
        if !found.contains(required) {
            return Err(format!("missing {required} road member").into());
        }
    }
    let records = compact_road_dbf(&temporary.path().join("road.dbf"))?;
    fs::create_dir_all(output.parent().ok_or("output needs a parent directory")?)?;
    let temporary_zip = output.with_extension("part");
    let mut writer = ZipWriter::new(File::create(&temporary_zip)?);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for extension in ["shp", "shx", "dbf", "prj", "cpg"] {
        if !found.contains(extension) {
            continue;
        }
        writer.start_file(format!("road.{extension}"), options)?;
        let mut input = File::open(temporary.path().join(format!("road.{extension}")))?;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let read = input.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            writer.write_all(&buffer[..read])?;
        }
    }
    writer.finish()?;
    fs::rename(&temporary_zip, output)?;
    println!(
        "upstream_sha256={upstream_sha256} selected_members={} dbf_records={records} output={} source_sha256={}",
        found.len(),
        output.display(),
        sha256(output)?
    );
    Ok(())
}
