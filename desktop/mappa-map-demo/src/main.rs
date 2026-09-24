use mappa_map_core::{MapCamera, TileKey, VisibleTile, WorldPoint, project, unproject};
use mappa_map_data::{
    CountryLabel, DecodedTile, LocalPmTiles, PlaceKind, TileSource, decode_mvt, load_country_labels,
};
use mappa_map_render::{MapRenderer, MapStyle, PreparedTile, STYLES, prepare};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

mod labels;
mod scale;
#[cfg(feature = "text-rnd")]
mod text;

type DynError = Box<dyn Error + Send + Sync>;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
const CPU_BUDGET: usize = 64 * 1024 * 1024;
const LIVE_PENDING_LIMIT: usize = 12;
const LIVE_UPLOADS_PER_FRAME: usize = 1;

fn public_roads_mode() -> bool {
    std::env::var("MAPPA_DATASET").is_ok_and(|value| value == "public-naju")
}

fn world_mode() -> bool {
    std::env::var("MAPPA_DATASET").map_or(true, |value| value == "world")
}

fn first_party_mode() -> bool {
    std::env::var("MAPPA_DATASET").is_ok_and(|value| value == "first-party")
}

fn canonical_proof_mode() -> bool {
    std::env::var("MAPPA_DATASET").is_ok_and(|value| value == "canonical-proof")
}

fn isolated_mode() -> bool {
    first_party_mode() || canonical_proof_mode()
}

fn canonical_proof_file() -> PathBuf {
    std::env::var_os("MAPPA_CANONICAL_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../artifacts/map-v0.3c/naju-roads.pmtiles")
        })
}

fn first_party_file() -> PathBuf {
    std::env::var_os("MAPPA_FIRST_PARTY_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/map/first_party.pmtiles")
        })
}

fn map_style(index: usize) -> MapStyle {
    let mut style = STYLES[index];
    if isolated_mode() {
        // An unrecorded area has unknown geography, not ocean or land.
        style.ocean = [0.91, 0.93, 0.94, 1.0];
    }
    style
}

#[derive(Clone, Copy)]
enum UserEvent {
    TileReady,
}

fn map_file() -> PathBuf {
    std::env::var_os("MAPPA_MAP_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/map/world_110m.pmtiles")
        })
}

fn detail_map_file() -> PathBuf {
    std::env::var_os("MAPPA_DETAIL_MAP_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/map/east_asia_10m.pmtiles")
        })
}

fn world_detail_file() -> PathBuf {
    std::env::var_os("MAPPA_WORLD_DETAIL_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/map/world_50m.pmtiles")
        })
}

fn street_map_file() -> PathBuf {
    std::env::var_os("MAPPA_STREET_MAP_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let name = if public_roads_mode() {
                "naju_public_roads.pmtiles"
            } else {
                "seoul_osm_streets.pmtiles"
            };
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../assets/map")
                .join(name)
        })
}

fn mid_map_file() -> PathBuf {
    std::env::var_os("MAPPA_MID_MAP_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let name = if public_roads_mode() {
                "naju_public_roads.pmtiles"
            } else {
                "korea_osm_roads.pmtiles"
            };
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../assets/map")
                .join(name)
        })
}

fn country_label_file() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/map/source/ne_110m_admin_0_countries.geojson")
}

fn viewport_inside(camera: &MapCamera, bounds: [f64; 4]) -> bool {
    let Ok((left, top)) = camera.screen_to_coordinate(0.0, 0.0) else {
        return false;
    };
    let Ok((right, bottom)) =
        camera.screen_to_coordinate(camera.width_px as f64, camera.height_px as f64)
    else {
        return false;
    };
    left < right
        && left >= bounds[0]
        && right <= bounds[2]
        && bottom >= bounds[1]
        && top <= bounds[3]
}

fn tile_inside(key: TileKey, bounds: [f64; 4]) -> bool {
    let n = (1u32 << key.z) as f64;
    let west = key.x as f64 / n * 360.0 - 180.0;
    let east = (key.x + 1) as f64 / n * 360.0 - 180.0;
    let north = unproject(WorldPoint {
        x: 0.0,
        y: key.y as f64 / n,
    })
    .expect("valid tile coordinate")
    .1;
    let south = unproject(WorldPoint {
        x: 0.0,
        y: (key.y + 1) as f64 / n,
    })
    .expect("valid tile coordinate")
    .1;
    west >= bounds[0] && east <= bounds[2] && south >= bounds[1] && north <= bounds[3]
}

fn source_for_key(key: TileKey, sources: [&Arc<LocalPmTiles>; 4]) -> &Arc<LocalPmTiles> {
    if key.z >= sources[3].min_zoom && tile_inside(key, sources[3].bounds) {
        sources[3]
    } else if key.z >= sources[2].min_zoom && tile_inside(key, sources[2].bounds) {
        sources[2]
    } else if key.z >= sources[1].min_zoom && tile_inside(key, sources[1].bounds) {
        sources[1]
    } else {
        sources[0]
    }
}

fn tile_on_screen(camera: &MapCamera, tile: &VisibleTile) -> bool {
    let n = (1u32 << tile.key.z) as f64;
    let (x, y) = camera.unwrapped_to_screen(WorldPoint {
        x: tile.world_x as f64 / n,
        y: tile.key.y as f64 / n,
    });
    let size = camera.world_size_px() / n;
    x < camera.width_px as f64 && x + size > 0.0 && y < camera.height_px as f64 && y + size > 0.0
}

struct CacheEntry {
    decoded: Arc<DecodedTile>,
    touched: u64,
    bytes: usize,
}

struct ScreenLabel {
    name: String,
    x: f32,
    y: f32,
    rank: u8,
    kind: LabelKind,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum LabelKind {
    Country,
    City,
    Station,
    Civic,
    Attribution,
    LegendStation,
    LegendCivic,
    Scale,
}

impl ScreenLabel {
    fn text_origin(&self, scale: f32) -> (f32, f32) {
        let width = self.text_width(scale);
        match self.kind {
            LabelKind::Country => (self.x - width / 2.0, self.y - 15.0 * scale),
            LabelKind::Attribution
            | LabelKind::LegendStation
            | LabelKind::LegendCivic
            | LabelKind::Scale => (self.x, self.y),
            _ => (self.x + 12.0 * scale, self.y - 12.0 * scale),
        }
    }

