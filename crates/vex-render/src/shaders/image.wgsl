// Image shader — textured quad rendering for decoded images.
// Each instance is one image quad with position + UV into an image atlas.

struct Viewport {
    size: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> viewport: Viewport;
@group(1) @binding(0) var image_tex: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) opacity: f32,
};

// Per-image instance data.
struct ImageData {
    // Pixel rect: x, y, width, height (screen position).
    @location(0) rect: vec4<f32>,
    // Atlas UV: u_min, v_min, u_max, v_max (normalised 0..1).
    @location(1) uv_rect: vec4<f32>,
    // Opacity (packed into a vec4 for alignment; only .x used).
    @location(2) opacity: f32,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vi: u32,
    img: ImageData,
) -> VertexOutput {
    // 6-vertex unit quad (two triangles).
    var corners = array<vec2<f32>, 6>(
        vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0),
        vec2(0.0, 1.0), vec2(1.0, 0.0), vec2(1.0, 1.0),
    );
    let corner = corners[vi];

    // Screen-space pixel position.
    let px = img.rect.xy + corner * img.rect.zw;

    // Pixels → NDC.
    let ndc = vec2(
        (px.x / viewport.size.x) * 2.0 - 1.0,
        1.0 - (px.y / viewport.size.y) * 2.0,
    );

    // Interpolate UV across the image's atlas region.
    let uv = mix(img.uv_rect.xy, img.uv_rect.zw, corner);

    var out: VertexOutput;
    out.position = vec4(ndc, 0.0, 1.0);
    out.uv = uv;
    out.opacity = img.opacity;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(image_tex, image_sampler, in.uv);
    return vec4(tex_color.rgb, tex_color.a * in.opacity);
}
