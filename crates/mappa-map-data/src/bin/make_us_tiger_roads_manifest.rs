//! Produce an approved, checksum-pinned county manifest from a publisher file inventory.

use mappa_map_data::canonical::{SourceManifest, SourceRecord};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    error::Error,
    fs::{self, File},
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
        return Err(format!("expected exactly one {suffix} in source ZIP").into());
    };
    let mut member = archive.by_name(name)?;
    if member.size() > 64 * 1024 * 1024 {
        return Err(format!("{suffix} exceeds 64 MiB audit limit").into());
    }
    let mut bytes = Vec::with_capacity(member.size() as usize);
    member.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn audit(path: &Path) -> Result<(String, [f64; 4], u32), Box<dyn Error>> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let hash = format!("{:x}", hasher.finalize());
    file.rewind()?;
    let mut archive = ZipArchive::new(file)?;
    let shp = member(&mut archive, ".shp")?;
    let dbf = member(&mut archive, ".dbf")?;
    let prj = member(&mut archive, ".prj")?;
    if shp.len() < 100
        || dbf.len() < 32
        || u32::from_be_bytes(shp[0..4].try_into()?) != 9994
        || u32::from_le_bytes(shp[28..32].try_into()?) != 1000
        || u32::from_le_bytes(shp[32..36].try_into()?) != 3
        || !std::str::from_utf8(&prj)?.contains("GCS_North_American_1983")
    {
        return Err(format!(
            "unexpected TIGER All Roads header or CRS: {}",
            path.display()
        )
        .into());
    }
    let mut bounds = [0.0; 4];
    for (index, value) in bounds.iter_mut().enumerate() {
        let start = 36 + index * 8;
        *value = f64::from_le_bytes(shp[start..start + 8].try_into()?);
    }
    if !bounds.iter().all(|value| value.is_finite())
        || bounds[0] >= bounds[2]
        || bounds[1] >= bounds[3]
        || bounds[0] < -180.0
        || bounds[2] > 180.0
        || bounds[1] < -85.051_128_78
        || bounds[3] > 85.051_128_78
    {
        return Err(format!("invalid source bounds: {}", path.display()).into());
    }
    Ok((hash, bounds, u32::from_le_bytes(dbf[4..8].try_into()?)))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err(
            "usage: make_us_tiger_roads_manifest INVENTORY.txt SOURCE_DIR OUTPUT.toml DOWNLOAD_DATE(YYYY-MM-DD)".into(),
        );
    }
    let inventory = Path::new(&args[1]);
    let source_dir = Path::new(&args[2]);
    let output = Path::new(&args[3]);
    let download_date = &args[4];
    if download_date.len() != 10
        || !download_date.bytes().enumerate().all(|(index, byte)| {
            if index == 4 || index == 7 {
                byte == b'-'
            } else {
                byte.is_ascii_digit()
            }
        })
    {
        return Err("invalid download date".into());
    }
    let parent = output.parent().ok_or("manifest needs a parent directory")?;
    let relative_dir = source_dir
        .strip_prefix(parent)
        .map_err(|_| "source directory must be inside the manifest parent")?;
    let mut names = BTreeSet::new();
    for line in fs::read_to_string(inventory)?.lines() {
        let name = line.trim();
        if name.is_empty() || name.starts_with('#') {
            continue;
        }
        let Some(county) = name
            .strip_prefix("tl_2025_")
            .and_then(|rest| rest.strip_suffix("_roads.zip"))
        else {
            return Err(format!("unexpected county source name: {name}").into());
        };
        if county.len() != 5 || !county.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(format!("invalid county FIPS in source name: {name}").into());
        }
        if !names.insert(name.to_owned()) {
            return Err(format!("duplicate inventory name: {name}").into());
        }
    }
    if names.is_empty() {
        return Err("empty county source inventory".into());
    }
    let mut source = Vec::with_capacity(names.len());
    let mut union = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    let mut total_records = 0u64;
    let mut state_code: Option<String> = None;
    for name in names {
        let (sha256, bounds, records) = audit(&source_dir.join(&name))?;
        total_records += u64::from(records);
        union[0] = union[0].min(bounds[0]);
        union[1] = union[1].min(bounds[1]);
        union[2] = union[2].max(bounds[2]);
        union[3] = union[3].max(bounds[3]);
        let county = &name[8..13];
        let state = &county[..2];
        if state_code
            .as_deref()
            .is_some_and(|existing| existing != state)
        {
            return Err("inventory mixes multiple state FIPS codes".into());
        }
        state_code = Some(state.to_owned());
        source.push(SourceRecord {
            id: format!("us-census-tiger-2025-roads-{county}"),
            name: format!("2025 TIGER/Line All Roads, county FIPS {county}"),
            provider: "U.S. Census Bureau".into(),
            source_version: "2025 TIGER/Line, legal boundaries as of 2025-01-01".into(),
            download_date: download_date.clone(),
            official_url: format!(
                "https://www2.census.gov/geo/tiger/TIGER2025/ROADS/{name}"
            ),
            download_page_url: "https://www.census.gov/geographies/mapping-files/time-series/geo/tiger-line-file.html".into(),
            file: relative_dir.join(&name).to_string_lossy().into_owned(),
            sha256,
            upstream_file: None,
            upstream_sha256: None,
            dedup_file: None,
            dedup_sha256: None,
            crs: "EPSG:4269 NAD83; numeric degrees retained; WGS84 datum transform and positional accuracy pending".into(),
            format: "ZIP Shapefile, county All Roads".into(),
            coverage: format!("County FIPS {county}, source header bounds {bounds:?}; selected road classes only"),
            resolution: "official TIGER/Line linear feature geometry; source accuracy varies".into(),
            update_frequency: "annual publication; pinned 2025 snapshot".into(),
            license_id: "US-GOV-PUBLIC-DOMAIN".into(),
            license_url: "https://www2.census.gov/geo/pdfs/maps-data/data/tiger/tgrshp2025/TGRSHP2025_TechDoc.pdf".into(),
            license_status: "APPROVED_WITH_ATTRIBUTION".into(),
            commercial_use: true,
            modification: true,
            redistribution: true,
            attribution_required: true,
            attribution_text: Some("U.S. Census Bureau, 2025 TIGER/Line All Roads; Mappa modifications".into()),
            share_alike: false,
            adapter: "us-census-tiger-roads".into(),
            adapter_version: 1,
        });
    }
    let manifest = SourceManifest {
        schema_version: 1,
        proof_region: format!(
            "us-tiger-2025-state-{}-{}-county-roads",
            state_code.as_deref().ok_or("empty state inventory")?,
            source.len()
        ),
        proof_bbox_wgs84: union,
        source,
    };
    fs::write(output, toml::to_string_pretty(&manifest)?)?;
    let checked = SourceManifest::open(output)?;
    println!(
        "manifest={} sources={} dbf_records={} source_numeric_union={union:?}",
        output.display(),
        checked.source.len(),
        total_records
    );
    Ok(())
}
