//! Extract one IGN BD TOPO class from a published 7z into a reproducible ZIP.

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

fn selected_extension(entry: &ArchiveEntry, class: &str) -> Option<&'static str> {
    if entry.is_directory() || entry.size() > 4 * 1024 * 1024 * 1024 {
        return None;
    }
    let name = entry.name().to_ascii_lowercase();
    ["shp", "shx", "dbf", "prj", "cpg"]
        .into_iter()
        .find(|ext| name.ends_with(&format!("{class}.{ext}")))
}

/// Keep only fields used by the canonical adapter, preserving raw values and
/// record order. This avoids parsing unrelated nullable `00000000` dates.
fn compact_dbf(path: &Path, fields: &[&str]) -> Result<u32, Box<dyn Error>> {
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
        if fields.contains(&name) {
            if !seen.insert(name.to_owned()) {
                return Err(format!("duplicate IGN DBF field {name}").into());
            }
            selected.push((bytes.to_vec(), old_offset, length));
        }
        old_offset += length;
    }
    if old_offset != old_record_bytes || seen.len() != fields.len() {
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
    let (class, output_stem) = match args.as_slice() {
        [_, _, _] => ("troncon_de_route", "road"),
        [_, _, _, mode] if mode == "--water" => ("surface_hydrographique", "water"),
        [_, _, _, mode] if mode == "--transport" => ("equipement_de_transport", "transport"),
        _ => {
            return Err(
                "usage: extract_ign_roads PUBLISHED.7z SELECTED.zip [--water|--transport]".into(),
            );
        }
    };
    let archive = Path::new(&args[1]);
    let output = Path::new(&args[2]);
    let upstream_sha256 = sha256(archive)?;
    let temporary = tempfile::tempdir()?;
    let mut found = BTreeSet::new();
    sevenz_rust2::decompress_file_with_extract_fn(
        archive,
        temporary.path(),
        |entry, reader, _| -> Result<bool, SevenZError> {
            let Some(extension) = selected_extension(entry, class) else {
                // Solid 7z blocks require every preceding member to be drained.
                // This also checks the archive's per-member CRC for skipped files.
                if entry.has_stream() {
                    std::io::copy(reader, &mut std::io::sink())?;
                }
                return Ok(true);
            };
            if !found.insert(extension) {
                return Err(SevenZError::Other(
                    format!("duplicate {extension} {class} member").into(),
                ));
            }
            let path = temporary.path().join(format!("{output_stem}.{extension}"));
            let mut file = File::create(path)?;
            let bytes = std::io::copy(reader, &mut file)?;
            if bytes != entry.size() {
                return Err(SevenZError::Other(
                    format!("short {extension} {class} member").into(),
                ));
            }
            Ok(true)
        },
    )?;
    for required in ["shp", "shx", "dbf", "prj"] {
        if !found.contains(required) {
            return Err(format!("missing {required} {class} member").into());
        }
    }
    let selected_fields: &[&str] = if output_stem == "road" {
        &[
            "ID",
            "NATURE",
            "NOM_COLL_G",
            "NOM_COLL_D",
            "IMPORTANCE",
            "FICTIF",
            "ETAT",
            "ACCES_VL",
        ]
    } else if output_stem == "water" {
        &[
            "ID",
            "NATURE",
            "ETAT",
            "PERSISTANC",
            "NOM_P_EAU",
            "NOM_C_EAU",
        ]
    } else {
        &["ID", "NATURE", "NAT_DETAIL", "TOPONYME", "FICTIF", "ETAT"]
    };
    let dbf = temporary.path().join(format!("{output_stem}.dbf"));
    let records = compact_dbf(&dbf, selected_fields)?;
    fs::create_dir_all(output.parent().ok_or("output needs a parent directory")?)?;
    let temporary_zip = output.with_extension("part");
    let mut writer = ZipWriter::new(File::create(&temporary_zip)?);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for extension in ["shp", "shx", "dbf", "prj", "cpg"] {
        if !found.contains(extension) {
            continue;
        }
        writer.start_file(format!("{output_stem}.{extension}"), options)?;
        let mut input = File::open(temporary.path().join(format!("{output_stem}.{extension}")))?;
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
