//! Real populated-place names over the offline detail map.

use super::{DynError, FORMAT, LabelKind, MapCamera, MapRenderer, ScreenLabel};
use crate::scale::ScaleOverlay;
use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache,
    TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use std::collections::HashMap;

const LABEL_CACHE_LIMIT: usize = 512;

#[derive(Hash, PartialEq, Eq)]
struct LabelKey {
    name: String,
    kind: LabelKind,
    scale_bits: u32,
}

pub struct LabelRenderer {
    fonts: FontSystem,
    swash: SwashCache,
    atlas: TextAtlas,
    viewport: Viewport,
    text: TextRenderer,
    buffers: HashMap<LabelKey, Buffer>,
    scale_overlay: ScaleOverlay,
}

impl LabelRenderer {
    pub fn new(renderer: &MapRenderer) -> Self {
        let cache = Cache::new(&renderer.device);
        let viewport = Viewport::new(&renderer.device, &cache);
        let mut atlas = TextAtlas::new(&renderer.device, &renderer.queue, &cache, FORMAT);
        let text = TextRenderer::new(
            &mut atlas,
            &renderer.device,
            wgpu::MultisampleState::default(),
            None,
        );
        Self {
            fonts: FontSystem::new(),
            swash: SwashCache::new(),
            atlas,
            viewport,
            text,
            buffers: HashMap::new(),
            scale_overlay: ScaleOverlay::new(renderer),
        }
    }

    pub fn draw(
        &mut self,
        renderer: &MapRenderer,
        view: &wgpu::TextureView,
        camera: &MapCamera,
        labels: &[ScreenLabel],
    ) -> Result<(), DynError> {
        let overlay = self.scale_overlay.layout(camera);
        self.scale_overlay.upload(renderer, &overlay);
        self.viewport.update(
            &renderer.queue,
            Resolution {
                width: camera.width_px,
                height: camera.height_px,
            },
        );
        let scale = camera.scale_factor as f32;
        if self.buffers.len() > LABEL_CACHE_LIMIT {
            self.buffers.clear();
        }
        let labels: Vec<&ScreenLabel> = labels
            .iter()
            .filter(|label| {
                let (left, top) = label.text_origin(scale);
                let right = left + label.text_width(scale);
                let bottom = top + 26.0 * scale;
                !overlay.intersects(left, top, right, bottom)
            })
            .chain(overlay.labels.iter())
            .collect();
        let mut keys = Vec::with_capacity(labels.len());
        for label in &labels {
            let key = LabelKey {
                name: label.name.clone(),
                kind: label.kind,
                scale_bits: scale.to_bits(),
            };
            if !self.buffers.contains_key(&key) {
                let size = match label.kind {
                    LabelKind::Country => 17.0,
                    LabelKind::City => 19.0,
                    LabelKind::Station => 16.0,
                    LabelKind::Civic => 14.0,
                    LabelKind::Attribution => 11.0,
                    LabelKind::LegendStation | LabelKind::LegendCivic => 13.0,
                    LabelKind::Scale => 13.0,
                };
                let mut buffer = Buffer::new(
                    &mut self.fonts,
                    Metrics::new(size * scale, (size + 6.0) * scale),
                );
                buffer.set_size(Some(280.0 * scale), Some(32.0 * scale));
                buffer.set_text(
                    &label.name,
                    &Attrs::new().family(Family::SansSerif),
                    Shaping::Advanced,
                    None,
                );
                buffer.shape_until_scroll(&mut self.fonts, false);
                self.buffers.insert(
                    LabelKey {
                        name: key.name.clone(),
                        kind: key.kind,
                        scale_bits: key.scale_bits,
                    },
                    buffer,
                );
            }
            keys.push(key);
        }
        let areas: Vec<_> = labels
            .iter()
            .zip(&keys)
            .map(|(label, key)| {
                let buffer = &self.buffers[key];
                let (left, top) = label.text_origin(scale);
                TextArea {
                    buffer,
                    left,
                    top,
                    scale: 1.0,
                    bounds: TextBounds {
                        left: 0,
                        top: 0,
                        right: camera.width_px as i32,
                        bottom: camera.height_px as i32,
                    },
                    default_color: match label.kind {
                        LabelKind::Station | LabelKind::LegendStation => Color::rgb(12, 55, 120),
                        LabelKind::Civic | LabelKind::LegendCivic => Color::rgb(8, 83, 53),
                        LabelKind::Attribution => Color::rgb(72, 80, 84),
                        LabelKind::Scale => Color::rgb(30, 48, 65),
                        _ => Color::rgb(48, 50, 58),
                    },
                    custom_glyphs: &[],
                }
            })
            .collect();
        self.text.prepare(
            &renderer.device,
            &renderer.queue,
            &mut self.fonts,
            &mut self.atlas,
            &self.viewport,
            areas,
            &mut self.swash,
        )?;
        let mut encoder = renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("place labels"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("place labels over map"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            self.scale_overlay.render(&mut pass, &overlay);
            self.text.render(&self.atlas, &self.viewport, &mut pass)?;
        }
        renderer.queue.submit(Some(encoder.finish()));
        self.atlas.trim();
        Ok(())
    }
}
