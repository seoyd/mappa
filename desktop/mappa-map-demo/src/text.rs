//! Optional desktop-only multilingual text feasibility probe.
use super::{
    DynError, FORMAT, MapCamera, MapRenderer, Path, STYLES, TileManager, runtime, screenshot_inner,
};
use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache,
    TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};

pub fn run() -> Result<(), DynError> {
    let runtime = runtime()?;
    let mut renderer = runtime.block_on(MapRenderer::new(FORMAT))?;
    let mut manager = runtime.block_on(TileManager::open())?;
    let camera = MapCamera::new(70.0, 30.0, 1.3, 1200, 720, 1.0)?;
    let visible = camera.visible_tiles(manager.max_zoom_for(&camera), 1);
    runtime.block_on(manager.ensure(&mut renderer, &visible, usize::MAX));
    std::fs::create_dir_all("artifacts/map-v0.3a")?;
    let path = Path::new("artifacts/map-v0.3a/text-rnd.png");
    screenshot_inner(
        &mut renderer,
        &camera,
        &visible,
        super::map_style(0),
        path,
        |view, renderer| {
            let mut fonts = FontSystem::new();
            let mut swash = SwashCache::new();
            let cache = Cache::new(&renderer.device);
            let mut viewport = Viewport::new(&renderer.device, &cache);
            viewport.update(
                &renderer.queue,
                Resolution {
                    width: camera.width_px,
                    height: camera.height_px,
                },
            );
            let mut atlas = TextAtlas::new(&renderer.device, &renderer.queue, &cache, FORMAT);
            let mut text = TextRenderer::new(
                &mut atlas,
                &renderer.device,
                wgpu::MultisampleState::default(),
                None,
            );
            let mut buffer = Buffer::new(&mut fonts, Metrics::new(36.0, 76.0));
            buffer.set_size(Some(750.0), Some(600.0));
            buffer.set_text(
                "Seoul\n서울\n東京\n北京\nМосква\nالقاهرة",
                &Attrs::new().family(Family::SansSerif),
                Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(&mut fonts, false);
            text.prepare(
                &renderer.device,
                &renderer.queue,
                &mut fonts,
                &mut atlas,
                &viewport,
                [TextArea {
                    buffer: &buffer,
                    left: 70.0,
                    top: 80.0,
                    scale: 1.0,
                    bounds: TextBounds {
                        left: 0,
                        top: 0,
                        right: 1200,
                        bottom: 720,
                    },
                    default_color: Color::rgb(22, 50, 54),
                    custom_glyphs: &[],
                }],
                &mut swash,
            )?;
            let mut encoder =
                renderer
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("text prototype"),
                    });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("text over map"),
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
                text.render(&atlas, &viewport, &mut pass)?;
            }
            renderer.queue.submit(Some(encoder.finish()));
            atlas.trim();
            Ok(())
        },
    )?;
    println!("text R&D: {} / {}", path.display(), STYLES[0].name);
    Ok(())
}