    fn text_width(&self, scale: f32) -> f32 {
        self.name
            .chars()
            .map(|c| if c.is_ascii() { 9.0 } else { 17.0 })
            .sum::<f32>()
            * scale
            + 8.0 * scale
    }
}
#[derive(Default)]
struct LoadStats {
    requests: u64,
    raw_hits: u64,
    raw_hit_bytes: u64,
    decode_hits: u64,
    missing: u64,
    failures: u64,
    evictions: u64,
    lookup_ms: f64,
    decode_ms: f64,
    prepare_ms: f64,
    upload_ms: f64,
}
struct TileManager {
    source: Arc<LocalPmTiles>,
    detail: Arc<LocalPmTiles>,
    mid: Arc<LocalPmTiles>,
    street: Arc<LocalPmTiles>,
    countries: Vec<CountryLabel>,
    cache: HashMap<TileKey, CacheEntry>,
    missing: HashSet<TileKey>,
    tick: u64,
    bytes: usize,
    stats: LoadStats,
}

impl TileManager {
    async fn open() -> Result<Self, DynError> {
        if !world_mode()
            && !first_party_mode()
            && !canonical_proof_mode()
            && !public_roads_mode()
            && std::env::var("MAPPA_DATASET").as_deref() != Ok("legacy-osm")
        {
            return Err(
                "MAPPA_DATASET must be world, first-party, canonical-proof, public-naju, or legacy-osm".into(),
            );
        }
        if isolated_mode() {
            let file = if canonical_proof_mode() {
                canonical_proof_file()
            } else {
                first_party_file()
            };
            let survey = Arc::new(LocalPmTiles::open(file).await?);
            if canonical_proof_mode() && survey.attribution.is_none() {
                return Err("canonical proof archive has no source attribution".into());
            }
            return Ok(Self {
                source: survey.clone(),
                detail: survey.clone(),
                mid: survey.clone(),
                street: survey,
                countries: Vec::new(),
                cache: HashMap::new(),
                missing: HashSet::new(),
                tick: 0,
                bytes: 0,
                stats: LoadStats::default(),
            });
        }
        if world_mode() {
            let source = Arc::new(LocalPmTiles::open(map_file()).await?);
            let detail = Arc::new(LocalPmTiles::open(world_detail_file()).await?);
            let asia = Arc::new(LocalPmTiles::open(detail_map_file()).await?);
            return Ok(Self {
                source,
                detail: detail.clone(),
                mid: detail.clone(),
                street: asia,
                countries: load_country_labels(&country_label_file())?,
                cache: HashMap::new(),
                missing: HashSet::new(),
                tick: 0,
                bytes: 0,
                stats: LoadStats::default(),
            });
        }
        Ok(Self {
            source: Arc::new(LocalPmTiles::open(map_file()).await?),
            detail: Arc::new(LocalPmTiles::open(detail_map_file()).await?),
            mid: Arc::new(LocalPmTiles::open(mid_map_file()).await?),
            street: Arc::new(LocalPmTiles::open(street_map_file()).await?),
            countries: load_country_labels(&country_label_file())?,
            cache: HashMap::new(),
            missing: HashSet::new(),
            tick: 0,
            bytes: 0,
            stats: LoadStats::default(),
        })
    }

    fn max_zoom_for(&self, camera: &MapCamera) -> u8 {
        if camera.zoom >= self.street.min_zoom as f64 && viewport_inside(camera, self.street.bounds)
        {
            self.street.max_zoom
        } else if camera.zoom >= self.mid.min_zoom as f64
            && viewport_inside(camera, self.mid.bounds)
        {
            self.mid.max_zoom
        } else if camera.zoom >= self.detail.min_zoom as f64
            && viewport_inside(camera, self.detail.bounds)
        {
            self.detail.max_zoom
        } else {
            self.source.max_zoom
        }
    }

    fn visible_labels(&self, camera: &MapCamera, visible: &[VisibleTile]) -> Vec<ScreenLabel> {
        let mut labels = Vec::new();
        if !isolated_mode()
            && (camera.zoom < 6.0
                || (world_mode() && visible.first().is_some_and(|tile| tile.key.z <= 4)))
        {
            for country in &self.countries {
                if let Ok(world) = project(country.longitude, country.latitude) {
                    let (x, y) = camera.world_to_screen(world);
                    if x >= 0.0
                        && y >= 0.0
                        && x < camera.width_px as f64
                        && y < camera.height_px as f64
                    {
                        labels.push(ScreenLabel {
                            name: country.name.clone(),
                            x: x as f32,
                            y: y as f32,
                            rank: country.rank,
                            kind: LabelKind::Country,
                        });
                    }
                }
            }
        } else {
            for placement in visible {
                let Some(tile) = self.cache.get(&placement.key) else {
                    continue;
                };
                let n = (1u32 << placement.key.z) as f64;
                for place in &tile.decoded.place {
                    let world = WorldPoint {
                        x: (placement.world_x as f64 + place.point.x() as f64 / 4096.0) / n,
                        y: (placement.key.y as f64 + place.point.y() as f64 / 4096.0) / n,
                    };
                    let (x, y) = camera.unwrapped_to_screen(world);
                    if x >= 0.0
                        && y >= 0.0
                        && x < camera.width_px as f64
                        && y < camera.height_px as f64
                    {
                        labels.push(ScreenLabel {
                            name: place.name.clone(),
                            x: x as f32,
                            y: y as f32,
                            rank: place.rank,
                            kind: match place.kind {
                                PlaceKind::City => LabelKind::City,
                                PlaceKind::Station => LabelKind::Station,
                                PlaceKind::Civic => LabelKind::Civic,
                                PlaceKind::District => LabelKind::City,
                            },
                        });
                    }
                }
            }
        }
        labels.sort_by(|a, b| a.rank.cmp(&b.rank).then_with(|| a.name.cmp(&b.name)));
        let mut accepted = Vec::new();
        let mut boxes: Vec<[f32; 4]> = Vec::new();
        let scale = camera.scale_factor as f32;
        for label in labels {
            let (left, top) = label.text_origin(scale);
            let right = left + label.text_width(scale);
            let bottom = top + 26.0 * scale;
            if left < 0.0
                || right > camera.width_px as f32
                || top < 0.0
                || bottom > camera.height_px as f32
                || boxes
                    .iter()
                    .any(|b| left < b[2] && right > b[0] && top < b[3] && bottom > b[1])
            {
                continue;
            }
            boxes.push([left, top, right, bottom]);
            accepted.push(label);
            let label_limit = if world_mode() && camera.zoom < 2.5 {
                20
            } else {
                36
            };
            if accepted.len() == label_limit {
                break;
            }
        }
        if visible
            .first()
            .is_some_and(|tile| tile.key.z >= self.mid.min_zoom)
        {
            if !world_mode()
                && !public_roads_mode()
                && !isolated_mode()
                && visible.first().is_some_and(|tile| tile.key.z >= 11)
            {
                accepted.push(ScreenLabel {
                    name: "● 역".to_owned(),
                    x: 12.0 * scale,
                    y: camera.height_px as f32 - 58.0 * scale,
                    rank: 0,
                    kind: LabelKind::LegendStation,
                });
            }
            if !world_mode()
                && !public_roads_mode()
                && !isolated_mode()
                && visible.first().is_some_and(|tile| tile.key.z >= 12)
            {
                accepted.push(ScreenLabel {
                    name: "● 공공기관".to_owned(),
                    x: 12.0 * scale,
                    y: camera.height_px as f32 - 39.0 * scale,
                    rank: 0,
                    kind: LabelKind::LegendCivic,
                });
            }
            accepted.push(ScreenLabel {
                name: if first_party_mode() {
                    "Mappa 직접 기록 · 미기록 지역은 빈 화면".to_owned()
                } else if canonical_proof_mode() {
                    self.source.attribution.clone().unwrap_or_default()
                } else if world_mode() {
                    "지형: Natural Earth · Mappa 렌더링".to_owned()
                } else if public_roads_mode() {
                    "도로: 나주시 · 지형: Natural Earth".to_owned()
                } else {
                    "© OpenStreetMap contributors".to_owned()
                },
                x: 12.0 * scale,
                y: camera.height_px as f32 - 22.0 * scale,
                rank: 0,
                kind: LabelKind::Attribution,
            });
        }
        accepted
    }

