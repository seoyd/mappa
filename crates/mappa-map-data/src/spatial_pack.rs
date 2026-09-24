//! Experimental Mappa Spatial Pack. Feature geometry is stored once; fixed
//! spatial cells contain only feature ordinals. This is not the default map.

use crate::{
    DecodedTile, MapPlace, PlaceKind,
    canonical::{BBox, CanonicalError, CanonicalFeature, FeatureKind, GeoDb, Geometry},
};
use geo::{Coord, LineString, Point, Polygon};
use mappa_map_core::{MapError, TileKey, project};
use memmap2::{Mmap, MmapOptions};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::Write,
    path::Path,
};
use thiserror::Error;

const MAGIC: &[u8; 8] = b"MAPPASPK";
const VERSION: u32 = 1;
const HEADER_BYTES: usize = 96;
const RECORD_HEADER_BYTES: usize = 48;
const CELL_ZOOM: u8 = 12;
const SCALE: f64 = 10_000_000.0;
const MAX_FEATURES: usize = 1_000_000;
const MAX_RECORD_BYTES: usize = 8 * 1024 * 1024;
const MAX_CELLS: usize = 1_000_000;
const MAX_REFERENCES: usize = 8_000_000;

#[derive(Debug, Error)]
pub enum SpatialPackError {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("GeoDB: {0}")]
    Canonical(#[from] CanonicalError),
    #[error("projection: {0}")]
    Projection(#[from] MapError),
    #[error("corrupt MSP: {0}")]
    Corrupt(&'static str),
}

type Result<T> = std::result::Result<T, SpatialPackError>;

fn u32_at(bytes: &[u8], at: usize) -> Result<u32> {
    let end = at
        .checked_add(4)
        .ok_or(SpatialPackError::Corrupt("offset overflow"))?;
    Ok(u32::from_le_bytes(
        bytes
            .get(at..end)
            .ok_or(SpatialPackError::Corrupt("truncated integer"))?
            .try_into()
            .map_err(|_| SpatialPackError::Corrupt("integer length"))?,
    ))
}

fn u64_at(bytes: &[u8], at: usize) -> Result<u64> {
    let end = at
        .checked_add(8)
        .ok_or(SpatialPackError::Corrupt("offset overflow"))?;
    Ok(u64::from_le_bytes(
        bytes
            .get(at..end)
            .ok_or(SpatialPackError::Corrupt("truncated integer"))?
            .try_into()
            .map_err(|_| SpatialPackError::Corrupt("integer length"))?,
    ))
}

fn take<'a>(bytes: &'a [u8], cursor: &mut usize, count: usize) -> Result<&'a [u8]> {
    let end = cursor
        .checked_add(count)
        .ok_or(SpatialPackError::Corrupt("cursor overflow"))?;
    let result = bytes
        .get(*cursor..end)
        .ok_or(SpatialPackError::Corrupt("truncated section"))?;
    *cursor = end;
    Ok(result)
}

