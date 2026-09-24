//! Real populated-place names over the offline detail map.

use super::{DynError, FORMAT, LabelKind, MapCamera, MapRenderer, ScreenLabel};
use crate::scale::ScaleOverlay;
use glyphon::{
    Attrs, Buffer, Cache, Color, ContentType, CustomGlyph, Family, FontSystem, Metrics,
    RasterizeCustomGlyphRequest, RasterizedCustomGlyph, Resolution, Shaping, SwashCache, TextArea,
    TextAtlas, TextBounds, TextRenderer, Viewport,
};
use std::collections::HashMap;
use std::io::Cursor;

const LABEL_CACHE_LIMIT: usize = 512;
const CC_BY_LOGO: &[u8] = include_bytes!("../../../assets/map/licenses/land-tasmania-cc-by.png");

fn rasterize_cc_by_logo(request: RasterizeCustomGlyphRequest) -> Option<RasterizedCustomGlyph> {
    if request.id != 1 || request.width == 0 || request.height == 0 {
        return None;
    }
    let mut decoder = png::Decoder::new(Cursor::new(CC_BY_LOGO));
    decoder.set_transformations(png::Transformations::IDENTITY);
    let mut reader = decoder.read_info().ok()?;
    let mut source = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut source).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    let width = usize::from(request.width);
    let height = usize::from(request.height);
    let source_width = info.width as usize;
    let source_height = info.height as usize;
    let mut data = vec![0; width * height * 4];
    for y in 0..height {
        let source_y = (y * source_height / height).min(source_height - 1);
        for x in 0..width {
            let source_x = (x * source_width / width).min(source_width - 1);
            let from = (source_y * source_width + source_x) * 4;
            let to = (y * width + x) * 4;
            data[to..to + 4].copy_from_slice(&source[from..from + 4]);
        }
    }
    Some(RasterizedCustomGlyph {
        data,
        content_type: ContentType::Color,
    })
}

#[derive(Hash, PartialEq, Eq)]
struct LabelKey {
    name: String,
    kind: LabelKind,
    scale_bits: u32,
    width_bits: u32,
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
                if label.kind == LabelKind::Attribution {
                    return true;
                }
                let (left, top) = label.text_origin(scale);
                let right = left + label.text_width(scale);
                let bottom = top + 26.0 * scale;
                !overlay.intersects(left, top, right, bottom)
            })
            .chain(overlay.labels.iter())
            .collect();
        let mut keys = Vec::with_capacity(labels.len());
        for label in &labels {
            let attribution_above_scale = label.kind == LabelKind::Attribution
                && label.y < camera.height_px as f32 - 90.0 * scale;
            let width = if attribution_above_scale {
                camera.width_px as f32 - 24.0 * scale
            } else if label.kind == LabelKind::Attribution {
                camera.width_px as f32 - 300.0 * scale
            } else {
                280.0 * scale
            }
            .max(100.0 * scale);
            let key = LabelKey {
                name: label.name.clone(),
                kind: label.kind,
                scale_bits: scale.to_bits(),
                width_bits: width.to_bits(),
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
                buffer.set_size(
                    Some(width),
                    Some(if attribution_above_scale {
                        80.0 * scale
                    } else {
                        32.0 * scale
                    }),
                );
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
                        width_bits: key.width_bits,
                    },
                    buffer,
                );
            }
            keys.push(key);
        }
        let custom_glyphs: Vec<Vec<CustomGlyph>> = labels
            .iter()
            .map(|label| {
                if label.kind == LabelKind::Attribution
                    && label.name.contains("LIST Transport Segments")
                {
                    vec![CustomGlyph {
                        id: 1,
                        left: 0.0,
                        top: -35.0 * scale,
                        width: 88.0 * scale,
                        height: 31.0 * scale,
                        color: None,
                        snap_to_physical_pixel: true,
                        metadata: 0,
                    }]
                } else {
                    Vec::new()
                }
            })
            .collect();
        let areas: Vec<_> = labels
            .iter()
            .zip(&keys)
            .zip(&custom_glyphs)
            .map(|((label, key), glyphs)| {
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
                    custom_glyphs: glyphs,
                }
            })
            .collect();
        self.text.prepare_with_custom(
            &renderer.device,
            &renderer.queue,
            &mut self.fonts,
            &mut self.atlas,
            &self.viewport,
            areas,
            &mut self.swash,
            rasterize_cc_by_logo,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_cc_by_logo_rasterizes_at_screen_size() {
        let raster = rasterize_cc_by_logo(RasterizeCustomGlyphRequest {
            id: 1,
            width: 88,
            height: 31,
            x_bin: glyphon::SubpixelBin::Zero,
            y_bin: glyphon::SubpixelBin::Zero,
            scale: 1.0,
        })
        .expect("embedded Land Tasmania CC BY logo");
        assert_eq!(raster.content_type, ContentType::Color);
        assert_eq!(raster.data.len(), 88 * 31 * 4);
        assert!(raster.data.chunks_exact(4).any(|pixel| pixel[3] > 0));
    }
}