    async fn ensure(
        &mut self,
        renderer: &mut MapRenderer,
        placements: &[VisibleTile],
        max_new: usize,
    ) {
        let mut new_requests = 0;
        for placement in placements {
            let key = placement.key;
            if renderer.has_tile(key) || self.missing.contains(&key) {
                continue;
            }
            if new_requests >= max_new {
                break;
            }
            self.stats.requests += 1;
            self.tick += 1;
            if let Some(entry) = self.cache.get_mut(&key) {
                entry.touched = self.tick;
                self.stats.raw_hits += 1;
                self.stats.raw_hit_bytes += entry.decoded.raw_bytes as u64;
                self.stats.decode_hits += 1;
                let start = Instant::now();
                match prepare(&entry.decoded) {
                    Ok(prepared) => {
                        self.stats.prepare_ms += start.elapsed().as_secs_f64() * 1000.0;
                        let start = Instant::now();
                        renderer.upload_tile(key, prepared);
                        self.stats.upload_ms += start.elapsed().as_secs_f64() * 1000.0;
                    }
                    Err(e) => {
                        self.stats.failures += 1;
                        eprintln!("tile {key:?} preparation: {e}");
                    }
                }
                continue;
            }
            new_requests += 1;
            let start = Instant::now();
            let source = source_for_key(key, [&self.source, &self.detail, &self.mid, &self.street]);
            match source.tile_bytes(key).await {
                Ok(Some(raw)) => {
                    self.stats.lookup_ms += start.elapsed().as_secs_f64() * 1000.0;
                    let start = Instant::now();
                    match decode_mvt(raw) {
                        Ok(decoded) => {
                            self.stats.decode_ms += start.elapsed().as_secs_f64() * 1000.0;
                            let decoded = Arc::new(decoded);
                            let start = Instant::now();
                            match prepare(&decoded) {
                                Ok(prepared) => {
                                    self.stats.prepare_ms += start.elapsed().as_secs_f64() * 1000.0;
                                    let start = Instant::now();
                                    renderer.upload_tile(key, prepared);
                                    self.stats.upload_ms += start.elapsed().as_secs_f64() * 1000.0;
                                    let bytes = decoded.estimated_bytes();
                                    self.bytes += bytes;
                                    self.cache.insert(
                                        key,
                                        CacheEntry {
                                            decoded,
                                            touched: self.tick,
                                            bytes,
                                        },
                                    );
                                    self.evict();
                                }
                                Err(e) => {
                                    self.stats.failures += 1;
                                    eprintln!("tile {key:?} preparation: {e}");
                                }
                            }
                        }
                        Err(e) => {
                            self.stats.failures += 1;
                            eprintln!("tile {key:?} decode: {e}");
                        }
                    }
                }
                Ok(None) => {
                    self.stats.missing += 1;
                    self.missing.insert(key);
                }
                Err(e) => {
                    self.stats.failures += 1;
                    eprintln!("tile {key:?} read: {e}");
                }
            }
        }
    }
    fn evict(&mut self) {
        while self.bytes > CPU_BUDGET {
            let Some(victim) = self
                .cache
                .iter()
                .min_by_key(|(_, v)| v.touched)
                .map(|(k, _)| *k)
            else {
                break;
            };
            if let Some(entry) = self.cache.remove(&victim) {
                self.bytes -= entry.bytes;
                self.stats.evictions += 1;
            }
        }
    }
}

struct TileRequest {
    key: TileKey,
    decoded: Option<Arc<DecodedTile>>,
}

struct TileResult {
    key: TileKey,
    outcome: TileOutcome,
    lookup_ms: f64,
    decode_ms: f64,
    prepare_ms: f64,
}

enum TileOutcome {
    Ready {
        decoded: Arc<DecodedTile>,
        prepared: Box<PreparedTile>,
    },
    Missing,
    Skipped,
    Failed(String),
}

struct LiveTileLoader {
    requests: mpsc::SyncSender<TileRequest>,
    results: mpsc::Receiver<TileResult>,
    desired: Arc<Mutex<HashSet<TileKey>>>,
    pending: HashSet<TileKey>,
}

impl LiveTileLoader {
    fn new(
        manager: &TileManager,
        wake: Option<EventLoopProxy<UserEvent>>,
    ) -> Result<Self, DynError> {
        let (requests, request_rx) = mpsc::sync_channel::<TileRequest>(LIVE_PENDING_LIMIT);
        let (result_tx, results) = mpsc::sync_channel::<TileResult>(LIVE_UPLOADS_PER_FRAME * 2);
        let desired = Arc::new(Mutex::new(HashSet::new()));
        let source = Arc::clone(&manager.source);
        let detail = Arc::clone(&manager.detail);
        let mid = Arc::clone(&manager.mid);
        let street = Arc::clone(&manager.street);
        let wanted = Arc::clone(&desired);
        let worker_runtime = runtime()?;
        std::thread::Builder::new()
            .name("mappa-tile".to_owned())
            .spawn(move || {
                while let Ok(request) = request_rx.recv() {
                    let key = request.key;
                    let still_wanted = wanted.lock().expect("wanted tiles lock").contains(&key);
                    let result = if still_wanted {
                        load_tile(request, [&source, &detail, &mid, &street], &worker_runtime)
                    } else {
                        TileResult {
                            key,
                            outcome: TileOutcome::Skipped,
                            lookup_ms: 0.0,
                            decode_ms: 0.0,
                            prepare_ms: 0.0,
                        }
                    };
                    if result_tx.send(result).is_err() {
                        break;
                    }
                    if let Some(proxy) = &wake {
                        let _ = proxy.send_event(UserEvent::TileReady);
                    }
                }
            })?;
        Ok(Self {
            requests,
            results,
            desired,
            pending: HashSet::new(),
        })
    }

    fn tick(
        &mut self,
        manager: &mut TileManager,
        renderer: &mut MapRenderer,
        camera: &MapCamera,
        placements: &[VisibleTile],
    ) {
        let desired: HashSet<_> = placements.iter().map(|tile| tile.key).collect();
        *self.desired.lock().expect("wanted tiles lock") = desired;
        let mut uploads = 0;
        while let Ok(result) = self.results.try_recv() {
            self.pending.remove(&result.key);
            manager.stats.lookup_ms += result.lookup_ms;
            manager.stats.decode_ms += result.decode_ms;
            manager.stats.prepare_ms += result.prepare_ms;
            match result.outcome {
                TileOutcome::Ready { decoded, prepared } => {
                    let wanted = self
                        .desired
                        .lock()
                        .expect("wanted tiles lock")
                        .contains(&result.key);
                    if wanted && !manager.cache.contains_key(&result.key) {
                        let bytes = decoded.estimated_bytes();
                        manager.tick += 1;
                        manager.bytes += bytes;
                        manager.cache.insert(
                            result.key,
                            CacheEntry {
                                decoded,
                                touched: manager.tick,
                                bytes,
                            },
                        );
                        manager.evict();
                    }
                    if wanted {
                        let start = Instant::now();
                        renderer.upload_tile(result.key, *prepared);
                        manager.stats.upload_ms += start.elapsed().as_secs_f64() * 1000.0;
                        uploads += 1;
                    }
                }
                TileOutcome::Missing => {
                    manager.stats.missing += 1;
                    manager.missing.insert(result.key);
                }
                TileOutcome::Failed(error) => {
                    manager.stats.failures += 1;
                    eprintln!("tile {:?}: {error}", result.key);
                }
                TileOutcome::Skipped => {}
            }
            if uploads == LIVE_UPLOADS_PER_FRAME {
                break;
            }
        }
        let mut prioritized = Vec::new();
        for placement in placements {
            let key = placement.key;
            if renderer.has_tile(key)
                || manager.missing.contains(&key)
                || self.pending.contains(&key)
            {
                continue;
            }
            let n = (1u32 << key.z) as f64;
            let (x, y) = camera.unwrapped_to_screen(WorldPoint {
                x: (placement.world_x as f64 + 0.5) / n,
                y: (key.y as f64 + 0.5) / n,
            });
            let distance = (x - camera.width_px as f64 / 2.0).powi(2)
                + (y - camera.height_px as f64 / 2.0).powi(2);
            prioritized.push((distance, key));
        }
        prioritized.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, key) in prioritized {
            let decoded = manager.cache.get_mut(&key).map(|entry| {
                manager.tick += 1;
                entry.touched = manager.tick;
                Arc::clone(&entry.decoded)
            });
            let was_cached = decoded.is_some();
            match self.requests.try_send(TileRequest { key, decoded }) {
                Ok(()) => {
                    self.pending.insert(key);
                    manager.stats.requests += 1;
                    if was_cached {
                        manager.stats.raw_hits += 1;
                        manager.stats.decode_hits += 1;
                        manager.stats.raw_hit_bytes += manager.cache[&key].decoded.raw_bytes as u64;
                    }
                }
                Err(mpsc::TrySendError::Full(_)) => break,
                Err(mpsc::TrySendError::Disconnected(_)) => {
                    manager.stats.failures += 1;
                    eprintln!("tile worker disconnected");
                    break;
                }
            }
        }
    }

