//! Viewport scale, computed from the actual camera projection and display scale.

use super::{FORMAT, LabelKind, MapCamera, MapRenderer, ScreenLabel};
use bytemuck::{Pod, Zeroable};
use std::mem;

const MAX_VERTICES: usize = 12 * 6;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

pub struct ScaleOverlay {
    pipeline: wgpu::RenderPipeline,
    vertices: wgpu::Buffer,
}

pub struct ScaleLayout {
    bounds: [f32; 4],
    vertices: Vec<Vertex>,
    pub labels: Vec<ScreenLabel>,
}

impl ScaleLayout {
    pub fn intersects(&self, left: f32, top: f32, right: f32, bottom: f32) -> bool {
        left < self.bounds[2]
            && right > self.bounds[0]
            && top < self.bounds[3]
            && bottom > self.bounds[1]
    }
}

impl ScaleOverlay {
    pub fn new(renderer: &MapRenderer) -> Self {
        let shader = renderer
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("scale overlay shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("scale.wgsl").into()),
            });
        let pipeline = renderer
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("scale overlay"),
                layout: None,
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
                                format: wgpu::VertexFormat::Float32x4,
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
                        format: FORMAT,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });
        let vertices = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scale overlay vertices"),
            size: (MAX_VERTICES * mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { pipeline, vertices }
    }

    pub fn layout(&self, camera: &MapCamera) -> ScaleLayout {
        let scale = camera.scale_factor as f32;
        let width = 270.0 * scale;
        let height = 78.0 * scale;
        let x = (camera.width_px as f32 - width - 14.0 * scale).max(8.0 * scale);
        let y = (camera.height_px as f32 - height - 14.0 * scale).max(8.0 * scale);
        let bounds = [
            x,
            y,
            (x + width).min(camera.width_px as f32),
            (y + height).min(camera.height_px as f32),
        ];
        let meters_per_pixel = camera.meters_per_pixel_at_center();
        let distance_m = nice_distance(meters_per_pixel * 115.0 * camera.scale_factor * 1.12);
        let bar_px = (distance_m / meters_per_pixel) as f32;
        let mut vertices = Vec::with_capacity(MAX_VERTICES);
        let viewport = [camera.width_px as f32, camera.height_px as f32];
        let white = [0.94, 0.96, 0.97, 0.96];
        let border = [0.12, 0.18, 0.23, 0.58];
        let ink = [0.015, 0.032, 0.06, 1.0];
        rect(
            &mut vertices,
            viewport,
            [x, y, x + width, y + height],
            white,
        );
        rect(
            &mut vertices,
            viewport,
            [x, y, x + width, y + scale],
            border,
        );
        rect(
            &mut vertices,
            viewport,
            [x, y + height - scale, x + width, y + height],
            border,
        );
        rect(
            &mut vertices,
            viewport,
            [x, y, x + scale, y + height],
            border,
        );
        rect(
            &mut vertices,
            viewport,
            [x + width - scale, y, x + width, y + height],
            border,
        );
        let bar_x = x + 13.0 * scale;
        let bar_y = y + 62.0 * scale;
        let thick = 2.0 * scale;
        rect(
            &mut vertices,
            viewport,
            [bar_x, bar_y, bar_x + bar_px, bar_y + thick],
            ink,
        );
        rect(
            &mut vertices,
            viewport,
            [
                bar_x,
                bar_y - 7.0 * scale,
                bar_x + thick,
                bar_y + 5.0 * scale,
            ],
            ink,
        );
        rect(
            &mut vertices,
            viewport,
            [
                bar_x + bar_px - thick,
                bar_y - 7.0 * scale,
                bar_x + bar_px,
                bar_y + 5.0 * scale,
            ],
            ink,
        );

        let labels = vec![
            ScreenLabel {
                name: format!(
                    "중심 {}  ·  z{:.1}",
                    format_resolution(meters_per_pixel),
                    camera.zoom
                ),
                x: x + 12.0 * scale,
                y: y + 7.0 * scale,
                rank: 0,
                kind: LabelKind::Scale,
            },
            ScreenLabel {
                name: format_distance(distance_m),
                x: x + 13.0 * scale,
                y: y + 32.0 * scale,
                rank: 0,
                kind: LabelKind::Scale,
            },
        ];
        ScaleLayout {
            bounds,
            vertices,
            labels,
        }
    }

    pub fn upload(&self, renderer: &MapRenderer, layout: &ScaleLayout) {
        renderer
            .queue
            .write_buffer(&self.vertices, 0, bytemuck::cast_slice(&layout.vertices));
    }

    pub fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, layout: &ScaleLayout) {
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.draw(0..layout.vertices.len() as u32, 0..1);
    }
}

fn rect(vertices: &mut Vec<Vertex>, viewport: [f32; 2], bounds: [f32; 4], color: [f32; 4]) {
    let [x0, y0, x1, y1] = bounds;
    let clip = |x: f32, y: f32| [2.0 * x / viewport[0] - 1.0, 1.0 - 2.0 * y / viewport[1]];
    for position in [
        clip(x0, y0),
        clip(x1, y0),
        clip(x0, y1),
        clip(x0, y1),
        clip(x1, y0),
        clip(x1, y1),
    ] {
        vertices.push(Vertex { position, color });
    }
}

fn nice_distance(max_meters: f64) -> f64 {
    let unit = 10.0_f64.powf(max_meters.log10().floor());
    let step = max_meters / unit;
    if step >= 5.0 {
        5.0 * unit
    } else if step >= 2.0 {
        2.0 * unit
    } else {
        unit
    }
}

fn format_distance(meters: f64) -> String {
    if meters >= 1_000.0 {
        format!("{} km", meters / 1_000.0)
    } else if meters < 1.0 {
        format!("{meters:.1} m")
    } else {
        format!("{meters:.0} m")
    }
}

fn format_resolution(meters_per_pixel: f64) -> String {
    if meters_per_pixel >= 1_000.0 {
        format!("{:.1} km/px", meters_per_pixel / 1_000.0)
    } else if meters_per_pixel >= 100.0 {
        format!("{meters_per_pixel:.0} m/px")
    } else if meters_per_pixel >= 10.0 {
        format!("{meters_per_pixel:.1} m/px")
    } else {
        format!("{meters_per_pixel:.2} m/px")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_distance_is_a_real_screen_distance() {
        for meters_per_pixel in [0.2, 1.0, 25.0, 1_000.0, 80_000.0] {
            let distance = nice_distance(meters_per_pixel * 115.0 * 1.12);
            let width_px = distance / meters_per_pixel;
            assert!((40.0..=129.0).contains(&width_px));
        }
    }
}
