use futures_util::StreamExt;
use mappa_map_core::{TileKey, project};
use mappa_map_data::{LocalPmTiles, PlaceKind, TileSource, canonical::SourceManifest, decode_mvt};
use pmtiles::{AsyncPmTilesReader, TileCoord};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    path::Path,
    sync::Arc,
};

type Port = (u8, u16);
// A shallow road segment can move its interpolated edge crossing several units
// after MVT rounding. Close to a four-tile corner its neighbor is ambiguous;
// report such ports separately instead of certifying a two-tile seam.
const CORNER_AMBIGUITY_UNITS: u16 = 8;

#[derive(Default)]
struct EdgePorts {
    left: BTreeSet<Port>,
    right: BTreeSet<Port>,
    top: BTreeSet<Port>,
    bottom: BTreeSet<Port>,
    left_through: BTreeSet<Port>,
    right_through: BTreeSet<Port>,
    top_through: BTreeSet<Port>,
    bottom_through: BTreeSet<Port>,
    left_uncertainty: BTreeMap<Port, u16>,
    right_uncertainty: BTreeMap<Port, u16>,
    top_uncertainty: BTreeMap<Port, u16>,
    bottom_uncertainty: BTreeMap<Port, u16>,
}

fn edge_ports(tile: &mappa_map_data::DecodedTile) -> EdgePorts {
    fn crossing(a: f32, b: f32, along_a: f32, along_b: f32, edge: f32) -> Option<(u16, bool, u16)> {
        if a == b || edge < a.min(b) || edge > a.max(b) {
            return None;
        }
        let along = along_a + (edge - a) * (along_b - along_a) / (b - a);
        // A corner may belong to four tiles, so it is not a two-tile seam.
        let snapped = along.round();
        // Each encoded endpoint can move by half a unit in both axes. A
        // nearly parallel segment therefore has a wide *inferred* crossing
        // interval. This is uncertainty, not evidence of a matching road.
        let normal_span = (b - a).abs();
        let along_span = (along_b - along_a).abs();
        let uncertainty = if normal_span <= 1.0 {
            4096
        } else {
            (1.0 + along_span / (normal_span - 1.0)).ceil().min(4096.0) as u16
        };
        (0.0 < snapped && snapped < 4096.0).then_some((
            snapped as u16,
            edge > a.min(b) && edge < a.max(b),
            uncertainty,
        ))
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
                for (edge, output, through, uncertainty) in [
                    (
                        0.0,
                        &mut ports.left,
                        &mut ports.left_through,
                        &mut ports.left_uncertainty,
                    ),
                    (
                        4096.0,
                        &mut ports.right,
                        &mut ports.right_through,
                        &mut ports.right_uncertainty,
                    ),
                ] {
                    if let Some((along, crosses, radius)) = crossing(a.x, b.x, a.y, b.y, edge) {
                        let port = (layer, along);
                        output.insert(port);
                        if crosses {
                            through.insert(port);
                            uncertainty
                                .entry(port)
                                .and_modify(|r: &mut u16| *r = (*r).min(radius))
                                .or_insert(radius);
                        }
                    }
                }
                for (edge, output, through, uncertainty) in [
                    (
                        0.0,
                        &mut ports.top,
                        &mut ports.top_through,
                        &mut ports.top_uncertainty,
                    ),
                    (
                        4096.0,
                        &mut ports.bottom,
                        &mut ports.bottom_through,
                        &mut ports.bottom_uncertainty,
                    ),
                ] {
                    if let Some((along, crosses, radius)) = crossing(a.y, b.y, a.x, b.x, edge) {
                        let port = (layer, along);
                        output.insert(port);
                        if crosses {
                            through.insert(port);
                            uncertainty
                                .entry(port)
                                .and_modify(|r: &mut u16| *r = (*r).min(radius))
                                .or_insert(radius);
                        }
                    }
                }
            }
            // Explicit boundary vertices represent a through connection only
            // when the line continues on opposite sides of that boundary.
            for triple in line.0.windows(3) {
                let (before, middle, after) = (triple[0], triple[1], triple[2]);
                for (edge, output, uncertainty) in [
                    (0.0, &mut ports.left_through, &mut ports.left_uncertainty),
                    (
                        4096.0,
                        &mut ports.right_through,
                        &mut ports.right_uncertainty,
                    ),
                ] {
                    if middle.x == edge
                        && ((before.x < edge && after.x > edge)
                            || (before.x > edge && after.x < edge))
                    {
                        let along = middle.y.round();
                        if 0.0 < along && along < 4096.0 {
                            let port = (layer, along as u16);
                            output.insert(port);
                            uncertainty.insert(port, 0);
                        }
                    }
                }
                for (edge, output, uncertainty) in [
                    (0.0, &mut ports.top_through, &mut ports.top_uncertainty),
                    (
                        4096.0,
                        &mut ports.bottom_through,
                        &mut ports.bottom_uncertainty,
                    ),
                ] {
                    if middle.y == edge
                        && ((before.y < edge && after.y > edge)
                            || (before.y > edge && after.y < edge))
                    {
                        let along = middle.x.round();
                        if 0.0 < along && along < 4096.0 {
                            let port = (layer, along as u16);
                            output.insert(port);
                            uncertainty.insert(port, 0);
                        }
                    }
                }
            }
        }
    }
    ports
}

