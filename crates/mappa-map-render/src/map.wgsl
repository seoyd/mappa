struct TileUniform {
    origin: vec2<f32>,
    tile_px: f32,
    line_width: f32,
    viewport: vec2<f32>,
    padding: vec2<f32>,
    color: vec4<f32>,
};
@group(0) @binding(0) var<uniform> tile: TileUniform;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) extrude: vec2<f32>,
};
struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) extrude: vec2<f32>,
};
@vertex fn vs_main(input: VertexInput) -> VertexOutput {
    let screen = tile.origin + input.position * (tile.tile_px / 4096.0) + input.extrude * (tile.line_width * 0.5 + 0.5);
    var out: VertexOutput;
    out.clip = vec4<f32>(2.0 * screen.x / tile.viewport.x - 1.0, 1.0 - 2.0 * screen.y / tile.viewport.y, 0.0, 1.0);
    out.extrude = input.extrude;
    return out;
}
@fragment fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    if tile.line_width == 0.0 {
        return tile.color;
    }
    let edge_px = length(input.extrude) * (tile.line_width * 0.5 + 0.5);
    let coverage = clamp(tile.line_width * 0.5 + 0.5 - edge_px, 0.0, 1.0);
    return vec4<f32>(tile.color.rgb, tile.color.a * coverage);
}