    fn settled(
        &self,
        manager: &TileManager,
        renderer: &MapRenderer,
        placements: &[VisibleTile],
    ) -> bool {
        placements
            .iter()
            .all(|tile| renderer.contains_tile(tile.key) || manager.missing.contains(&tile.key))
    }
}

fn load_tile(
    request: TileRequest,
    sources: [&Arc<LocalPmTiles>; 4],
    runtime: &tokio::runtime::Runtime,
) -> TileResult {
    let TileRequest { key, decoded } = request;
    let mut result = TileResult {
        key,
        outcome: TileOutcome::Skipped,
        lookup_ms: 0.0,
        decode_ms: 0.0,
        prepare_ms: 0.0,
    };
    let decoded = if let Some(decoded) = decoded {
        decoded
    } else {
        let source = source_for_key(key, sources);
        let start = Instant::now();
        let raw = runtime.block_on(source.tile_bytes(key));
        result.lookup_ms = start.elapsed().as_secs_f64() * 1000.0;
        let raw = match raw {
            Ok(Some(raw)) => raw,
            Ok(None) => {
                result.outcome = TileOutcome::Missing;
                return result;
            }
            Err(error) => {
                result.outcome = TileOutcome::Failed(error.to_string());
                return result;
            }
        };
        let start = Instant::now();
        let decoded = decode_mvt(raw);
        result.decode_ms = start.elapsed().as_secs_f64() * 1000.0;
        match decoded {
            Ok(decoded) => Arc::new(decoded),
            Err(error) => {
                result.outcome = TileOutcome::Failed(error.to_string());
                return result;
            }
        }
    };
    let start = Instant::now();
    let prepared = prepare(&decoded);
    result.prepare_ms = start.elapsed().as_secs_f64() * 1000.0;
    result.outcome = match prepared {
        Ok(prepared) => TileOutcome::Ready {
            decoded,
            prepared: Box::new(prepared),
        },
        Err(error) => TileOutcome::Failed(error.to_string()),
    };
    result
}

fn runtime() -> Result<tokio::runtime::Runtime, DynError> {
    Ok(tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?)
}

fn screenshot(
    renderer: &mut MapRenderer,
    camera: &MapCamera,
    visible: &[VisibleTile],
    style_idx: usize,
    path: &Path,
) -> Result<(), DynError> {
    screenshot_inner(renderer, camera, visible, style_idx, path, |_, _| Ok(()))
}

fn screenshot_inner(
    renderer: &mut MapRenderer,
    camera: &MapCamera,
    visible: &[VisibleTile],
    style_idx: usize,
    path: &Path,
    overlay: impl FnOnce(&wgpu::TextureView, &mut MapRenderer) -> Result<(), DynError>,
) -> Result<(), DynError> {
    let width = camera.width_px;
    let height = camera.height_px;
    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("preview"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    renderer.render(&view, camera, visible, map_style(style_idx));
    overlay(&view, renderer)?;
    let row_bytes = width * 4;
    let padded = row_bytes.div_ceil(256) * 256;
    let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = renderer
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("readback"),
        });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    renderer.queue.submit(Some(encoder.finish()));
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = sender.send(r);
    });
    renderer.device.poll(wgpu::PollType::wait_indefinitely())?;
    receiver.recv()?.map_err(|e| format!("GPU readback: {e}"))?;
    let mapped = buffer.slice(..).get_mapped_range()?;
    let mut rgba = Vec::with_capacity((row_bytes * height) as usize);
    for row in mapped.chunks(padded as usize).take(height as usize) {
        for pixel in row[..row_bytes as usize].as_chunks::<4>().0 {
            rgba.extend([pixel[2], pixel[1], pixel[0], pixel[3]]);
        }
    }
    drop(mapped);
    buffer.unmap();
    let file = File::create(path)?;
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&rgba)?;
    Ok(())
}

fn contact_sheet(region: bool) -> Result<(), DynError> {
    const WIDTH: usize = 1200;
    const HEIGHT: usize = 720;
    let suffix = if region { "-region" } else { "" };
    let mut pixels = vec![0u8; WIDTH * 2 * HEIGHT * 2 * 4];
    for (index, style) in STYLES.iter().enumerate() {
        let path = format!(
            "artifacts/map-v0.3a/{}{suffix}.png",
            style.name.to_lowercase()
        );
        let mut reader = png::Decoder::new(BufReader::new(File::open(path)?)).read_info()?;
        let mut source = vec![0; reader.output_buffer_size().ok_or("PNG output too large")?];
        let info = reader.next_frame(&mut source)?;
        if info.width != WIDTH as u32
            || info.height != HEIGHT as u32
            || info.color_type != png::ColorType::Rgba
        {
            return Err("unexpected style preview format".into());
        }
        let source = &source[..info.buffer_size()];
        let x = (index % 2) * WIDTH;
        let y = (index / 2) * HEIGHT;
        for row in 0..HEIGHT {
            let source_start = row * WIDTH * 4;
            let target_start = ((y + row) * WIDTH * 2 + x) * 4;
            pixels[target_start..target_start + WIDTH * 4]
                .copy_from_slice(&source[source_start..source_start + WIDTH * 4]);
        }
    }
    let file = File::create(format!("artifacts/map-v0.3a/contact-sheet-v3{suffix}.png"))?;
    let mut encoder = png::Encoder::new(file, (WIDTH * 2) as u32, (HEIGHT * 2) as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&pixels)?;
    Ok(())
}