fn compare_ports(
    a_present: &BTreeSet<Port>,
    b_present: &BTreeSet<Port>,
    a_through: &BTreeSet<Port>,
    b_through: &BTreeSet<Port>,
    a_uncertainty: &BTreeMap<Port, u16>,
    b_uncertainty: &BTreeMap<Port, u16>,
) -> (usize, usize, usize, usize, usize, u16) {
    fn match_required(
        required: &BTreeSet<Port>,
        offered: &BTreeSet<Port>,
        uncertainty: &BTreeMap<Port, u16>,
    ) -> (usize, usize, usize, usize, usize, u16) {
        let mut available = offered.clone();
        let mut exact = 0;
        let mut one_unit = 0;
        let mut unmatched = 0;
        let mut corner_ambiguous = 0;
        let mut quantization_ambiguous = 0;
        let mut max_corner_distance = 0;
        for &port in required {
            if available.remove(&port) {
                exact += 1;
            } else if [port.1.checked_sub(1), port.1.checked_add(1)]
                .into_iter()
                .flatten()
                .any(|coordinate| available.remove(&(port.0, coordinate)))
            {
                one_unit += 1;
            } else {
                let distance = port.1.min(4096 - port.1);
                if distance <= CORNER_AMBIGUITY_UNITS {
                    corner_ambiguous += 1;
                } else if uncertainty
                    .get(&port)
                    .is_some_and(|radius| distance <= *radius)
                {
                    quantization_ambiguous += 1;
                } else {
                    unmatched += 1;
                    max_corner_distance = max_corner_distance.max(distance);
                }
            }
        }
        (
            exact,
            one_unit,
            unmatched,
            corner_ambiguous,
            quantization_ambiguous,
            max_corner_distance,
        )
    }
    let forward = match_required(a_through, b_present, a_uncertainty);
    let backward = match_required(b_through, a_present, b_uncertainty);
    (
        forward.0 + backward.0,
        forward.1 + backward.1,
        forward.2 + backward.2,
        forward.3 + backward.3,
        forward.4 + backward.4,
        forward.5.max(backward.5),
    )
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
    let directory = Arc::new(AsyncPmTilesReader::new_with_path(archive_path).await?);
    let mut tile_coords = vec![Vec::new(); usize::from(archive.max_zoom) + 1];
    let mut entries = Arc::clone(&directory).entries();
    while let Some(entry) = entries.next().await {
        for id in entry?.iter_coords() {
            let coord = TileCoord::from(id);
            if coord.z() < archive.min_zoom || coord.z() > archive.max_zoom {
                return Err("archive tile outside declared zoom range".into());
            }
            tile_coords[usize::from(coord.z())].push((coord.x(), coord.y()));
        }
    }
    let [west, south, east, north] = manifest.proof_bbox_wgs84;
    let nw = project(west, north)?;
    let se = project(east, south)?;
    let mut total_unmatched = 0;
    let mut unmatched_samples = Vec::new();
    let mut worst_unmatched = (0, String::new());
    for z in archive.min_zoom..=archive.max_zoom {
        let n = (1u32 << z) as f64;
        let (x0, x1) = ((nw.x * n).floor() as u32, (se.x * n).floor() as u32);
        let (y0, y1) = ((nw.y * n).floor() as u32, (se.y * n).floor() as u32);
        let mut tiles = 0;
        let mut bytes = 0;
        let mut roads = 0;
        let mut surfaces = 0;
        let mut water = 0;
        let mut vegetation = 0;
        let mut districts = 0;
        let mut road_labels = 0;
        let mut distinct_road_names = BTreeSet::new();
        let mut edges = BTreeMap::new();
        for &(x, y) in &tile_coords[usize::from(z)] {
            if x < x0 || x > x1 || y < y0 || y > y1 {
                return Err(format!("tile outside manifest bounds: z{z}/{x}/{y}").into());
            }
            let payload = archive
                .tile_bytes(TileKey::new(z, x, y)?)
                .await?
                .ok_or("archive directory points to a missing tile")?;
            bytes += payload.len();
            tiles += 1;
            let decoded = decode_mvt(payload)?;
            if !decoded.land.is_empty()
                || decoded
                    .place
                    .iter()
                    .any(|place| place.kind != PlaceKind::District)
            {
                return Err("proof contains an unsupported layer".into());
            }
            roads +=
                decoded.road_major.len() + decoded.road_collector.len() + decoded.road_local.len();
            surfaces += decoded.road_surface.len();
            water += decoded.water.len();
            vegetation += decoded.green.len();
            districts += decoded.place.len();
            road_labels += decoded.road_labels.len();
            distinct_road_names.extend(decoded.road_labels.iter().map(|label| label.name.clone()));
            if edges.insert((x, y), edge_ports(&decoded)).is_some() {
                return Err("duplicate tile coordinate in archive directory".into());
            }
        }
        let mut exact = 0;
        let mut one_unit = 0;
        let mut unmatched = 0;
        let mut corner_ambiguous = 0;
        let mut quantization_ambiguous = 0;
        let mut max_corner_distance = 0;
        let empty = EdgePorts::default();
        let mut horizontal_seams = BTreeSet::new();
        let mut vertical_seams = BTreeSet::new();
        for &(x, y) in edges.keys() {
            if x < x1 {
                horizontal_seams.insert((x, y));
            }
            if x > x0 {
                horizontal_seams.insert((x - 1, y));
            }
            if y < y1 {
                vertical_seams.insert((x, y));
            }
            if y > y0 {
                vertical_seams.insert((x, y - 1));
            }
        }
        for (x, y) in horizontal_seams {
            let edge = edges.get(&(x, y)).unwrap_or(&empty);
            let next = edges.get(&(x + 1, y)).unwrap_or(&empty);
            let result = compare_ports(
                &edge.right,
                &next.left,
                &edge.right_through,
                &next.left_through,
                &edge.right_uncertainty,
                &next.left_uncertainty,
            );
            if result.2 > 0 && unmatched_samples.len() < 24 {
                unmatched_samples.push(format!(
                    "z{z}/{x}/{y} right: this={:?} next={:?}",
                    edge.right.iter().take(20).collect::<Vec<_>>(),
                    next.left.iter().take(20).collect::<Vec<_>>()
                ));
            }
            if result.5 > worst_unmatched.0 {
                worst_unmatched = (
                    result.5,
                    format!(
                        "z{z}/{x}/{y} right: this={:?} next={:?} this_through={:?} next_through={:?}",
                        edge.right, next.left, edge.right_through, next.left_through
                    ),
                );
            }
            exact += result.0;
            one_unit += result.1;
            unmatched += result.2;
            corner_ambiguous += result.3;
            quantization_ambiguous += result.4;
            max_corner_distance = max_corner_distance.max(result.5);
        }
        for (x, y) in vertical_seams {
            let edge = edges.get(&(x, y)).unwrap_or(&empty);
            let next = edges.get(&(x, y + 1)).unwrap_or(&empty);
            let result = compare_ports(
                &edge.bottom,
                &next.top,
                &edge.bottom_through,
                &next.top_through,
                &edge.bottom_uncertainty,
                &next.top_uncertainty,
            );
            if result.2 > 0 && unmatched_samples.len() < 24 {
                unmatched_samples.push(format!(
                    "z{z}/{x}/{y} bottom: this={:?} next={:?}",
                    edge.bottom.iter().take(20).collect::<Vec<_>>(),
                    next.top.iter().take(20).collect::<Vec<_>>()
                ));
            }
            if result.5 > worst_unmatched.0 {
                worst_unmatched = (
                    result.5,
                    format!(
                        "z{z}/{x}/{y} bottom: this={:?} next={:?} this_through={:?} next_through={:?}",
                        edge.bottom, next.top, edge.bottom_through, next.top_through
                    ),
                );
            }
            exact += result.0;
            one_unit += result.1;
            unmatched += result.2;
            corner_ambiguous += result.3;
            quantization_ambiguous += result.4;
            max_corner_distance = max_corner_distance.max(result.5);
        }
        total_unmatched += unmatched;
        println!(
            "z={z} tiles={tiles} decoded_mvt_bytes={bytes} road_lines={roads} road_surfaces={surfaces} water={water} tree_cover={vegetation} district_labels={districts} road_labels={road_labels} distinct_road_names={} seam_exact={exact} seam_within_1_unit={one_unit} seam_unmatched={unmatched} seam_corner_ambiguous={corner_ambiguous} seam_quantization_ambiguous={quantization_ambiguous} seam_unmatched_max_corner_distance={max_corner_distance}",
            distinct_road_names.len()
        );
        let expects_named_roads = manifest
            .source
            .iter()
            .any(|source| source.adapter == "no-nvdb-v4-road-links-enriched");
        if expects_named_roads && ((z >= 14 && road_labels == 0) || (z < 14 && road_labels != 0)) {
            return Err(format!("z{z} has unexpected NVDB road-label coverage").into());
        }
        if manifest
            .source
            .iter()
            .any(|source| source.adapter == "esa-worldcover-water")
            && water == 0
        {
            return Err(format!("z{z} is missing ESA permanent water").into());
        }
        if z >= 14
            && manifest
                .source
                .iter()
                .any(|source| source.adapter == "esa-worldcover-tree")
            && vegetation == 0
        {
            return Err(format!("z{z} is missing ESA tree cover").into());
        }
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
            "{total_unmatched} road seam crossings lack a match within one MVT unit; worst corner distance {}: {}; samples:\n{}",
            worst_unmatched.0,
            worst_unmatched.1,
            unmatched_samples.join("\n")
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn through_road_matches_neighbor_that_follows_the_edge() {
        let mut tile = mappa_map_data::DecodedTile::default();
        tile.road_local.push(geo::LineString::from(vec![
            (-13.0_f32, 1640.0_f32),
            (0.0, 52.0),
            (0.0, 0.0),
            (1.0, -32.0),
        ]));
        let edge = edge_ports(&tile);
        assert!(edge.left.contains(&(2, 52)));
        assert!(!edge.left_through.contains(&(2, 52)));
        let required = BTreeSet::from([(2, 52)]);
        assert_eq!(
            compare_ports(
                &required,
                &edge.left,
                &required,
                &edge.left_through,
                &BTreeMap::new(),
                &edge.left_uncertainty,
            )
            .2,
            0
        );
        assert_eq!(
            compare_ports(
                &required,
                &BTreeSet::new(),
                &required,
                &BTreeSet::new(),
                &BTreeMap::new(),
                &BTreeMap::new(),
            )
            .2,
            1
        );
    }

    #[test]
    fn unresolved_four_tile_corner_is_reported_separately() {
        let required = BTreeSet::from([(2, 4089)]);
        let result = compare_ports(
            &required,
            &BTreeSet::new(),
            &required,
            &BTreeSet::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
        );
        assert_eq!(result.2, 0);
        assert_eq!(result.3, 1);
        let interior = BTreeSet::from([(2, 4087)]);
        let result = compare_ports(
            &interior,
            &BTreeSet::new(),
            &interior,
            &BTreeSet::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
        );
        assert_eq!(result.2, 1);
        assert_eq!(result.3, 0);
    }

    #[test]
    fn shallow_quantized_crossing_is_uncertain_but_explicit_vertex_is_not() {
        let mut tile = mappa_map_data::DecodedTile::default();
        tile.road_local.push(geo::LineString::from(vec![
            (4128.0_f32, 1.0_f32),
            (3775.0, -6.0),
        ]));
        let ports = edge_ports(&tile);
        let crossing = BTreeSet::from([(2, 4078)]);
        assert!(ports.top_through.contains(&(2, 4078)));
        let ambiguous = compare_ports(
            &crossing,
            &BTreeSet::new(),
            &crossing,
            &BTreeSet::new(),
            &ports.top_uncertainty,
            &BTreeMap::new(),
        );
        assert_eq!(ambiguous.2, 0);
        assert_eq!(ambiguous.4, 1);
        let explicit = BTreeMap::from([((2, 4078), 0)]);
        let unmatched = compare_ports(
            &crossing,
            &BTreeSet::new(),
            &crossing,
            &BTreeSet::new(),
            &explicit,
            &BTreeMap::new(),
        );
        assert_eq!(unmatched.2, 1);
        assert_eq!(unmatched.4, 0);
    }

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
