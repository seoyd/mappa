use mappa_map_core::{TileKey, project};
use mappa_map_data::{LocalPmTiles, PlaceKind, TileSource, canonical::SourceManifest, decode_mvt};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    path::Path,
};

type Port = (u8, u16);

#[derive(Default)]
struct EdgePorts {
    left: BTreeSet<Port>,
    right: BTreeSet<Port>,
    top: BTreeSet<Port>,
    bottom: BTreeSet<Port>,
}

fn edge_ports(tile: &mappa_map_data::DecodedTile) -> EdgePorts {
    fn crossing(a: f32, b: f32, along_a: f32, along_b: f32, edge: f32) -> Option<u16> {
        if a == b || edge < a.min(b) || edge > a.max(b) {
            return None;
        }
        let along = along_a + (edge - a) * (along_b - along_a) / (b - a);
        (0.0..=4096.0)
            .contains(&along)
            .then_some(along.round() as u16)
    }
    let mut ports = EdgePorts::default();
    for (layer, lines) in [
        (0, &tile.road_major),
        (1, &tile.road_collector),
        (2, &tile.road_local),
    ] {
        for line in lines {
            for segment in line.0.windows(2) {
                let (a, b) = (segment[0], segment[1]);
                for (edge, output) in [(0.0, &mut ports.left), (4096.0, &mut ports.right)] {
                    if let Some(along) = crossing(a.x, b.x, a.y, b.y, edge) {
                        output.insert((layer, along));
                    }
                }
                for (edge, output) in [(0.0, &mut ports.top), (4096.0, &mut ports.bottom)] {
                    if let Some(along) = crossing(a.y, b.y, a.x, b.x, edge) {
                        output.insert((layer, along));
                    }
                }
            }
        }
    }
    ports
}

fn compare_ports(a: &BTreeSet<Port>, b: &BTreeSet<Port>) -> (usize, usize, usize) {
    let exact = a.intersection(b).count();
    let mut remaining: BTreeSet<_> = b.difference(a).copied().collect();
    let mut one_unit = 0;
    let mut unmatched = 0;
    for &port in a.difference(b) {
        if [port.1.checked_sub(1), port.1.checked_add(1)]
            .into_iter()
            .flatten()
            .any(|coordinate| remaining.remove(&(port.0, coordinate)))
        {
            one_unit += 1;
        } else {
            unmatched += 1;
        }
    }
    (exact, one_unit, unmatched + remaining.len())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: audit_canonical_tiles data/sources.toml INPUT.pmtiles".into());
    }
    audit(Path::new(&args[1]), Path::new(&args[2])).await
}

async fn audit(
    manifest_path: &Path,
    archive_path: &Path,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let manifest = SourceManifest::open(manifest_path)?;
    let archive = LocalPmTiles::open(archive_path).await?;
    let [west, south, east, north] = manifest.proof_bbox_wgs84;
    let nw = project(west, north)?;
    let se = project(east, south)?;
    let mut total_unmatched = 0;
    for z in archive.min_zoom..=archive.max_zoom {
        let n = (1u32 << z) as f64;
        let (x0, x1) = ((nw.x * n).floor() as u32, (se.x * n).floor() as u32);
        let (y0, y1) = ((nw.y * n).floor() as u32, (se.y * n).floor() as u32);
        let mut tiles = 0;
        let mut bytes = 0;
        let mut roads = 0;
        let mut surfaces = 0;
        let mut districts = 0;
        let mut edges = BTreeMap::new();
        for y in y0..=y1 {
            for x in x0..=x1 {
                let Some(payload) = archive.tile_bytes(TileKey::new(z, x, y)?).await? else {
                    continue;
                };
                bytes += payload.len();
                tiles += 1;
                let decoded = decode_mvt(payload)?;
                if !decoded.land.is_empty()
                    || !decoded.water.is_empty()
                    || !decoded.green.is_empty()
                    || decoded
                        .place
                        .iter()
                        .any(|place| place.kind != PlaceKind::District)
                {
                    return Err("proof contains an unsupported layer".into());
                }
                roads += decoded.road_major.len()
                    + decoded.road_collector.len()
                    + decoded.road_local.len();
                surfaces += decoded.road_surface.len();
                districts += decoded.place.len();
                edges.insert((x, y), edge_ports(&decoded));
            }
        }
        let mut exact = 0;
        let mut one_unit = 0;
        let mut unmatched = 0;
        let empty = EdgePorts::default();
        for y in y0..=y1 {
            for x in x0..=x1 {
                let edge = edges.get(&(x, y)).unwrap_or(&empty);
                if x < x1 {
                    let next = edges.get(&(x + 1, y)).unwrap_or(&empty);
                    let result = compare_ports(&edge.right, &next.left);
                    exact += result.0;
                    one_unit += result.1;
                    unmatched += result.2;
                }
                if y < y1 {
                    let next = edges.get(&(x, y + 1)).unwrap_or(&empty);
                    let result = compare_ports(&edge.bottom, &next.top);
                    exact += result.0;
                    one_unit += result.1;
                    unmatched += result.2;
                }
            }
        }
        total_unmatched += unmatched;
        println!(
            "z={z} tiles={tiles} decoded_mvt_bytes={bytes} road_lines={roads} road_surfaces={surfaces} district_labels={districts} seam_exact={exact} seam_within_1_unit={one_unit} seam_unmatched={unmatched}"
        );
        if z >= 12
            && districts == 0
            && manifest
                .source
                .iter()
                .any(|source| source.adapter == "sgis-admin-district")
        {
            return Err(format!("z{z} is missing official district labels").into());
        }
    }
    println!("archive_bytes={}", std::fs::metadata(archive_path)?.len());
    if total_unmatched != 0 {
        return Err(format!(
            "{total_unmatched} road seam crossings lack a match within one MVT unit"
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn committed_proof_roads_join_across_tiles() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        audit(
            &root.join("data/sources.toml"),
            &root.join("artifacts/map-v0.3c/naju-roads.pmtiles"),
        )
        .await
        .unwrap();
    }
}