fn offscreen(benchmark: bool) -> Result<(), DynError> {
    let runtime = runtime()?;
    let mut renderer = runtime.block_on(MapRenderer::new(FORMAT))?;
    let info = renderer.adapter.get_info();
    println!("GPU: {:?} / {}", info.backend, info.name);
    let start = Instant::now();
    let mut manager = runtime.block_on(TileManager::open())?;
    let open_ms = start.elapsed().as_secs_f64() * 1000.0;
    let camera = MapCamera::new(0.0, 15.0, 1.3, 1200, 720, 1.0)?;
    let visible = camera.visible_tiles(manager.max_zoom_for(&camera), 1);
    let start = Instant::now();
    runtime.block_on(manager.ensure(&mut renderer, &visible, usize::MAX));
    let cold_load_ms = start.elapsed().as_secs_f64() * 1000.0;
    let probe = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("benchmark"),
        size: wgpu::Extent3d {
            width: camera.width_px,
            height: camera.height_px,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let probe_view = probe.create_view(&wgpu::TextureViewDescriptor::default());
    let start = Instant::now();
    renderer.render(&probe_view, &camera, &visible, map_style(0));
    renderer.device.poll(wgpu::PollType::wait_indefinitely())?;
    let cold_first_frame_ms = open_ms + cold_load_ms + start.elapsed().as_secs_f64() * 1000.0;
    std::fs::create_dir_all("artifacts/map-v0.3a")?;
    for (i, style) in STYLES.iter().enumerate() {
        let path = format!("artifacts/map-v0.3a/{}.png", style.name.to_lowercase());
        screenshot(&mut renderer, &camera, &visible, i, Path::new(&path))?;
        println!("{}: {}", style.name, path);
    }
    contact_sheet(false)?;
    println!("world comparison: artifacts/map-v0.3a/contact-sheet-v3.png");
    if !benchmark {
        let region = MapCamera::new(120.0, 35.0, 3.2, 1200, 720, 1.0)?;
        let visible_region = region.visible_tiles(manager.max_zoom_for(&region), 1);
        runtime.block_on(manager.ensure(&mut renderer, &visible_region, usize::MAX));
        for (i, style) in STYLES.iter().enumerate() {
            let path = format!(
                "artifacts/map-v0.3a/{}-region.png",
                style.name.to_lowercase()
            );
            screenshot(&mut renderer, &region, &visible_region, i, Path::new(&path))?;
            println!("{} region: {}", style.name, path);
        }
        contact_sheet(true)?;
        println!("regional comparison: artifacts/map-v0.3a/contact-sheet-v3-region.png");
    }
    if benchmark {
        let mut samples = Vec::new();
        for _ in 0..120 {
            let start = Instant::now();
            renderer.render(&probe_view, &camera, &visible, map_style(0));
            renderer.device.poll(wgpu::PollType::wait_indefinitely())?;
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        samples.sort_by(f64::total_cmp);
        let q = |p: f64| samples[((samples.len() - 1) as f64 * p).round() as usize];
        println!(
            "open_ms={open_ms:.3} cold_load_ms={cold_load_ms:.3} cold_first_frame_ms={cold_first_frame_ms:.3} lookup_ms={:.3} decode_ms={:.3} prepare_ms={:.3} upload_ms={:.3}",
            manager.stats.lookup_ms,
            manager.stats.decode_ms,
            manager.stats.prepare_ms,
            manager.stats.upload_ms
        );
        println!(
            "warm_offscreen_ms median={:.3} p95={:.3} p99={:.3}",
            q(0.5),
            q(0.95),
            q(0.99)
        );
        let mut pan = MapCamera::new(-100.0, 38.0, 3.3, 1200, 720, 1.0)?;
        let mut pan_samples = Vec::new();
        for _ in 0..40 {
            let start = Instant::now();
            pan.pan_physical(-95.0, 0.0)?;
            let tiles = pan.visible_tiles(manager.max_zoom_for(&pan), 1);
            runtime.block_on(manager.ensure(&mut renderer, &tiles, 4));
            renderer.render(&probe_view, &pan, &tiles, map_style(0));
            renderer.device.poll(wgpu::PollType::wait_indefinitely())?;
            pan_samples.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        pan_samples.sort_by(f64::total_cmp);
        let p = |q: f64| pan_samples[((pan_samples.len() - 1) as f64 * q).round() as usize];
        println!(
            "pan_offscreen_ms median={:.3} p95={:.3} p99={:.3}",
            p(0.5),
            p(0.95),
            p(0.99)
        );
        let mut zoom = MapCamera::new(110.0, 35.0, 1.3, 1200, 720, 1.0)?;
        let mut zoom_samples = Vec::new();
        for _ in 0..24 {
            let start = Instant::now();
            zoom.zoom_at(0.14, 600.0, 360.0)?;
            let tiles = zoom.visible_tiles(manager.max_zoom_for(&zoom), 1);
            runtime.block_on(manager.ensure(&mut renderer, &tiles, 4));
            renderer.render(&probe_view, &zoom, &tiles, map_style(0));
            renderer.device.poll(wgpu::PollType::wait_indefinitely())?;
            zoom_samples.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        zoom_samples.sort_by(f64::total_cmp);
        let z = |q: f64| zoom_samples[((zoom_samples.len() - 1) as f64 * q).round() as usize];
        println!(
            "zoom_offscreen_ms median={:.3} p95={:.3} p99={:.3}",
            z(0.5),
            z(0.95),
            z(0.99)
        );
        println!(
            "tile requests={} raw_hits={} raw_hit_bytes={} decode_hits={} missing={} failures={} cpu_evictions={} cpu_bytes={} gpu_uploads={} gpu_hits={} gpu_evictions={} gpu_bytes={}",
            manager.stats.requests,
            manager.stats.raw_hits,
            manager.stats.raw_hit_bytes,
            manager.stats.decode_hits,
            manager.stats.missing,
            manager.stats.failures,
            manager.stats.evictions,
            manager.bytes,
            renderer.stats.uploads,
            renderer.stats.hits,
            renderer.stats.evictions,
            renderer.stats.bytes
        );
    }
    Ok(())
}

fn views() -> Result<(), DynError> {
    let runtime = runtime()?;
    let mut renderer = runtime.block_on(MapRenderer::new(FORMAT))?;
    let mut manager = runtime.block_on(TileManager::open())?;
    std::fs::create_dir_all("artifacts/map-v0.3a/views")?;
    for (name, lon, lat, zoom) in [
        ("world", 0.0, 15.0, 1.3),
        ("east-asia", 120.0, 35.0, 3.2),
        ("korea", 127.5, 37.5, 5.3),
        ("europe", 15.0, 50.0, 3.2),
        ("north-america", -105.0, 40.0, 3.2),
        ("antimeridian", 179.0, 10.0, 2.5),
    ] {
        let camera = MapCamera::new(lon, lat, zoom, 1200, 720, 1.0)?;
        let visible = camera.visible_tiles(manager.max_zoom_for(&camera), 1);
        runtime.block_on(manager.ensure(&mut renderer, &visible, usize::MAX));
        let path = format!("artifacts/map-v0.3a/views/{name}.png");
        screenshot(&mut renderer, &camera, &visible, 0, Path::new(&path))?;
        println!("{name}: {path} ({} placements)", visible.len());
    }
    let camera = MapCamera::new(0.0, 15.0, 1.3, 2400, 1440, 2.0)?;
    let visible = camera.visible_tiles(manager.max_zoom_for(&camera), 1);
    runtime.block_on(manager.ensure(&mut renderer, &visible, usize::MAX));
    screenshot(
        &mut renderer,
        &camera,
        &visible,
        0,
        Path::new("artifacts/map-v0.3a/views/retina-2x.png"),
    )?;
    println!("retina-2x: artifacts/map-v0.3a/views/retina-2x.png");
    Ok(())
}

fn zoom_comparison() -> Result<(), DynError> {
    let runtime = runtime()?;
    let mut renderer = runtime.block_on(MapRenderer::new(FORMAT))?;
    let mut labels = labels::LabelRenderer::new(&renderer);
    let mut manager = runtime.block_on(TileManager::open())?;
    std::fs::create_dir_all("artifacts/map-v0.3a/zoom")?;
    for (name, lon, lat, zoom) in [
        ("overview", 127.5, 37.5, 4.4),
        ("coast", 127.5, 37.5, 5.4),
        ("roads", 127.5, 37.5, 6.4),
        ("close", 127.5, 37.5, 7.4),
        ("korea-network", 126.978_291, 37.566_679, 8.4),
        ("seoul-approach", 126.978_291, 37.566_679, 9.4),
        ("city", 126.978_291, 37.566_679, 10.4),
        ("stations", 126.978_291, 37.566_679, 11.4),
        ("civic", 126.978_291, 37.566_679, 12.4),
        ("civic-close", 126.978_291, 37.566_679, 13.4),
        ("neighborhood", 126.978_291, 37.566_679, 14.4),
        ("street", 126.978_291, 37.566_679, 15.4),
        ("street-close", 126.978_291, 37.566_679, 16.0),
    ] {
        let camera = MapCamera::new(lon, lat, zoom, 1200, 720, 1.0)?;
        let visible = camera.visible_tiles(manager.max_zoom_for(&camera), 1);
        let tile_zoom = visible.first().map_or(0, |placement| placement.key.z);
        let source = if tile_zoom >= manager.street.min_zoom {
            "seoul-osm"
        } else if tile_zoom >= manager.mid.min_zoom {
            "korea-osm"
        } else if tile_zoom >= manager.detail.min_zoom {
            "regional-10m"
        } else {
            "world-110m"
        };
        let stages_before = (
            manager.stats.lookup_ms,
            manager.stats.decode_ms,
            manager.stats.prepare_ms,
            manager.stats.upload_ms,
        );
        let start = Instant::now();
        runtime.block_on(manager.ensure(&mut renderer, &visible, usize::MAX));
        let load_ms = start.elapsed().as_secs_f64() * 1000.0;
        let stage_ms = (
            manager.stats.lookup_ms - stages_before.0,
            manager.stats.decode_ms - stages_before.1,
            manager.stats.prepare_ms - stages_before.2,
            manager.stats.upload_ms - stages_before.3,
        );
        let unresolved_visible = visible
            .iter()
            .filter(|tile| {
                !renderer.contains_tile(tile.key) && !manager.missing.contains(&tile.key)
            })
            .count();
        if unresolved_visible != 0 {
            return Err(format!(
                "{name}: {unresolved_visible} visible tiles unresolved or evicted"
            )
            .into());
        }
        let path = format!("artifacts/map-v0.3a/zoom/{name}.png");
        let visible_labels = manager.visible_labels(&camera, &visible);
        screenshot_inner(
            &mut renderer,
            &camera,
            &visible,
            0,
            Path::new(&path),
            |view, renderer| labels.draw(renderer, view, &camera, &visible_labels),
        )?;
        println!(
            "{name}: {path}, camera_zoom={zoom}, source={source}, tile_zoom={tile_zoom}, labels={}, load_ms={load_ms:.3}, lookup_ms={:.3}, decode_ms={:.3}, prepare_ms={:.3}, upload_ms={:.3}, cpu_bytes={}, gpu_bytes={}, failures={}",
            visible_labels.len(),
            stage_ms.0,
            stage_ms.1,
            stage_ms.2,
            stage_ms.3,
            manager.bytes,
            renderer.stats.bytes,
            manager.stats.failures
        );
    }
    Ok(())
}

fn street_benchmark() -> Result<(), DynError> {
    let runtime = runtime()?;
    let mut renderer = runtime.block_on(MapRenderer::new(FORMAT))?;
    let mut labels = labels::LabelRenderer::new(&renderer);
    let mut manager = runtime.block_on(TileManager::open())?;
    let mut camera = MapCamera::new(126.866_778, 37.302_813, 11.4, 1200, 720, 1.0)?;
    let visible = camera.visible_tiles(manager.max_zoom_for(&camera), 1);
    let start = Instant::now();
    runtime.block_on(manager.ensure(&mut renderer, &visible, usize::MAX));
    println!(
        "street_cold_load_ms={:.3}",
        start.elapsed().as_secs_f64() * 1000.0
    );
    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("street benchmark"),
        size: wgpu::Extent3d {
            width: camera.width_px,
            height: camera.height_px,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut warm = Vec::new();
    for _ in 0..120 {
        let start = Instant::now();
        renderer.render(&view, &camera, &visible, map_style(0));
        labels.draw(
            &renderer,
            &view,
            &camera,
            &manager.visible_labels(&camera, &visible),
        )?;
        renderer.device.poll(wgpu::PollType::wait_indefinitely())?;
        warm.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    warm.sort_by(f64::total_cmp);
    println!(
        "street_warm_frame_ms median={:.3} p95={:.3} p99={:.3}",
        warm[60], warm[114], warm[118]
    );
    let mut pan = Vec::new();
    for _ in 0..40 {
        let start = Instant::now();
        camera.pan_physical(-24.0, 0.0)?;
        let visible = camera.visible_tiles(manager.max_zoom_for(&camera), 1);
        runtime.block_on(manager.ensure(&mut renderer, &visible, 4));
        renderer.render(&view, &camera, &visible, map_style(0));
        labels.draw(
            &renderer,
            &view,
            &camera,
            &manager.visible_labels(&camera, &visible),
        )?;
        renderer.device.poll(wgpu::PollType::wait_indefinitely())?;
        pan.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    pan.sort_by(f64::total_cmp);
    println!(
        "street_pan_frame_ms median={:.3} p95={:.3} p99={:.3} failures={}",
        pan[20], pan[37], pan[39], manager.stats.failures
    );
    Ok(())
}

fn async_street_benchmark(lon: f64, lat: f64, zoom: f64) -> Result<(), DynError> {
    let runtime = runtime()?;
    let mut renderer = runtime.block_on(MapRenderer::new(FORMAT))?;
    let mut labels = labels::LabelRenderer::new(&renderer);
    let mut manager = runtime.block_on(TileManager::open())?;
    let mut loader = LiveTileLoader::new(&manager, None)?;
    let mut camera = MapCamera::new(lon, lat, zoom, 1200, 720, 1.0)?;
    let visible = camera.visible_tiles(manager.max_zoom_for(&camera), 1);
    let start = Instant::now();
    while !loader.settled(&manager, &renderer, &visible) {
        loader.tick(&mut manager, &mut renderer, &camera, &visible);
        std::thread::sleep(Duration::from_millis(1));
        if start.elapsed() > Duration::from_secs(10) {
            return Err("async street benchmark timed out".into());
        }
    }
    println!(
        "async_street_cold_load_ms={:.3}",
        start.elapsed().as_secs_f64() * 1000.0
    );
    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("async street benchmark"),
        size: wgpu::Extent3d {
            width: camera.width_px,
            height: camera.height_px,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut pan = Vec::new();
    let mut frames_with_unresolved = 0;
    for _ in 0..40 {
        let start = Instant::now();
        camera.pan_physical(-24.0, 0.0)?;
        let visible = camera.visible_tiles(manager.max_zoom_for(&camera), 1);
        loader.tick(&mut manager, &mut renderer, &camera, &visible);
        if visible.iter().any(|tile| {
            tile_on_screen(&camera, tile)
                && !renderer.contains_tile(tile.key)
                && !manager.missing.contains(&tile.key)
        }) {
            frames_with_unresolved += 1;
        }
        renderer.render(&view, &camera, &visible, map_style(0));
        labels.draw(
            &renderer,
            &view,
            &camera,
            &manager.visible_labels(&camera, &visible),
        )?;
        renderer.device.poll(wgpu::PollType::wait_indefinitely())?;
        pan.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    pan.sort_by(f64::total_cmp);
    println!(
        "async_street_pan_frame_ms median={:.3} p95={:.3} p99={:.3} frames_with_unresolved={} failures={}",
        pan[20], pan[37], pan[39], frames_with_unresolved, manager.stats.failures
    );
    Ok(())
}

fn capture_location(
    lon: f64,
    lat: f64,
    zoom: f64,
    path: &Path,
    async_load: bool,
) -> Result<(), DynError> {
    let runtime = runtime()?;
    let mut renderer = runtime.block_on(MapRenderer::new(FORMAT))?;
    let mut labels = labels::LabelRenderer::new(&renderer);
    let mut manager = runtime.block_on(TileManager::open())?;
    let camera = MapCamera::new(lon, lat, zoom, 1200, 720, 1.0)?;
    let visible = camera.visible_tiles(manager.max_zoom_for(&camera), 1);
    if async_load {
        let mut loader = LiveTileLoader::new(&manager, None)?;
        let start = Instant::now();
        while !loader.settled(&manager, &renderer, &visible) {
            loader.tick(&mut manager, &mut renderer, &camera, &visible);
            if start.elapsed() > Duration::from_secs(10) {
                return Err("async tile load timed out".into());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        println!(
            "async_load_ms={:.3}",
            start.elapsed().as_secs_f64() * 1000.0
        );
    } else {
        runtime.block_on(manager.ensure(&mut renderer, &visible, usize::MAX));
    }
    let unresolved = visible
        .iter()
        .filter(|tile| !renderer.contains_tile(tile.key) && !manager.missing.contains(&tile.key))
        .count();
    if unresolved != 0 {
        return Err(format!("{unresolved} visible tiles unresolved").into());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let visible_labels = manager.visible_labels(&camera, &visible);
    screenshot_inner(
        &mut renderer,
        &camera,
        &visible,
        0,
        path,
        |view, renderer| labels.draw(renderer, view, &camera, &visible_labels),
    )?;
    println!(
        "{}: lon={lon}, lat={lat}, zoom={zoom}, tile_zoom={}, labels={}, failures={}",
        path.display(),
        visible.first().map_or(0, |tile| tile.key.z),
        visible_labels.len(),
        manager.stats.failures,
    );
    Ok(())
}

struct App {
    runtime: tokio::runtime::Runtime,
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    config: Option<wgpu::SurfaceConfiguration>,
    renderer: Option<MapRenderer>,
    labels: Option<labels::LabelRenderer>,
    manager: Option<TileManager>,
    loader: Option<LiveTileLoader>,
    event_proxy: EventLoopProxy<UserEvent>,
    camera: Option<MapCamera>,
    style: usize,
    dragging: bool,
    cursor: (f64, f64),
    last_frame: Instant,
    frames: u64,
    start_street: bool,
    focus: Option<(f64, f64, f64)>,
}

impl App {
    fn new(
        start_street: bool,
        focus: Option<(f64, f64, f64)>,
        event_proxy: EventLoopProxy<UserEvent>,
    ) -> Result<Self, DynError> {
        Ok(Self {
            runtime: runtime()?,
            window: None,
            surface: None,
            config: None,
            renderer: None,
            labels: None,
            manager: None,
            loader: None,
            event_proxy,
            camera: None,
            style: 0,
            dragging: false,
            cursor: (0.0, 0.0),
            last_frame: Instant::now(),
            frames: 0,
            start_street,
            focus,
        })
    }
    fn redraw(&mut self) {
        let (
            Some(surface),
            Some(config),
            Some(camera),
            Some(renderer),
            Some(manager),
            Some(loader),
            Some(labels),
        ) = (
            &self.surface,
            &self.config,
            &self.camera,
            &mut self.renderer,
            &mut self.manager,
            &mut self.loader,
            &mut self.labels,
        )
        else {
            return;
        };
        let visible = camera.visible_tiles(manager.max_zoom_for(camera), 1);
        loader.tick(manager, renderer, camera, &visible);
        let visible_labels = manager.visible_labels(camera, &visible);
        match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                renderer.render(&view, camera, &visible, map_style(self.style));
                if let Err(error) = labels.draw(renderer, &view, camera, &visible_labels) {
                    eprintln!("labels: {error}");
                }
                renderer.queue.present(frame);
                self.frames += 1;
                let elapsed = self.last_frame.elapsed();
                if elapsed >= Duration::from_secs(2) {
                    if elapsed < Duration::from_secs(4) && self.frames >= 30 {
                        let fps = self.frames as f64 / elapsed.as_secs_f64();
                        println!(
                            "style={} zoom={:.2} active_FPS={fps:.1} tiles={} GPU={} bytes",
                            STYLES[self.style].name,
                            camera.zoom,
                            renderer.cached_tiles(),
                            renderer.stats.bytes
                        );
                    }
                    self.frames = 0;
                    self.last_frame = Instant::now();
                }
            }
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                surface.configure(&renderer.device, config)
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {}
            wgpu::CurrentSurfaceTexture::Validation => eprintln!("surface validation error"),
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = match event_loop.create_window(
            Window::default_attributes()
                .with_title(if first_party_mode() {
                    "Mappa 직접 기록 지도 — 미기록 지역은 비어 있음"
                } else if canonical_proof_mode() {
                    "Mappa 자체 지도 실증 — 나주 도로 / 미구축 지역은 비어 있음"
                } else if world_mode() {
                    "Mappa 세계지도 — 1 Arcade · 2 Lagoon · 3 Candy · 4 Sunset"
                } else {
                    "Mappa Offline Map — 1 Arcade · 2 Lagoon · 3 Candy · 4 Sunset"
                })
                .with_inner_size(LogicalSize::new(1200.0, 720.0)),
        ) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                eprintln!("window: {e}");
                event_loop.exit();
                return;
            }
        };
        let size = window.inner_size();
        let renderer = match self.runtime.block_on(MapRenderer::new(FORMAT)) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("renderer: {e}");
                event_loop.exit();
                return;
            }
        };
        println!(
            "GPU: {:?} / {}",
            renderer.adapter.get_info().backend,
            renderer.adapter.get_info().name
        );
        let surface = match renderer.instance.create_surface(window.clone()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("surface: {e}");
                event_loop.exit();
                return;
            }
        };
        let Some(mut config) =
            surface.get_default_config(&renderer.adapter, size.width, size.height)
        else {
            eprintln!("Metal surface unsupported");
            event_loop.exit();
            return;
        };
        if !surface
            .get_capabilities(&renderer.adapter)
            .formats
            .contains(&FORMAT)
        {
            eprintln!("sRGB BGRA surface unsupported");
            event_loop.exit();
            return;
        }
        config.format = FORMAT;
        surface.configure(&renderer.device, &config);
        let manager = match self.runtime.block_on(TileManager::open()) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("fixture: {e}");
                event_loop.exit();
                return;
            }
        };
        let loader = match LiveTileLoader::new(&manager, Some(self.event_proxy.clone())) {
            Ok(loader) => loader,
            Err(error) => {
                eprintln!("tile worker: {error}");
                event_loop.exit();
                return;
            }
        };
        let (lon, lat, zoom) = if let Some(focus) = self.focus {
            focus
        } else if world_mode() {
            (0.0, 15.0, 1.3)
        } else if self.start_street {
            (manager.street.center[0], manager.street.center[1], 11.4)
        } else {
            (0.0, 15.0, 1.3)
        };
        let camera = MapCamera::new(
            lon,
            lat,
            zoom,
            size.width,
            size.height,
            window.scale_factor(),
        )
        .expect("window size valid");
        self.window = Some(window);
        self.surface = Some(surface);
        self.config = Some(config);
        self.labels = Some(labels::LabelRenderer::new(&renderer));
        self.renderer = Some(renderer);
        self.manager = Some(manager);
        self.loader = Some(loader);
        self.camera = Some(camera);
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let mut changed = false;
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::Resized(size) => {
                if size.width > 0
                    && size.height > 0
                    && let (Some(surface), Some(config), Some(renderer), Some(camera), Some(window)) = (
                        &self.surface,
                        &mut self.config,
                        &self.renderer,
                        &mut self.camera,
                        &self.window,
                    )
                {
                    config.width = size.width;
                    config.height = size.height;
                    surface.configure(&renderer.device, config);
                    let _ = camera.resize(size.width, size.height, window.scale_factor());
                    changed = true;
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let (Some(camera), Some(window)) = (&mut self.camera, &self.window) {
                    let size = window.inner_size();
                    let _ = camera.resize(size.width, size.height, scale_factor);
                    changed = true;
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => self.dragging = state == ElementState::Pressed,
            WindowEvent::CursorMoved { position, .. } => {
                if self.dragging
                    && let Some(camera) = &mut self.camera
                {
                    let _ =
                        camera.pan_physical(position.x - self.cursor.0, position.y - self.cursor.1);
                    changed = true;
                }
                self.cursor = (position.x, position.y);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y as f64,
                    MouseScrollDelta::PixelDelta(p) => p.y / 60.0,
                };
                if let Some(camera) = &mut self.camera {
                    let _ = camera.zoom_at(y * 0.15, self.cursor.0, self.cursor.1);
                    changed = true;
                }
            }
            WindowEvent::PinchGesture { delta, .. } if delta.is_finite() && delta > -1.0 => {
                if let Some(camera) = &mut self.camera {
                    let _ = camera.zoom_at((1.0 + delta).log2(), self.cursor.0, self.cursor.1);
                    changed = true;
                }
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    match code {
                        KeyCode::Digit1 => {
                            self.style = 0;
                            changed = true;
                        }
                        KeyCode::Digit2 => {
                            self.style = 1;
                            changed = true;
                        }
                        KeyCode::Digit3 => {
                            self.style = 2;
                            changed = true;
                        }
                        KeyCode::Digit4 => {
                            self.style = 3;
                            changed = true;
                        }
                        KeyCode::Equal | KeyCode::NumpadAdd => {
                            if let Some(camera) = &mut self.camera {
                                let _ = camera.zoom_at(
                                    1.0,
                                    camera.width_px as f64 / 2.0,
                                    camera.height_px as f64 / 2.0,
                                );
                                changed = true;
                            }
                        }
                        KeyCode::Minus | KeyCode::NumpadSubtract => {
                            if let Some(camera) = &mut self.camera {
                                let _ = camera.zoom_at(
                                    -1.0,
                                    camera.width_px as f64 / 2.0,
                                    camera.height_px as f64 / 2.0,
                                );
                                changed = true;
                            }
                        }
                        KeyCode::Escape => event_loop.exit(),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        if changed && let Some(window) = &self.window {
            window.request_redraw();
        }
    }
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: UserEvent) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
    }
}

fn main() -> Result<(), DynError> {
    match std::env::args().nth(1).as_deref() {
        Some("--variants") => offscreen(false),
        Some("--benchmark") => offscreen(true),
        Some("--views") => views(),
        Some("--zoom-compare") => zoom_comparison(),
        Some("--street-benchmark") => street_benchmark(),
        Some("--async-street-benchmark") => {
            let args: Vec<String> = std::env::args().collect();
            match args.len() {
                2 => async_street_benchmark(126.866_778, 37.302_813, 11.4),
                5 => async_street_benchmark(args[2].parse()?, args[3].parse()?, args[4].parse()?),
                _ => Err("usage: mappa-map-demo --async-street-benchmark [LON LAT ZOOM]".into()),
            }
        }
        Some("--capture" | "--async-capture") => {
            let args: Vec<String> = std::env::args().collect();
            if args.len() != 6 {
                return Err(
                    "usage: mappa-map-demo --[async-]capture LON LAT ZOOM OUTPUT.png".into(),
                );
            }
            capture_location(
                args[2].parse()?,
                args[3].parse()?,
                args[4].parse()?,
                Path::new(&args[5]),
                args[1] == "--async-capture",
            )
        }
        Some("--street-demo") => {
            let args: Vec<String> = std::env::args().collect();
            let focus = match args.len() {
                2 => None,
                5 => Some((args[2].parse()?, args[3].parse()?, args[4].parse()?)),
                _ => return Err("usage: mappa-map-demo --street-demo [LON LAT ZOOM]".into()),
            };
            let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
            let mut app = App::new(true, focus, event_loop.create_proxy())?;
            event_loop.run_app(&mut app)?;
            Ok(())
        }
        #[cfg(feature = "text-rnd")]
        Some("--text") => text::run(),
        _ => {
            let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
            let mut app = App::new(false, None, event_loop.create_proxy())?;
            event_loop.run_app(&mut app)?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_region_requires_the_whole_viewport() {
        let bounds = [90.0, 11.178_401, 157.5, 61.606_396];
        let korea = MapCamera::new(127.5, 37.5, 5.4, 1200, 720, 1.0).unwrap();
        let world = MapCamera::new(127.5, 37.5, 1.3, 1200, 720, 1.0).unwrap();
        let outside = MapCamera::new(2.35, 48.86, 6.4, 1200, 720, 1.0).unwrap();
        assert!(viewport_inside(&korea, bounds));
        assert!(!viewport_inside(&world, bounds));
        assert!(!viewport_inside(&outside, bounds));
    }

    #[tokio::test]
    async fn zoom_source_uses_actual_archive_bounds() {
        let manager = TileManager::open().await.unwrap();
        if world_mode() {
            assert!(Arc::ptr_eq(&manager.detail, &manager.mid));
            let asia = MapCamera::new(127.5, 37.5, 6.4, 1200, 720, 1.0).unwrap();
            let europe = MapCamera::new(2.35, 48.86, 6.4, 1200, 720, 1.0).unwrap();
            let overview = MapCamera::new(2.35, 48.86, 4.4, 1200, 720, 1.0).unwrap();
            assert_eq!(manager.max_zoom_for(&asia), 7);
            assert_eq!(manager.max_zoom_for(&europe), 7);
            assert_eq!(manager.max_zoom_for(&overview), 4);
            for (lon, lat, expected) in
                [(127.5, 37.5, &manager.street), (2.35, 48.86, &manager.mid)]
            {
                let point = project(lon, lat).unwrap();
                let key =
                    TileKey::new(6, (point.x * 64.0) as u32, (point.y * 64.0) as u32).unwrap();
                let selected = source_for_key(
                    key,
                    [
                        &manager.source,
                        &manager.detail,
                        &manager.mid,
                        &manager.street,
                    ],
                );
                assert!(Arc::ptr_eq(selected, expected));
            }
            return;
        }
        if first_party_mode() {
            assert!(Arc::ptr_eq(&manager.source, &manager.street));
            assert!(manager.countries.is_empty());
            let camera = MapCamera::new(0.0, 0.0, 10.4, 1200, 720, 1.0).unwrap();
            assert_eq!(manager.max_zoom_for(&camera), 14);
            return;
        }
        if canonical_proof_mode() {
            assert!(Arc::ptr_eq(&manager.source, &manager.street));
            assert!(manager.countries.is_empty());
            assert!(
                manager
                    .source
                    .attribution
                    .as_deref()
                    .unwrap()
                    .contains("나주시")
            );
            let camera = MapCamera::new(126.715, 35.025, 15.2, 1200, 720, 1.0).unwrap();
            assert_eq!(manager.max_zoom_for(&camera), 15);
            return;
        }
        let seoul = (126.978_291, 37.566_679);
        for (zoom, expected) in [(4.4, 4), (5.4, 7), (8.4, 9), (10.4, 12)] {
            let camera = MapCamera::new(seoul.0, seoul.1, zoom, 1200, 720, 1.0).unwrap();
            assert_eq!(manager.max_zoom_for(&camera), expected);
        }
        let outside_street = MapCamera::new(129.06, 35.16, 10.4, 1200, 720, 1.0).unwrap();
        assert_eq!(manager.max_zoom_for(&outside_street), 9);
    }
}