fn varint(out: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        out.push((value as u8) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

fn read_varint(bytes: &[u8], cursor: &mut usize) -> Result<u64> {
    let mut result = 0u64;
    for shift in (0..=63).step_by(7) {
        let byte = *take(bytes, cursor, 1)?
            .first()
            .ok_or(SpatialPackError::Corrupt("empty varint"))?;
        if shift == 63 && byte > 1 {
            return Err(SpatialPackError::Corrupt("varint overflow"));
        }
        result |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
    }
    Err(SpatialPackError::Corrupt("overlong varint"))
}

fn zigzag(value: i64) -> u64 {
    ((value << 1) ^ (value >> 63)) as u64
}

fn unzigzag(value: u64) -> i64 {
    ((value >> 1) as i64) ^ (-((value & 1) as i64))
}

fn quant(value: f64, origin: f64) -> Result<i32> {
    let scaled = ((value - origin) * SCALE).round();
    if !scaled.is_finite() || scaled < i32::MIN as f64 || scaled > i32::MAX as f64 {
        return Err(SpatialPackError::Corrupt(
            "coordinate exceeds regional quantizer",
        ));
    }
    Ok(scaled as i32)
}

fn encode_points(out: &mut Vec<u8>, points: &[[f64; 2]], region: BBox) -> Result<()> {
    varint(out, points.len() as u64);
    let (mut old_x, mut old_y) = (0i64, 0i64);
    for [lon, lat] in points {
        let x = i64::from(quant(*lon, region.west)?);
        let y = i64::from(quant(*lat, region.south)?);
        varint(out, zigzag(x - old_x));
        varint(out, zigzag(y - old_y));
        (old_x, old_y) = (x, y);
    }
    Ok(())
}

fn decode_points(bytes: &[u8], cursor: &mut usize, region: BBox) -> Result<Vec<[f64; 2]>> {
    let count = usize::try_from(read_varint(bytes, cursor)?)
        .map_err(|_| SpatialPackError::Corrupt("point count overflow"))?;
    if count > (bytes.len().saturating_sub(*cursor) / 2) {
        return Err(SpatialPackError::Corrupt("impossible point count"));
    }
    let mut points = Vec::with_capacity(count);
    let (mut x, mut y) = (0i64, 0i64);
    for _ in 0..count {
        x = x
            .checked_add(unzigzag(read_varint(bytes, cursor)?))
            .ok_or(SpatialPackError::Corrupt("coordinate overflow"))?;
        y = y
            .checked_add(unzigzag(read_varint(bytes, cursor)?))
            .ok_or(SpatialPackError::Corrupt("coordinate overflow"))?;
        if x < i32::MIN as i64 || x > i32::MAX as i64 || y < i32::MIN as i64 || y > i32::MAX as i64
        {
            return Err(SpatialPackError::Corrupt("coordinate outside quantizer"));
        }
        points.push([
            region.west + x as f64 / SCALE,
            region.south + y as f64 / SCALE,
        ]);
    }
    Ok(points)
}

fn kind_byte(kind: FeatureKind) -> u8 {
    match kind {
        FeatureKind::RoadPrimary => 0,
        FeatureKind::RoadSecondary => 1,
        FeatureKind::RoadResidential => 2,
        FeatureKind::RoadSurface => 3,
        FeatureKind::Building => 4,
        FeatureKind::Water => 5,
        FeatureKind::Park => 6,
        FeatureKind::Rail => 7,
        FeatureKind::Place => 8,
        FeatureKind::PlaceDistrict => 9,
        FeatureKind::Vegetation => 10,
    }
}

fn byte_kind(value: u8) -> Result<FeatureKind> {
    Ok(match value {
        0 => FeatureKind::RoadPrimary,
        1 => FeatureKind::RoadSecondary,
        2 => FeatureKind::RoadResidential,
        3 => FeatureKind::RoadSurface,
        4 => FeatureKind::Building,
        5 => FeatureKind::Water,
        6 => FeatureKind::Park,
        7 => FeatureKind::Rail,
        8 => FeatureKind::Place,
        9 => FeatureKind::PlaceDistrict,
        10 => FeatureKind::Vegetation,
        _ => return Err(SpatialPackError::Corrupt("unknown feature kind")),
    })
}

fn geometry_bbox(geometry: &Geometry) -> BBox {
    let mut bbox = BBox {
        west: f64::INFINITY,
        south: f64::INFINITY,
        east: f64::NEG_INFINITY,
        north: f64::NEG_INFINITY,
    };
    let mut include = |[x, y]: [f64; 2]| {
        bbox.west = bbox.west.min(x);
        bbox.south = bbox.south.min(y);
        bbox.east = bbox.east.max(x);
        bbox.north = bbox.north.max(y);
    };
    match geometry {
        Geometry::Point(point) => include(*point),
        Geometry::Line(points) => points.iter().copied().for_each(&mut include),
        Geometry::Polygon(rings) => rings.iter().flatten().copied().for_each(&mut include),
    }
    bbox
}

fn encode_feature(
    feature: &CanonicalFeature,
    region: BBox,
    name_ids: &BTreeMap<String, u32>,
) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&feature.id.to_le_bytes());
    bytes.push(kind_byte(feature.kind));
    bytes.push(feature.min_zoom);
    bytes.push(feature.max_zoom);
    bytes.push(match feature.geometry {
        Geometry::Point(_) => 0,
        Geometry::Line(_) => 1,
        Geometry::Polygon(_) => 2,
    });
    bytes.extend_from_slice(&feature.importance.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&feature.revision.to_le_bytes());
    let name = feature
        .name
        .as_ref()
        .map(|n| name_ids[n])
        .unwrap_or(u32::MAX);
    bytes.extend_from_slice(&name.to_le_bytes());
    for (value, origin) in [
        (feature.bbox.west, region.west),
        (feature.bbox.south, region.south),
        (feature.bbox.east, region.west),
        (feature.bbox.north, region.south),
    ] {
        bytes.extend_from_slice(&quant(value, origin)?.to_le_bytes());
    }
    match &feature.geometry {
        Geometry::Point(point) => encode_points(&mut bytes, &[*point], region)?,
        Geometry::Line(points) => encode_points(&mut bytes, points, region)?,
        Geometry::Polygon(rings) => {
            varint(&mut bytes, rings.len() as u64);
            for ring in rings {
                encode_points(&mut bytes, ring, region)?;
            }
        }
    }
    Ok(bytes)
}

