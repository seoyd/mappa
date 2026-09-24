//! Tile geometry preparation and a small wgpu/Metal basemap renderer.

use bytemuck::{Pod, Zeroable};
use lyon::{
    math::point,
    path::Path,
    tessellation::{
        BuffersBuilder, FillOptions, FillRule, FillTessellator, FillVertex, VertexBuffers,
    },
};
use mappa_map_core::{MapCamera, TileKey, VisibleTile};
use mappa_map_data::{DecodedTile, PlaceKind};
use std::{collections::HashMap, mem, num::NonZeroU64};
use thiserror::Error;
use wgpu::util::DeviceExt;

const UNIFORM_STRIDE: u64 = 256;
const MAX_DRAWS: usize = 2048;
const GPU_BUDGET: usize = 128 * 1024 * 1024;

fn road_fill(casing: [f32; 4], light: [f32; 4], light_fraction: f32) -> [f32; 4] {
    [
        casing[0] + (light[0] - casing[0]) * light_fraction,
        casing[1] + (light[1] - casing[1]) * light_fraction,
        casing[2] + (light[2] - casing[2]) * light_fraction,
        1.0,
    ]
}

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("no Metal adapter: {0}")]
    Adapter(#[from] wgpu::RequestAdapterError),
    #[error("device: {0}")]
    Device(#[from] wgpu::RequestDeviceError),
    #[error("polygon tessellation: {0}")]
    Tessellation(#[from] lyon::tessellation::TessellationError),
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub extrude: [f32; 2],
}

#[derive(Default)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

pub struct PreparedTile {
    pub land: Mesh,
    pub green: Mesh,
    pub water: Mesh,
    pub waterway: Mesh,
    pub road_surface: Mesh,
    pub building: Mesh,
    pub coast: Mesh,
    pub boundary: Mesh,
    pub road: Mesh,
    pub road_major: Mesh,
    pub road_collector: Mesh,
    pub road_local: Mesh,
    pub place: Mesh,
    pub station: Mesh,
    pub civic: Mesh,
}

fn add_polygon(
    buffers: &mut VertexBuffers<Vertex, u32>,
    tessellator: &mut FillTessellator,
    polygon: &geo::Polygon<f32>,
) -> Result<(), RenderError> {
    let mut builder = Path::builder();
    for ring in std::iter::once(polygon.exterior()).chain(polygon.interiors()) {
        let mut iter = ring.0.iter();
        let Some(first) = iter.next() else {
            continue;
        };
        builder.begin(point(first.x, first.y));
        for p in iter {
            builder.line_to(point(p.x, p.y));
        }
        builder.end(true);
    }
    let path = builder.build();
    tessellator.tessellate_path(
        &path,
        &FillOptions::default().with_fill_rule(FillRule::EvenOdd),
        &mut BuffersBuilder::new(buffers, |v: FillVertex<'_>| Vertex {
            position: v.position().to_array(),
            extrude: [0.0, 0.0],
        }),
    )?;
    Ok(())
}

fn add_segment(mesh: &mut Mesh, a: geo::Coord<f32>, b: geo::Coord<f32>) -> Option<u32> {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let length = (dx * dx + dy * dy).sqrt();
    if length < 0.001 {
        return None;
    }
    let normal = [-dy / length, dx / length];
    let base = mesh.vertices.len() as u32;
    for (p, side) in [(a, -1.0), (a, 1.0), (b, -1.0), (b, 1.0)] {
        mesh.vertices.push(Vertex {
            position: [p.x, p.y],
            extrude: [normal[0] * side, normal[1] * side],
        });
    }
    mesh.indices
        .extend([base, base + 1, base + 2, base + 2, base + 1, base + 3]);
    Some(base)
}

fn add_line(mesh: &mut Mesh, line: &geo::LineString<f32>) {
    let mut previous: Option<(u32, geo::Coord<f32>, geo::Coord<f32>)> = None;
    for pair in line.0.windows(2) {
        let Some(base) = add_segment(mesh, pair[0], pair[1]) else {
            continue;
        };
        if let Some((previous_base, a, joint)) = previous {
            let turn = (joint.x - a.x) * (pair[1].y - pair[0].y)
                - (joint.y - a.y) * (pair[1].x - pair[0].x);
            // Independent segment quads leave an outer wedge at each bend.
            // Reuse their corner vertices to close it without adding GPU vertices.
            if turn > 0.0 {
                mesh.indices
                    .extend([previous_base + 2, base, previous_base + 3]);
            } else if turn < 0.0 {
                mesh.indices
                    .extend([previous_base + 3, base + 1, previous_base + 2]);
            }
        }
        previous = Some((base, pair[0], pair[1]));
    }
}

fn add_round_cap(mesh: &mut Mesh, p: geo::Coord<f32>) {
    const STEPS: u32 = 8;
    let base = mesh.vertices.len() as u32;
    mesh.vertices.push(Vertex {
        position: [p.x, p.y],
        extrude: [0.0, 0.0],
    });
    for step in 0..=STEPS {
        let angle = std::f32::consts::TAU * step as f32 / STEPS as f32;
        mesh.vertices.push(Vertex {
            position: [p.x, p.y],
            extrude: [angle.cos(), angle.sin()],
        });
        if step > 0 {
            mesh.indices.extend([base, base + step, base + step + 1]);
        }
    }
}

fn add_coast(mesh: &mut Mesh, ring: &geo::LineString<f32>) {
    let mut cap = vec![false; ring.0.len()];
    for (index, pair) in ring.0.windows(2).enumerate() {
        let (a, b) = (pair[0], pair[1]);
        // Tile clipping edges are not coastlines; outlining them would expose seams.
        let clipped_edge = [0.0, mappa_map_data::EXTENT]
            .into_iter()
            .any(|edge| (a.x == edge && b.x == edge) || (a.y == edge && b.y == edge));
        if !clipped_edge {
            add_segment(mesh, a, b);
            cap[index] = true;
            cap[index + 1] = true;
        }
    }
    for (index, p) in ring.0.iter().enumerate() {
        if cap[index] && (index == 0 || index + 1 != ring.0.len() || *p != ring.0[0]) {
            add_round_cap(mesh, *p);
        }
    }
}

pub fn prepare(tile: &DecodedTile) -> Result<PreparedTile, RenderError> {
    let mut prepared = PreparedTile {
        land: Mesh::default(),
        green: Mesh::default(),
        water: Mesh::default(),
        waterway: Mesh::default(),
        road_surface: Mesh::default(),
        building: Mesh::default(),
        coast: Mesh::default(),
        boundary: Mesh::default(),
        road: Mesh::default(),
        road_major: Mesh::default(),
        road_collector: Mesh::default(),
        road_local: Mesh::default(),
        place: Mesh::default(),
        station: Mesh::default(),
        civic: Mesh::default(),
    };
    let mut tessellator = FillTessellator::new();
    let mut land = VertexBuffers::new();
    let mut water = VertexBuffers::new();
    let mut green = VertexBuffers::new();
    let mut road_surface = VertexBuffers::new();
    let mut building = VertexBuffers::new();
    for p in &tile.land {
        add_polygon(&mut land, &mut tessellator, p)?;
        if !tile.detailed {
            for ring in std::iter::once(p.exterior()).chain(p.interiors()) {
                add_coast(&mut prepared.coast, ring);
            }
        }
    }
    for p in &tile.water {
        add_polygon(&mut water, &mut tessellator, p)?;
    }
    for p in &tile.green {
        add_polygon(&mut green, &mut tessellator, p)?;
    }
    for p in &tile.road_surface {
        add_polygon(&mut road_surface, &mut tessellator, p)?;
    }
    for p in &tile.building {
        add_polygon(&mut building, &mut tessellator, p)?;
    }
    prepared.land = Mesh {
        vertices: land.vertices,
        indices: land.indices,
    };
    prepared.water = Mesh {
        vertices: water.vertices,
        indices: water.indices,
    };
    prepared.green = Mesh {
        vertices: green.vertices,
        indices: green.indices,
    };
    prepared.road_surface = Mesh {
        vertices: road_surface.vertices,
        indices: road_surface.indices,
    };
    prepared.building = Mesh {
        vertices: building.vertices,
        indices: building.indices,
    };
    for l in &tile.boundary {
        add_line(&mut prepared.boundary, l);
    }
    for l in &tile.waterway {
        add_line(&mut prepared.waterway, l);
    }
    for l in &tile.road {
        add_line(&mut prepared.road, l);
    }
    for l in &tile.road_major {
        add_line(&mut prepared.road_major, l);
    }
    for l in &tile.road_collector {
        add_line(&mut prepared.road_collector, l);
    }
    for l in &tile.road_local {
        add_line(&mut prepared.road_local, l);
    }
    for place in &tile.place {
        let target = match place.kind {
            PlaceKind::City => &mut prepared.place,
            PlaceKind::Station => &mut prepared.station,
            PlaceKind::Civic => &mut prepared.civic,
            PlaceKind::District => &mut prepared.place,
        };
        add_round_cap(target, place.point.0);
    }
    Ok(prepared)
}

#[derive(Debug, Clone, Copy)]
pub struct MapStyle {
    pub name: &'static str,
    pub ocean: [f32; 4],
    pub land: [f32; 4],
    pub lake: [f32; 4],
    pub green: [f32; 4],
    pub building: [f32; 4],
    pub coast: [f32; 4],
    pub coast_width: f32,
    pub boundary: [f32; 4],
    pub boundary_width: f32,
    pub road_local_casing: [f32; 4],
    pub road_collector_casing: [f32; 4],
    pub road_major_casing: [f32; 4],
    pub road_major_fill: [f32; 4],
}

pub const STYLES: [MapStyle; 4] = [
    MapStyle {
        name: "Arcade",
        ocean: [0.10, 0.34, 0.58, 1.0],
        land: [0.76, 0.78, 0.74, 1.0],
        lake: [0.10, 0.34, 0.58, 1.0],
        green: [0.22, 0.48, 0.31, 1.0],
        building: [0.42, 0.48, 0.52, 1.0],
        coast: [0.10, 0.28, 0.39, 1.0],
        coast_width: 1.5,
        boundary: [0.23, 0.27, 0.30, 1.0],
        boundary_width: 1.0,
        road_local_casing: [0.25, 0.31, 0.32, 1.0],
        road_collector_casing: [0.17, 0.25, 0.28, 1.0],
        road_major_casing: [0.12, 0.20, 0.25, 1.0],
        road_major_fill: [0.76, 0.78, 0.77, 1.0],
    },
    MapStyle {
        name: "Lagoon",
        ocean: [0.162, 0.479, 0.831, 1.0],
        land: [0.309, 0.799, 0.456, 1.0],
        lake: [0.162, 0.479, 0.831, 1.0],
        green: [0.18, 0.68, 0.37, 1.0],
        building: [0.42, 0.48, 0.52, 1.0],
        coast: [0.031, 0.479, 0.319, 1.0],
        coast_width: 5.0,
        boundary: [0.074, 0.332, 0.227, 1.0],
        boundary_width: 1.4,
        road_local_casing: [0.39, 0.45, 0.46, 1.0],
        road_collector_casing: [0.28, 0.36, 0.39, 1.0],
        road_major_casing: [0.22, 0.32, 0.38, 1.0],
        road_major_fill: [0.64, 0.71, 0.72, 1.0],
    },
    MapStyle {
        name: "Candy",
        ocean: [0.413, 0.328, 0.847, 1.0],
        land: [0.939, 0.429, 0.509, 1.0],
        lake: [0.413, 0.328, 0.847, 1.0],
        green: [0.56, 0.79, 0.44, 1.0],
        building: [0.42, 0.48, 0.52, 1.0],
        coast: [0.799, 0.191, 0.328, 1.0],
        coast_width: 5.0,
        boundary: [0.381, 0.136, 0.279, 1.0],
        boundary_width: 1.4,
        road_local_casing: [0.39, 0.45, 0.46, 1.0],
        road_collector_casing: [0.28, 0.36, 0.39, 1.0],
        road_major_casing: [0.22, 0.32, 0.38, 1.0],
        road_major_fill: [0.64, 0.71, 0.72, 1.0],
    },
    MapStyle {
        name: "Sunset",
        ocean: [0.191, 0.205, 0.768, 1.0],
        land: [1.0, 0.462, 0.188, 1.0],
        lake: [0.191, 0.205, 0.768, 1.0],
        green: [0.42, 0.74, 0.39, 1.0],
        building: [0.42, 0.48, 0.52, 1.0],
        coast: [0.871, 0.188, 0.156, 1.0],
        coast_width: 5.0,
        boundary: [0.474, 0.109, 0.168, 1.0],
        boundary_width: 1.4,
        road_local_casing: [0.39, 0.45, 0.46, 1.0],
        road_collector_casing: [0.28, 0.36, 0.39, 1.0],
        road_major_casing: [0.22, 0.32, 0.38, 1.0],
        road_major_fill: [0.64, 0.71, 0.72, 1.0],
    },
];

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TileUniform {
    origin: [f32; 2],
    tile_px: f32,
    line_width: f32,
    viewport: [f32; 2],
    padding: [f32; 2],
    color: [f32; 4],
}

struct GpuMesh {
    vertex: wgpu::Buffer,
    index: wgpu::Buffer,
    indices: u32,
    bytes: usize,
}
struct GpuTile {
    land: Option<GpuMesh>,
    green: Option<GpuMesh>,
    water: Option<GpuMesh>,
    waterway: Option<GpuMesh>,
    road_surface: Option<GpuMesh>,
    building: Option<GpuMesh>,
    coast: Option<GpuMesh>,
    boundary: Option<GpuMesh>,
    road: Option<GpuMesh>,
    road_major: Option<GpuMesh>,
    road_collector: Option<GpuMesh>,
    road_local: Option<GpuMesh>,
    place: Option<GpuMesh>,
    station: Option<GpuMesh>,
    civic: Option<GpuMesh>,
    touched: u64,
    bytes: usize,
}
#[derive(Clone, Copy)]
struct Draw {
    key: TileKey,
    layer: u8,
    offset: u32,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct GpuStats {
    pub hits: u64,
    pub uploads: u64,
    pub evictions: u64,
    pub bytes: usize,
}

pub struct MapRenderer {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    cache: HashMap<TileKey, GpuTile>,
    tick: u64,
    pub stats: GpuStats,
    scratch: Vec<u8>,
    draws: Vec<Draw>,
}

impl MapRenderer {
    pub async fn new(format: wgpu::TextureFormat) -> Result<Self, RenderError> {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = wgpu::Backends::METAL;
        let instance = wgpu::Instance::new(descriptor);
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await?;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Mappa basemap"),
            source: wgpu::ShaderSource::Wgsl(include_str!("map.wgsl").into()),
        });
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tile uniforms"),
            size: UNIFORM_STRIDE * MAX_DRAWS as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tile layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: NonZeroU64::new(mem::size_of::<TileUniform>() as u64),
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tile bind group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uniform_buffer,
                    offset: 0,
                    size: NonZeroU64::new(mem::size_of::<TileUniform>() as u64),
                }),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("map pipeline"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("map fill and line"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 8,
                            shader_location: 1,
                        },
                    ],
                })],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            pipeline,
            uniform_buffer,
            bind_group,
            cache: HashMap::new(),
            tick: 0,
            stats: GpuStats::default(),
            scratch: Vec::with_capacity(UNIFORM_STRIDE as usize * 128),
            draws: Vec::with_capacity(128),
        })
    }

    fn upload_mesh(&self, mesh: Mesh) -> Option<GpuMesh> {
        if mesh.indices.is_empty() {
            return None;
        }
        let bytes = mesh.vertices.len() * mem::size_of::<Vertex>() + mesh.indices.len() * 4;
        let vertex = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("map vertices"),
                contents: bytemuck::cast_slice(&mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let index = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("map indices"),
                contents: bytemuck::cast_slice(&mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        Some(GpuMesh {
            vertex,
            index,
            indices: mesh.indices.len() as u32,
            bytes,
        })
    }

    pub fn has_tile(&mut self, key: TileKey) -> bool {
        if let Some(tile) = self.cache.get_mut(&key) {
            self.tick += 1;
            tile.touched = self.tick;
            self.stats.hits += 1;
            true
        } else {
            false
        }
    }

    pub fn contains_tile(&self, key: TileKey) -> bool {
        self.cache.contains_key(&key)
    }

    pub fn upload_tile(&mut self, key: TileKey, tile: PreparedTile) {
        if self.cache.contains_key(&key) {
            return;
        }
        let land = self.upload_mesh(tile.land);
        let green = self.upload_mesh(tile.green);
        let water = self.upload_mesh(tile.water);
        let waterway = self.upload_mesh(tile.waterway);
        let road_surface = self.upload_mesh(tile.road_surface);
        let building = self.upload_mesh(tile.building);
        let coast = self.upload_mesh(tile.coast);
        let boundary = self.upload_mesh(tile.boundary);
        let road = self.upload_mesh(tile.road);
        let road_major = self.upload_mesh(tile.road_major);
        let road_collector = self.upload_mesh(tile.road_collector);
        let road_local = self.upload_mesh(tile.road_local);
        let place = self.upload_mesh(tile.place);
        let station = self.upload_mesh(tile.station);
        let civic = self.upload_mesh(tile.civic);
        let bytes = [
            &land,
            &green,
            &water,
            &waterway,
            &road_surface,
            &building,
            &coast,
            &boundary,
            &road,
            &road_major,
            &road_collector,
            &road_local,
            &place,
            &station,
            &civic,
        ]
        .into_iter()
        .flatten()
        .map(|m| m.bytes)
        .sum();
        self.tick += 1;
        self.cache.insert(
            key,
            GpuTile {
                land,
                green,
                water,
                waterway,
                road_surface,
                building,
                coast,
                boundary,
                road,
                road_major,
                road_collector,
                road_local,
                place,
                station,
                civic,
                touched: self.tick,
                bytes,
            },
        );
        self.stats.uploads += 1;
        self.stats.bytes += bytes;
        while self.stats.bytes > GPU_BUDGET && self.cache.len() > 1 {
            let Some(victim) = self
                .cache
                .iter()
                .filter(|(k, _)| **k != key)
                .min_by_key(|(_, v)| v.touched)
                .map(|(k, _)| *k)
            else {
                break;
            };
            if let Some(old) = self.cache.remove(&victim) {
                self.stats.bytes -= old.bytes;
                self.stats.evictions += 1;
            }
        }
    }

    pub fn render(
        &mut self,
        view: &wgpu::TextureView,
        camera: &MapCamera,
        visible: &[VisibleTile],
        style: MapStyle,
    ) {
        self.draws.clear();
        self.scratch.clear();
        let viewport = [camera.width_px as f32, camera.height_px as f32];
        let tile_px = (camera.world_size_px()
            / (1u32 << visible.first().map_or(0, |v| v.key.z)) as f64) as f32;
        let data_zoom = visible.first().map_or(0, |tile| tile.key.z);
        let local_fill = road_fill(style.road_local_casing, style.land, 0.62);
        let collector_fill = road_fill(style.road_collector_casing, style.land, 0.62);
        let major_fill = road_fill(style.road_major_casing, style.road_major_fill, 0.72);
        for layer in (0..3u8)
            .chain(std::iter::once(20))
            .chain(3..5u8)
            .chain(std::iter::once(19))
            .chain(std::iter::once(21))
            .chain(5..19u8)
        {
            // The OSM land source is distributed in overlapping polygon chunks.
            // Outlining those chunks would draw artificial inland seams.
            if data_zoom >= 8 && layer == 3 {
                continue;
            }
            for placement in visible {
                let Some(tile) = self.cache.get(&placement.key) else {
                    continue;
                };
                let mesh = match layer {
                    0 => &tile.land,
                    1 => &tile.green,
                    2 => &tile.water,
                    20 => &tile.waterway,
                    19 => &tile.road_surface,
                    21 => &tile.building,
                    3 => &tile.coast,
                    4 => &tile.boundary,
                    5 | 6 => &tile.road,
                    7 | 8 => &tile.road_local,
                    9 | 10 => &tile.road_collector,
                    11 | 12 => &tile.road_major,
                    13 | 14 => &tile.place,
                    15 | 16 => &tile.station,
                    _ => &tile.civic,
                };
                if mesh.is_none() || self.draws.len() == MAX_DRAWS {
                    continue;
                }
                let n = (1u32 << placement.key.z) as f64;
                let (ox, oy) = camera.unwrapped_to_screen(mappa_map_core::WorldPoint {
                    x: placement.world_x as f64 / n,
                    y: placement.key.y as f64 / n,
                });
                let color = match layer {
                    0 => style.land,
                    1 => style.green,
                    2 => style.lake,
                    20 => style.lake,
                    19 => local_fill,
                    21 => style.building,
                    3 => style.coast,
                    4 => style.boundary,
                    5 | 7 => style.road_local_casing,
                    6 | 8 => local_fill,
                    10 => collector_fill,
                    13 | 15 | 17 => [1.0; 4],
                    9 => style.road_collector_casing,
                    11 => style.road_major_casing,
                    12 => major_fill,
                    14 => style.coast,
                    16 => [0.03, 0.30, 0.81, 1.0],
                    _ => [0.02, 0.50, 0.32, 1.0],
                };
                let u = TileUniform {
                    origin: [ox as f32, oy as f32],
                    tile_px,
                    line_width: match layer {
                        3 => style.coast_width * camera.scale_factor as f32,
                        4 => style.boundary_width * camera.scale_factor as f32,
                        20 => 1.8 * camera.scale_factor as f32,
                        5 => {
                            (if data_zoom >= 12 {
                                2.4
                            } else if data_zoom >= 11 {
                                3.5
                            } else {
                                5.5
                            }) * camera.scale_factor as f32
                        }
                        6 => {
                            (if data_zoom >= 12 {
                                1.5
                            } else if data_zoom >= 11 {
                                2.2
                            } else {
                                3.5
                            }) * camera.scale_factor as f32
                        }
                        7 => 3.0 * camera.scale_factor as f32,
                        8 => 2.0 * camera.scale_factor as f32,
                        9 => 4.0 * camera.scale_factor as f32,
                        10 => 2.8 * camera.scale_factor as f32,
                        11 => 6.2 * camera.scale_factor as f32,
                        12 => 4.7 * camera.scale_factor as f32,
                        13 => 13.0 * camera.scale_factor as f32,
                        14 => 7.0 * camera.scale_factor as f32,
                        15 => {
                            (if data_zoom <= 11 { 7.0 } else { 10.0 }) * camera.scale_factor as f32
                        }
                        16 => {
                            (if data_zoom <= 11 { 4.0 } else { 6.0 }) * camera.scale_factor as f32
                        }
                        17 => 10.0 * camera.scale_factor as f32,
                        18 => 6.0 * camera.scale_factor as f32,
                        _ => 0.0,
                    },
                    viewport,
                    padding: [0.0; 2],
                    color,
                };
                let offset = (self.draws.len() as u64 * UNIFORM_STRIDE) as usize;
                self.scratch.resize(offset + UNIFORM_STRIDE as usize, 0);
                self.scratch[offset..offset + mem::size_of::<TileUniform>()]
                    .copy_from_slice(bytemuck::bytes_of(&u));
                self.draws.push(Draw {
                    key: placement.key,
                    layer,
                    offset: offset as u32,
                });
            }
        }
        if !self.scratch.is_empty() {
            self.queue
                .write_buffer(&self.uniform_buffer, 0, &self.scratch);
        }
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("map frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("basemap"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: style.ocean[0] as f64,
                            g: style.ocean[1] as f64,
                            b: style.ocean[2] as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            for draw in &self.draws {
                let tile = &self.cache[&draw.key];
                let mesh = match draw.layer {
                    0 => &tile.land,
                    1 => &tile.green,
                    2 => &tile.water,
                    20 => &tile.waterway,
                    19 => &tile.road_surface,
                    21 => &tile.building,
                    3 => &tile.coast,
                    4 => &tile.boundary,
                    5 | 6 => &tile.road,
                    7 | 8 => &tile.road_local,
                    9 | 10 => &tile.road_collector,
                    11 | 12 => &tile.road_major,
                    13 | 14 => &tile.place,
                    15 | 16 => &tile.station,
                    _ => &tile.civic,
                };
                let Some(mesh) = mesh else {
                    continue;
                };
                pass.set_bind_group(0, &self.bind_group, &[draw.offset]);
                pass.set_vertex_buffer(0, mesh.vertex.slice(..));
                pass.set_index_buffer(mesh.index.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.indices, 0, 0..1);
            }
        }
        self.queue.submit(Some(encoder.finish()));
    }

    pub fn cached_tiles(&self) -> usize {
        self.cache.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coastline_omits_clipped_tile_edges() {
        let ring = geo::LineString::from(vec![
            (0.0, 0.0),
            (4096.0, 0.0),
            (4096.0, 100.0),
            (0.0, 100.0),
            (0.0, 0.0),
        ]);
        let mut coast = Mesh::default();
        add_coast(&mut coast, &ring);
        assert!(coast.vertices.iter().all(|v| v.position[1] == 100.0));
        assert!(!coast.indices.is_empty());
    }

    #[test]
    fn road_bend_closes_outer_wedge_without_extra_vertices() {
        let mut bend = Mesh::default();
        add_line(
            &mut bend,
            &geo::LineString::from(vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]),
        );
        assert_eq!(bend.vertices.len(), 8);
        assert_eq!(&bend.indices[12..], &[2, 4, 3]);

        let mut straight = Mesh::default();
        add_line(
            &mut straight,
            &geo::LineString::from(vec![(0.0, 0.0), (10.0, 0.0), (20.0, 0.0)]),
        );
        assert_eq!(straight.indices.len(), 12);
    }
}
