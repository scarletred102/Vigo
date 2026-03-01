// Rect shader — solid color rectangle rendering.
// Each instance is one FillRect command from the display list.

struct Viewport {
    size: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> viewport: Viewport;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

// Instance data per rectangle: [x, y, w, h, r, g, b, a].
// Packed as two vec4<f32> for alignment.
struct RectInstance {
    @location(0) rect: vec4<f32>,   // x, y, width, height (pixels)
    @location(1) color: vec4<f32>,  // r, g, b, a (0..1)
};

@vertex
fn vs_main(
    @builtin(vertex_index) vi: u32,
    instance: RectInstance,
) -> VertexOutput {
    // Generate a unit quad from vertex index (two triangles, 6 verts).
    // 0: (0,0)  1: (1,0)  2: (0,1)
    // 3: (0,1)  4: (1,0)  5: (1,1)
    var corners = array<vec2<f32>, 6>(
        vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0),
        vec2(0.0, 1.0), vec2(1.0, 0.0), vec2(1.0, 1.0),
    );

    let corner = corners[vi];

    // Scale to pixel rect.
    let px = instance.rect.xy + corner * instance.rect.zw;

    // Convert pixels → NDC: x ∈ [0, width] → [-1, 1], y ∈ [0, height] → [1, -1].
    let ndc = vec2(
        (px.x / viewport.size.x) * 2.0 - 1.0,
        1.0 - (px.y / viewport.size.y) * 2.0,
    );

    var out: VertexOutput;
    out.position = vec4(ndc, 0.0, 1.0);
    out.color = instance.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