fn cell_span(bbox: BBox) -> Result<(u32, u32, u32, u32)> {
    let nw = project(bbox.west, bbox.north)?;
    let se = project(bbox.east, bbox.south)?;
    let n = (1u32 << CELL_ZOOM) as f64;
    let clamp = |v: f64| (v * n).floor().clamp(0.0, n - 1.0) as u32;
    Ok((clamp(nw.x), clamp(se.x), clamp(nw.y), clamp(se.y)))
}

/// Compile a regional MappaGeoDB into an experimental feature-once pack.
pub fn build(
    geodb_path: &Path,
    output: &Path,
    region: BBox,
    attribution: &str,
) -> Result<(usize, usize)> {
    if attribution.is_empty() || attribution.len() > 512 {
        return Err(SpatialPackError::Corrupt("invalid attribution"));
    }
    let mut db = GeoDb::open(geodb_path)?;
    let features = db.query(region)?;
    if features.is_empty() || features.len() > MAX_FEATURES {
        return Err(SpatialPackError::Corrupt("empty or oversized feature set"));
    }
    let names: Vec<String> = features
        .iter()
        .filter_map(|feature| feature.name.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let name_ids = names
        .iter()
        .enumerate()
        .map(|(index, name)| (name.clone(), index as u32))
        .collect::<BTreeMap<_, _>>();
    let mut cells: BTreeMap<(u32, u32), Vec<u32>> = BTreeMap::new();
    let mut out = vec![0; HEADER_BYTES];
    for (ordinal, feature) in features.iter().enumerate() {
        let record = encode_feature(feature, region, &name_ids)?;
        if record.len() > MAX_RECORD_BYTES {
            return Err(SpatialPackError::Corrupt("record exceeds limit"));
        }
        out.extend_from_slice(&(record.len() as u32).to_le_bytes());
        out.extend_from_slice(&record);
        let (x0, x1, y0, y1) = cell_span(feature.bbox)?;
        for y in y0..=y1 {
            for x in x0..=x1 {
                cells.entry((x, y)).or_default().push(ordinal as u32);
            }
        }
    }
    if cells.len() > MAX_CELLS || cells.values().map(Vec::len).sum::<usize>() > MAX_REFERENCES {
        return Err(SpatialPackError::Corrupt("spatial index exceeds limit"));
    }
    let name_offset = out.len() as u64;
    out.extend_from_slice(&(attribution.len() as u32).to_le_bytes());
    out.extend_from_slice(attribution.as_bytes());
    for name in &names {
        out.extend_from_slice(&(name.len() as u32).to_le_bytes());
        out.extend_from_slice(name.as_bytes());
    }
    let cell_offset = out.len() as u64;
    for ((x, y), refs) in &cells {
        out.extend_from_slice(&x.to_le_bytes());
        out.extend_from_slice(&y.to_le_bytes());
        out.extend_from_slice(&(refs.len() as u32).to_le_bytes());
        let mut previous = 0u32;
        for &ordinal in refs {
            varint(&mut out, u64::from(ordinal - previous));
            previous = ordinal;
        }
    }
    let footer_offset = out.len() as u64;
    let mut header = Vec::with_capacity(HEADER_BYTES);
    header.extend_from_slice(MAGIC);
    header.extend_from_slice(&VERSION.to_le_bytes());
    header.extend_from_slice(&(features.len() as u32).to_le_bytes());
    header.extend_from_slice(&(names.len() as u32).to_le_bytes());
    header.extend_from_slice(&(cells.len() as u32).to_le_bytes());
    header.extend_from_slice(&u32::from(CELL_ZOOM).to_le_bytes());
    for value in [region.west, region.south, region.east, region.north] {
        header.extend_from_slice(&value.to_le_bytes());
    }
    for offset in [HEADER_BYTES as u64, name_offset, cell_offset, footer_offset] {
        header.extend_from_slice(&offset.to_le_bytes());
    }
    header.extend_from_slice(&0u32.to_le_bytes());
    if header.len() != HEADER_BYTES {
        return Err(SpatialPackError::Corrupt("header layout"));
    }
    out[..HEADER_BYTES].copy_from_slice(&header);
    let digest = Sha256::digest(&out);
    out.extend_from_slice(&digest);
    let temporary = output.with_extension("msp.tmp");
    let mut file = File::create(&temporary)?;
    file.write_all(&out)?;
    file.sync_all()?;
    fs::rename(temporary, output)?;
    Ok((features.len(), cells.len()))
}

#[derive(Clone, Copy)]
struct Record {
    start: usize,
    end: usize,
    bbox: BBox,
    min_zoom: u8,
    max_zoom: u8,
}

pub struct SpatialPack {
    mmap: Mmap,
    region: BBox,
    attribution: String,
    records: Vec<Record>,
    names: Vec<String>,
    cells: BTreeMap<(u32, u32), Vec<u32>>,
}

impl SpatialPack {
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        // The file is immutable after publication; the checksum below rejects
        // accidental or malicious changes before any mapped record is decoded.
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        let bytes = mmap.as_ref();
        if bytes.len() < HEADER_BYTES + 32 || bytes.get(..8) != Some(MAGIC.as_slice()) {
            return Err(SpatialPackError::Corrupt("short file or bad magic"));
        }
        if u32_at(bytes, 8)? != VERSION || u32_at(bytes, 24)? != u32::from(CELL_ZOOM) {
            return Err(SpatialPackError::Corrupt(
                "unsupported version or cell zoom",
            ));
        }
        let feature_count = u32_at(bytes, 12)? as usize;
        let name_count = u32_at(bytes, 16)? as usize;
        let cell_count = u32_at(bytes, 20)? as usize;
        if feature_count > MAX_FEATURES || name_count > MAX_FEATURES || cell_count > MAX_CELLS {
            return Err(SpatialPackError::Corrupt("section count limit"));
        }
        let mut bounds = [0f64; 4];
        for (index, value) in bounds.iter_mut().enumerate() {
            *value = f64::from_le_bytes(
                bytes[28 + index * 8..36 + index * 8]
                    .try_into()
                    .map_err(|_| SpatialPackError::Corrupt("bounds"))?,
            );
        }
        let region = BBox {
            west: bounds[0],
            south: bounds[1],
            east: bounds[2],
            north: bounds[3],
        };
        if !bounds.iter().all(|v| v.is_finite())
            || region.west >= region.east
            || region.south >= region.north
        {
            return Err(SpatialPackError::Corrupt("invalid bounds"));
        }
        let feature_offset = u64_at(bytes, 60)? as usize;
        let name_offset = u64_at(bytes, 68)? as usize;
        let cell_offset = u64_at(bytes, 76)? as usize;
        let footer_offset = u64_at(bytes, 84)? as usize;
        if feature_offset != HEADER_BYTES
            || feature_offset > name_offset
            || name_offset > cell_offset
            || cell_offset > footer_offset
            || footer_offset.checked_add(32) != Some(bytes.len())
        {
            return Err(SpatialPackError::Corrupt("invalid section offsets"));
        }
        if Sha256::digest(&bytes[..footer_offset]).as_slice() != &bytes[footer_offset..] {
            return Err(SpatialPackError::Corrupt("checksum mismatch"));
        }
        let mut cursor = feature_offset;
        let mut records = Vec::with_capacity(feature_count);
        for _ in 0..feature_count {
            let size = u32_at(bytes, cursor)? as usize;
            cursor += 4;
            if !(RECORD_HEADER_BYTES..=MAX_RECORD_BYTES).contains(&size) {
                return Err(SpatialPackError::Corrupt("record length"));
            }
            let end = cursor
                .checked_add(size)
                .ok_or(SpatialPackError::Corrupt("record offset overflow"))?;
            if end > name_offset {
                return Err(SpatialPackError::Corrupt("record overlaps names"));
            }
            let record = &bytes[cursor..end];
            byte_kind(record[16])?;
            if record[19] > 2 || record[17] > record[18] {
                return Err(SpatialPackError::Corrupt("record type or zoom"));
            }
            let q = |at: usize| i32::from_le_bytes(record[at..at + 4].try_into().unwrap());
            let bbox = BBox {
                west: region.west + q(32) as f64 / SCALE,
                south: region.south + q(36) as f64 / SCALE,
                east: region.west + q(40) as f64 / SCALE,
                north: region.south + q(44) as f64 / SCALE,
            };
            records.push(Record {
                start: cursor,
                end,
                bbox,
                min_zoom: record[17],
                max_zoom: record[18],
            });
            cursor = end;
        }
        if cursor != name_offset {
            return Err(SpatialPackError::Corrupt("feature section length"));
        }
        let attribution_len = u32_at(bytes, cursor)? as usize;
        cursor += 4;
        if attribution_len == 0 || attribution_len > 512 || cursor + attribution_len > cell_offset {
            return Err(SpatialPackError::Corrupt("attribution length"));
        }
        let attribution = std::str::from_utf8(&bytes[cursor..cursor + attribution_len])
            .map_err(|_| SpatialPackError::Corrupt("invalid UTF-8 attribution"))?
            .to_owned();
        cursor += attribution_len;
        let mut names = Vec::with_capacity(name_count);
        for _ in 0..name_count {
            let size = u32_at(bytes, cursor)? as usize;
            cursor += 4;
            if size > 4096 || cursor + size > cell_offset {
                return Err(SpatialPackError::Corrupt("name length"));
            }
            names.push(
                std::str::from_utf8(&bytes[cursor..cursor + size])
                    .map_err(|_| SpatialPackError::Corrupt("invalid UTF-8 name"))?
                    .to_owned(),
            );
            cursor += size;
        }
        if cursor != cell_offset {
            return Err(SpatialPackError::Corrupt("name section length"));
        }
        let mut cells = BTreeMap::new();
        let mut references = 0usize;
        for _ in 0..cell_count {
            if cursor + 12 > footer_offset {
                return Err(SpatialPackError::Corrupt("cell header"));
            }
            let x = u32_at(bytes, cursor)?;
            let y = u32_at(bytes, cursor + 4)?;
            let count = u32_at(bytes, cursor + 8)? as usize;
            cursor += 12;
            references += count;
            if x >= 1 << CELL_ZOOM || y >= 1 << CELL_ZOOM || references > MAX_REFERENCES {
                return Err(SpatialPackError::Corrupt("cell bounds or reference limit"));
            }
            let mut ordinals = Vec::with_capacity(count);
            let mut previous = 0u32;
            for _ in 0..count {
                let delta = read_varint(&bytes[..footer_offset], &mut cursor)?;
                let ordinal = u64::from(previous) + delta;
                if ordinal >= feature_count as u64
                    || ordinals.last().is_some_and(|last| *last >= ordinal as u32)
                {
                    return Err(SpatialPackError::Corrupt("cell reference order"));
                }
                previous = ordinal as u32;
                ordinals.push(previous);
            }
            if cells.insert((x, y), ordinals).is_some() {
                return Err(SpatialPackError::Corrupt("duplicate cell"));
            }
        }
        if cursor != footer_offset {
            return Err(SpatialPackError::Corrupt("cell section length"));
        }
        Ok(Self {
            mmap,
            region,
            attribution,
            records,
            names,
            cells,
        })
    }

    pub fn feature_count(&self) -> usize {
        self.records.len()
    }

    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    pub fn region(&self) -> BBox {
        self.region
    }

    pub fn attribution(&self) -> &str {
        &self.attribution
    }

    pub fn decode(&self, ordinal: usize) -> Result<CanonicalFeature> {
        let record = *self
            .records
            .get(ordinal)
            .ok_or(SpatialPackError::Corrupt("feature ordinal"))?;
        let bytes = &self.mmap[record.start..record.end];
        let id = u128::from_le_bytes(bytes[..16].try_into().unwrap());
        let kind = byte_kind(bytes[16])?;
        let importance = u16::from_le_bytes(bytes[20..22].try_into().unwrap());
        let revision = u32_at(bytes, 24)?;
        let name_id = u32_at(bytes, 28)?;
        let name = if name_id == u32::MAX {
            None
        } else {
            Some(
                self.names
                    .get(name_id as usize)
                    .ok_or(SpatialPackError::Corrupt("name reference"))?
                    .clone(),
            )
        };
        let mut cursor = RECORD_HEADER_BYTES;
        let geometry = match bytes[19] {
            0 => {
                let points = decode_points(bytes, &mut cursor, self.region)?;
                if points.len() != 1 {
                    return Err(SpatialPackError::Corrupt("point geometry count"));
                }
                Geometry::Point(points[0])
            }
            1 => Geometry::Line(decode_points(bytes, &mut cursor, self.region)?),
            2 => {
                let count = usize::try_from(read_varint(bytes, &mut cursor)?)
                    .map_err(|_| SpatialPackError::Corrupt("ring count overflow"))?;
                if count > bytes.len() - cursor {
                    return Err(SpatialPackError::Corrupt("impossible ring count"));
                }
                let mut rings = Vec::with_capacity(count);
                for _ in 0..count {
                    rings.push(decode_points(bytes, &mut cursor, self.region)?);
                }
                Geometry::Polygon(rings)
            }
            _ => return Err(SpatialPackError::Corrupt("geometry kind")),
        };
        if cursor != bytes.len() {
            return Err(SpatialPackError::Corrupt("trailing geometry bytes"));
        }
        Ok(CanonicalFeature {
            id,
            kind,
            bbox: geometry_bbox(&geometry),
            geometry,
            importance,
            min_zoom: record.min_zoom,
            max_zoom: record.max_zoom,
            name,
            revision,
        })
    }

    /// Query references once per fixed cell, then deduplicate crossing features.
    pub fn query(&self, bounds: BBox, zoom: u8) -> Result<Vec<CanonicalFeature>> {
        let (x0, x1, y0, y1) = cell_span(bounds)?;
        let mut ordinals = BTreeSet::new();
        for y in y0..=y1 {
            for x in x0..=x1 {
                if let Some(refs) = self.cells.get(&(x, y)) {
                    ordinals.extend(refs.iter().copied());
                }
            }
        }
        let mut output = Vec::new();
        for ordinal in ordinals {
            let record = &self.records[ordinal as usize];
            if zoom < record.min_zoom
                || zoom > record.max_zoom
                || record.bbox.east + 1.0 / SCALE < bounds.west
                || record.bbox.west - 1.0 / SCALE > bounds.east
                || record.bbox.north + 1.0 / SCALE < bounds.south
                || record.bbox.south - 1.0 / SCALE > bounds.north
            {
                continue;
            }
            output.push(self.decode(ordinal as usize)?);
        }
        Ok(output)
    }

    /// Experimental bridge to the existing Rust/Metal renderer. Geometry stays
    /// whole across tile edges; this is one viewport mesh, not a tile archive.
    pub fn decode_viewport(&self, bounds: BBox, key: TileKey) -> Result<DecodedTile> {
        let mut tile = DecodedTile {
            detailed: true,
            ..Default::default()
        };
        let n = (1u32 << key.z) as f64;
        let local = |[lon, lat]: [f64; 2]| -> Result<Coord<f32>> {
            let p = project(lon, lat)?;
            let x = (p.x * n - key.x as f64) * f64::from(crate::EXTENT);
            let y = (p.y * n - key.y as f64) * f64::from(crate::EXTENT);
            if !x.is_finite() || !y.is_finite() || x.abs() > 1_000_000.0 || y.abs() > 1_000_000.0 {
                return Err(SpatialPackError::Corrupt("viewport geometry range"));
            }
            Ok(Coord {
                x: x as f32,
                y: y as f32,
            })
        };
        for feature in self.query(bounds, key.z)? {
            match (feature.kind, feature.geometry) {
                (FeatureKind::RoadPrimary, Geometry::Line(line)) => {
                    tile.road_major.push(LineString::new(
                        line.into_iter().map(local).collect::<Result<_>>()?,
                    ));
                }
                (FeatureKind::RoadSecondary, Geometry::Line(line)) => {
                    tile.road_collector.push(LineString::new(
                        line.into_iter().map(local).collect::<Result<_>>()?,
                    ));
                }
                (FeatureKind::RoadResidential, Geometry::Line(line)) => {
                    tile.road_local.push(LineString::new(
                        line.into_iter().map(local).collect::<Result<_>>()?,
                    ));
                }
                (
                    FeatureKind::RoadSurface | FeatureKind::Water | FeatureKind::Vegetation,
                    Geometry::Polygon(rings),
                ) => {
                    let mut rings = rings
                        .into_iter()
                        .map(|ring| ring.into_iter().map(local).collect::<Result<Vec<_>>>())
                        .collect::<Result<Vec<_>>>()?
                        .into_iter()
                        .map(LineString::new);
                    let polygon = Polygon::new(
                        rings
                            .next()
                            .ok_or(SpatialPackError::Corrupt("empty polygon"))?,
                        rings.collect(),
                    );
                    match feature.kind {
                        FeatureKind::RoadSurface => tile.road_surface.push(polygon),
                        FeatureKind::Water => tile.water.push(polygon),
                        FeatureKind::Vegetation => tile.green.push(polygon),
                        _ => unreachable!(),
                    }
                }
                (FeatureKind::PlaceDistrict, Geometry::Point(point)) => {
                    tile.place.push(MapPlace {
                        point: Point(local(point)?),
                        name: feature
                            .name
                            .ok_or(SpatialPackError::Corrupt("unnamed place"))?,
                        rank: 4,
                        kind: PlaceKind::District,
                    });
                }
                _ => return Err(SpatialPackError::Corrupt("unsupported proof geometry")),
            }
        }
        Ok(tile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_delta_varints_roundtrip() {
        for value in [i64::MIN, -1_000_000, -1, 0, 1, 1_000_000, i64::MAX] {
            let mut bytes = Vec::new();
            varint(&mut bytes, zigzag(value));
            let mut cursor = 0;
            assert_eq!(unzigzag(read_varint(&bytes, &mut cursor).unwrap()), value);
            assert_eq!(cursor, bytes.len());
        }
    }

    #[test]
    fn committed_proof_pack_roundtrips_and_rejects_corruption() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let source = root.join("artifacts/map-v0.3c/naju-roads.mgeodb");
        let output =
            std::env::temp_dir().join(format!("mappa-spatial-pack-{}.msp", std::process::id()));
        let region = BBox {
            west: 126.69,
            south: 34.99,
            east: 126.78,
            north: 35.08,
        };
        let (count, _) = build(
            &source,
            &output,
            region,
            "나주시 · 국가데이터처 · ESA WorldCover",
        )
        .unwrap();
        let pack = SpatialPack::open(&output).unwrap();
        assert_eq!(count, pack.feature_count());
        let query = pack.query(region, 15).unwrap();
        assert!(!query.is_empty());
        assert!(query.iter().any(|f| f.kind == FeatureKind::RoadPrimary));
        assert!(query.iter().any(|f| f.kind == FeatureKind::Water));
        let mut database = GeoDb::open(&source).unwrap();
        for bounds in [
            region,
            BBox {
                east: (region.west + region.east) / 2.0,
                north: (region.south + region.north) / 2.0,
                ..region
            },
            BBox {
                west: (region.west + region.east) / 2.0,
                south: (region.south + region.north) / 2.0,
                ..region
            },
        ] {
            for zoom in 10..=15 {
                let expected = database
                    .query(bounds)
                    .unwrap()
                    .into_iter()
                    .filter(|feature| (feature.min_zoom..=feature.max_zoom).contains(&zoom))
                    .map(|feature| feature.id)
                    .collect::<BTreeSet<_>>();
                let observed = pack
                    .query(bounds, zoom)
                    .unwrap()
                    .into_iter()
                    .map(|feature| feature.id)
                    .collect::<BTreeSet<_>>();
                assert_eq!(observed, expected, "viewport query mismatch at z{zoom}");
            }
        }
        drop(pack);
        let mut corrupt = fs::read(&output).unwrap();
        corrupt[HEADER_BYTES + 8] ^= 1;
        fs::write(&output, corrupt).unwrap();
        assert!(matches!(
            SpatialPack::open(&output),
            Err(SpatialPackError::Corrupt("checksum mismatch"))
        ));
        fs::remove_file(output).unwrap();
    }
}
