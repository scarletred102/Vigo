// Text shader — alpha-tested glyph rendering from an atlas texture.
// Each instance is one glyph: position + UV coords into the atlas + color.

struct Viewport {
    size: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> viewport: Viewport;
@group(1) @binding(0) var atlas_tex: texture_2d<f32>;
@group(1) @binding(1) var atlas_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

// Per-glyph instance data.
struct GlyphData {
    // Pixel rect: x, y, width, height (screen position).
    @location(0) rect: vec4<f32>,
    // Atlas UV: u_min, v_min, u_max, v_max (normalised 0..1).
    @location(1) uv_rect: vec4<f32>,
    // Text color: r, g, b, a.
    @location(2) color: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vi: u32,
    glyph: GlyphData,
) -> VertexOutput {
    // 6-vertex unit quad (two triangles).
    var corners = array<vec2<f32>, 6>(
        vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0),
        vec2(0.0, 1.0), vec2(1.0, 0.0), vec2(1.0, 1.0),
    );
    let corner = corners[vi];

    // Screen-space pixel position.
    let px = glyph.rect.xy + corner * glyph.rect.zw;

    // Pixels → NDC.
    let ndc = vec2(
        (px.x / viewport.size.x) * 2.0 - 1.0,
        1.0 - (px.y / viewport.size.y) * 2.0,
    );

    // Interpolate UV across the glyph's atlas region.
    let uv = mix(glyph.uv_rect.xy, glyph.uv_rect.zw, corner);

    var out: VertexOutput;
    out.position = vec4(ndc, 0.0, 1.0);
    out.uv = uv;
    out.color = glyph.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample alpha from the glyph atlas (stored in R channel).
    let alpha = textureSample(atlas_tex, atlas_sampler, in.uv).r;

    // Discard fully transparent fragments.
    if alpha < 0.004 {
        discard;
    }

    return vec4(in.color.rgb, in.color.a * alpha);
}
