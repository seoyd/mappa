//! Audit the pinned Microsoft source index without claiming map completeness.
use flate2::read::GzDecoder;
use mappa_map_core::project;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    io::Read,
    path::Path,
};

const OFFICIAL_INDEX_SHA256: &str =
    "de61b569b0c23c364d61173632d8c5487ea30bf20912c55b9fa1ab4a89d01148";

fn point_quadkey(lon: f64, lat: f64) -> Result<String, Box<dyn Error + Send + Sync>> {
    let point = project(lon, lat)?;
    let n = 1u32 << 9;
    let x = (point.x * f64::from(n))
        .floor()
        .clamp(0.0, f64::from(n - 1)) as u32;
    let y = (point.y * f64::from(n))
        .floor()
        .clamp(0.0, f64::from(n - 1)) as u32;
    Ok((0..9)
        .rev()
        .map(|shift| char::from(b'0' + (((x >> shift) & 1) + 2 * ((y >> shift) & 1)) as u8))
        .collect())
}

fn approximate_bytes(value: &str) -> Result<f64, Box<dyn Error + Send + Sync>> {
    for (suffix, multiplier) in [
        ("GB", 1_000_000_000.0),
        ("MB", 1_000_000.0),
        ("KB", 1_000.0),
        ("B", 1.0),
    ] {
        if let Some(number) = value.strip_suffix(suffix) {
            let parsed: f64 = number.parse()?;
            if !parsed.is_finite() || parsed < 0.0 {
                return Err("invalid index size".into());
            }
            return Ok(parsed * multiplier);
        }
    }
    Err("invalid index size unit".into())
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 && args.len() != 4 {
        return Err("usage: audit_building_index INDEX.csv.gz [LON LAT]".into());
    }
    let query = if args.len() == 4 {
        Some(point_quadkey(args[2].parse()?, args[3].parse()?)?)
    } else {
        None
    };
    let mut csv = Vec::new();
    GzDecoder::new(std::fs::File::open(Path::new(&args[1]))?).read_to_end(&mut csv)?;
    let digest = format!("{:x}", Sha256::digest(&csv));
    if digest != OFFICIAL_INDEX_SHA256 {
        return Err("official Microsoft index SHA-256 mismatch".into());
    }
    let contents = std::str::from_utf8(&csv)?;
    let mut lines = contents.lines();
    if lines.next() != Some("Location,QuadKey,Url,Size,UploadDate") {
        return Err("unexpected Microsoft index header".into());
    }
    let mut rows = 0usize;
    let mut estimated_source_bytes = 0.0;
    let mut locations = BTreeSet::new();
    let mut quadkeys = BTreeSet::new();
    let mut coarse_cells = BTreeSet::new();
    let mut matches = BTreeMap::<String, String>::new();
    for line in lines {
        let fields = line.split(',').collect::<Vec<_>>();
        let [location, quadkey, url, size, date] = fields.as_slice() else {
            return Err("unexpected Microsoft index row format".into());
        };
        if quadkey.len() != 9
            || !quadkey.bytes().all(|digit| (b'0'..=b'3').contains(&digit))
            || !url.starts_with("https://bfppub.z5.web.core.windows.net/2026-08-13/")
            || !url.contains(&format!("/quadkey={quadkey}/"))
            || *date != "2026-08-13"
        {
            return Err("invalid index quadkey, URL or release date".into());
        }
        rows += 1;
        estimated_source_bytes += approximate_bytes(size)?;
        locations.insert(*location);
        quadkeys.insert(*quadkey);
        coarse_cells.insert(&quadkey[..3]);
        if query.as_deref() == Some(*quadkey) {
            matches.insert((*location).to_string(), (*url).to_string());
        }
    }
    println!(
        "release=2026-08-13 rows={rows} unique_z9_quadkeys={} publisher_regions={} occupied_z3_cells={}/64 listed_compressed_gb_approx={:.1} index_sha256={digest}",
        quadkeys.len(),
        locations.len(),
        coarse_cells.len(),
        estimated_source_bytes / 1_000_000_000.0,
    );
    if let Some(quadkey) = query {
        println!("query_z9_quadkey={quadkey} source_files={}", matches.len());
        for (location, url) in matches {
            println!("location={location} url={url}");
        }
    }
    Ok(())
}
