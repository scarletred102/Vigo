// Rect shader — solid color rectangle rendering with optional rounded corners.
// Each instance is one FillRect command from the display list.

struct Viewport {
    size: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> viewport: Viewport;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) local_pos: vec2<f32>,  // position within rect (0..w, 0..h)
    @location(2) rect_size: vec2<f32>,  // width, height of the rect
    @location(3) border_radius: f32,    // corner radius in pixels
};

// Instance data per rectangle: [x, y, w, h], [r, g, b, a], [border_radius, pad, pad, pad].
struct RectInstance {
    @location(0) rect: vec4<f32>,   // x, y, width, height (pixels)
    @location(1) color: vec4<f32>,  // r, g, b, a (0..1)
    @location(2) extra: vec4<f32>,  // border_radius, _pad, _pad, _pad
};

@vertex
fn vs_main(
    @builtin(vertex_index) vi: u32,
    instance: RectInstance,
) -> VertexOutput {
    // Generate a unit quad from vertex index (two triangles, 6 verts).
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
    out.local_pos = corner * instance.rect.zw;
    out.rect_size = instance.rect.zw;
    out.border_radius = instance.extra.x;
    return out;
}

/// Signed distance from point `p` to a rounded rectangle centered at origin
/// with half-extents `b` and corner radius `r`.
fn sdf_rounded_rect(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b + vec2(r, r);
    return length(max(q, vec2(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let radius = in.border_radius;

    // Fast path: no border radius → solid fill.
    if radius <= 0.0 {
        return in.color;
    }

    // Clamp radius to half the smallest dimension.
    let r = min(radius, min(in.rect_size.x, in.rect_size.y) * 0.5);

    // Map local_pos to centered coordinates for SDF evaluation.
    let half = in.rect_size * 0.5;
    let p = in.local_pos - half;

    let d = sdf_rounded_rect(p, half, r);

    // Anti-alias: smooth 1px edge.
    let alpha = 1.0 - smoothstep(-0.5, 0.5, d);

    return vec4(in.color.rgb, in.color.a * alpha);
}
