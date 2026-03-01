// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! GPU renderer — consumes a display list and renders via wgpu.
//!
//! Supports:
//! - `FillRect` / `DrawBorder` — solid-color rectangles (instanced).
//! - `DrawText` — alpha-tested glyph rendering from a glyph atlas.

use wgpu::util::DeviceExt;

use crate::display_list::{DisplayCommand, DisplayList, RenderBorderStyle};
use crate::glyph_atlas::GlyphAtlas;

/// Maximum rectangles per frame before the instance buffer grows.
const INITIAL_MAX_RECTS: usize = 4096;
/// Maximum glyph instances per frame before the instance buffer grows.
const INITIAL_MAX_GLYPHS: usize = 8192;

/// Instance data for one rectangle: [x, y, w, h, r, g, b, a].
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct RectInstance {
    rect: [f32; 4],  // x, y, width, height
    color: [f32; 4], // r, g, b, a
}

/// Instance data for one glyph: rect + atlas UV + color.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct TextInstance {
    rect: [f32; 4],  // x, y, width, height
    uv: [f32; 4],    // u0, v0, u1, v1
    color: [f32; 4], // r, g, b, a
}

/// Viewport uniform: [width, height, padding, padding].
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ViewportUniform {
    size: [f32; 2],
    _pad: [f32; 2],
}

/// GPU renderer for the Vex engine.
///
/// Call [`prepare`](Renderer::prepare) once per frame to upload display list data,
/// then [`render`](Renderer::render) to execute the draw calls.
pub struct Renderer {
    // ── Shared ──
    viewport_buffer: wgpu::Buffer,
    viewport_bind_group: wgpu::BindGroup,
    clear_color: wgpu::Color,

    // ── Rect pipeline ──
    rect_pipeline: wgpu::RenderPipeline,
    rect_buffer: wgpu::Buffer,
    rect_capacity: usize,
    rect_count: u32,

    // ── Text pipeline ──
    text_pipeline: wgpu::RenderPipeline,
    text_buffer: wgpu::Buffer,
    text_capacity: usize,
    text_count: u32,
    glyph_atlas: GlyphAtlas,
    atlas_texture: wgpu::Texture,
    atlas_bind_group: wgpu::BindGroup,
}

impl Renderer {
    /// Create a new renderer for the given surface format.
    ///
    /// Loads system fonts and creates the glyph atlas GPU texture.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        // ── Viewport uniform (shared bind group 0) ──
        let viewport_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport-uniform"),
            contents: bytemuck::cast_slice(&[ViewportUniform {
                size: [800.0, 600.0],
                _pad: [0.0; 2],
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let viewport_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("viewport-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let viewport_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport-bg"),
            layout: &viewport_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: viewport_buffer.as_entire_binding(),
            }],
        });

        // ── Rect pipeline ──
        let rect_pipeline = create_rect_pipeline(device, &viewport_bgl, format);
        let rect_buffer = create_instance_buffer(device, "rect-instances", INITIAL_MAX_RECTS, std::mem::size_of::<RectInstance>());

        // ── Glyph atlas + text pipeline ──
        let glyph_atlas = GlyphAtlas::new();
        let (aw, ah) = glyph_atlas.dimensions();

        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyph-atlas"),
            size: wgpu::Extent3d { width: aw, height: ah, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let atlas_view = atlas_texture.create_view(&Default::default());
        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("glyph-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let texture_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let atlas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("glyph-atlas-bg"),
            layout: &texture_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&atlas_sampler),
                },
            ],
        });

        let text_pipeline = create_text_pipeline(device, &viewport_bgl, &texture_bgl, format);
        let text_buffer = create_instance_buffer(device, "text-instances", INITIAL_MAX_GLYPHS, std::mem::size_of::<TextInstance>());

        // Upload initial (blank) glyph atlas texture.
        upload_atlas_texture(queue, &atlas_texture, &glyph_atlas);

        Self {
            viewport_buffer,
            viewport_bind_group,
            clear_color: wgpu::Color { r: 0.08, g: 0.08, b: 0.12, a: 1.0 },
            rect_pipeline,
            rect_buffer,
            rect_capacity: INITIAL_MAX_RECTS,
            rect_count: 0,
            text_pipeline,
            text_buffer,
            text_capacity: INITIAL_MAX_GLYPHS,
            text_count: 0,
            glyph_atlas,
            atlas_texture,
            atlas_bind_group,
        }
    }

    /// Set the background clear color.
    pub fn set_clear_color(&mut self, r: f64, g: f64, b: f64) {
        self.clear_color = wgpu::Color { r, g, b, a: 1.0 };
    }

    /// Upload display list data to GPU buffers.
    ///
    /// Call once per frame before [`render`](Renderer::render).
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        dl: &DisplayList,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        // Update viewport uniform.
        queue.write_buffer(
            &self.viewport_buffer,
            0,
            bytemuck::cast_slice(&[ViewportUniform {
                size: [viewport_width, viewport_height],
                _pad: [0.0; 2],
            }]),
        );

        // ── Rect instances ──
        let rect_instances = collect_rect_instances(dl);
        self.rect_count = rect_instances.len() as u32;
        upload_instances(device, queue, &rect_instances, &mut self.rect_buffer, &mut self.rect_capacity, "rect-instances");

        // ── Text instances — shape each DrawText through the glyph atlas ──
        self.glyph_atlas.begin_frame();
        let mut text_instances: Vec<TextInstance> = Vec::new();

        for cmd in dl.commands() {
            if let DisplayCommand::DrawText {
                position,
                text,
                color,
                font_size,
                line_height,
            } = cmd
            {
                let c = color.to_f32_array();
                let positioned = self.glyph_atlas.prepare_text(
                    text,
                    position.x,
                    position.y,
                    *font_size,
                    *line_height,
                    c,
                );
                for pg in &positioned {
                    text_instances.push(TextInstance {
                        rect: pg.rect,
                        uv: pg.uv,
                        color: pg.color,
                    });
                }
            }
        }
        self.text_count = text_instances.len() as u32;
        upload_instances(device, queue, &text_instances, &mut self.text_buffer, &mut self.text_capacity, "text-instances");

        // Re-upload glyph atlas if new glyphs were rasterized.
        if self.glyph_atlas.is_dirty() {
            upload_atlas_texture(queue, &self.atlas_texture, &self.glyph_atlas);
            self.glyph_atlas.mark_clean();
        }

        // Periodic LRU eviction (~every 5 seconds at 60fps).
        if self.glyph_atlas.usage_fraction() > 0.9 {
            self.glyph_atlas.evict_stale(300);
        }
    }

    /// Record render commands into the encoder.
    ///
    /// Creates a render pass, clears to the background color, draws
    /// rectangles, then text glyphs.
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("vex-render"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.clear_color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });

        // Rects first (backgrounds, borders).
        if self.rect_count > 0 {
            pass.set_pipeline(&self.rect_pipeline);
            pass.set_bind_group(0, &self.viewport_bind_group, &[]);
            pass.set_vertex_buffer(0, self.rect_buffer.slice(..));
            pass.draw(0..6, 0..self.rect_count);
        }

        // Text on top.
        if self.text_count > 0 {
            pass.set_pipeline(&self.text_pipeline);
            pass.set_bind_group(0, &self.viewport_bind_group, &[]);
            pass.set_bind_group(1, &self.atlas_bind_group, &[]);
            pass.set_vertex_buffer(0, self.text_buffer.slice(..));
            pass.draw(0..6, 0..self.text_count);
        }
    }
}

// ── Pipeline creation helpers ──────────────────────────────────────

fn create_rect_pipeline(
    device: &wgpu::Device,
    viewport_bgl: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("rect-shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/rect.wgsl").into()),
    });

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("rect-pipeline-layout"),
        bind_group_layouts: &[viewport_bgl],
        push_constant_ranges: &[],
    });

    let instance_layout = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<RectInstance>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &[
            wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x4 },
            wgpu::VertexAttribute { offset: 16, shader_location: 1, format: wgpu::VertexFormat::Float32x4 },
        ],
    };

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("rect-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[instance_layout],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

fn create_text_pipeline(
    device: &wgpu::Device,
    viewport_bgl: &wgpu::BindGroupLayout,
    texture_bgl: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("text-shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/text.wgsl").into()),
    });

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("text-pipeline-layout"),
        bind_group_layouts: &[viewport_bgl, texture_bgl],
        push_constant_ranges: &[],
    });

    let instance_layout = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<TextInstance>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &[
            wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x4 },
            wgpu::VertexAttribute { offset: 16, shader_location: 1, format: wgpu::VertexFormat::Float32x4 },
            wgpu::VertexAttribute { offset: 32, shader_location: 2, format: wgpu::VertexFormat::Float32x4 },
        ],
    };

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("text-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[instance_layout],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

// ── Buffer / upload helpers ────────────────────────────────────────

fn create_instance_buffer(
    device: &wgpu::Device,
    label: &str,
    capacity: usize,
    stride: usize,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: (capacity * stride) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn upload_instances<T: bytemuck::Pod>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    data: &[T],
    buffer: &mut wgpu::Buffer,
    capacity: &mut usize,
    label: &str,
) {
    if data.is_empty() {
        return;
    }
    if data.len() > *capacity {
        let new_cap = data.len().next_power_of_two();
        *buffer = create_instance_buffer(device, label, new_cap, std::mem::size_of::<T>());
        *capacity = new_cap;
    }
    queue.write_buffer(buffer, 0, bytemuck::cast_slice(data));
}

fn upload_atlas_texture(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    atlas: &GlyphAtlas,
) {
    let (w, h) = atlas.dimensions();
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        atlas.pixels(),
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(w),
            rows_per_image: Some(h),
        },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
}

/// Extract `RectInstance` data from all `FillRect` and `DrawBorder` commands.
fn collect_rect_instances(dl: &DisplayList) -> Vec<RectInstance> {
    let mut instances = Vec::with_capacity(dl.len());

    for cmd in dl.commands() {
        match cmd {
            DisplayCommand::FillRect { rect, color } => {
                instances.push(RectInstance {
                    rect: [rect.origin.x, rect.origin.y, rect.size.width, rect.size.height],
                    color: color.to_f32_array(),
                });
            }
            DisplayCommand::DrawBorder {
                rect,
                widths,
                colors,
                styles,
            } => {
                // Decompose border into up to 4 thin rectangles.
                emit_border_rects(rect, widths, colors, styles, &mut instances);
            }
            // DrawText/DrawImage handled by separate pipelines.
            _ => {}
        }
    }

    instances
}

/// Emit up to 4 rectangle instances for a border.
fn emit_border_rects(
    rect: &vex_core::geometry::Rect,
    widths: &vex_core::geometry::Insets,
    colors: &[vex_core::color::Color; 4],
    styles: &[RenderBorderStyle; 4],
    out: &mut Vec<RectInstance>,
) {
    let x = rect.origin.x;
    let y = rect.origin.y;
    let w = rect.size.width;
    let h = rect.size.height;

    // Top border.
    if widths.top > 0.0 && styles[0] != RenderBorderStyle::None {
        out.push(RectInstance {
            rect: [x, y, w, widths.top],
            color: colors[0].to_f32_array(),
        });
    }
    // Right border.
    if widths.right > 0.0 && styles[1] != RenderBorderStyle::None {
        out.push(RectInstance {
            rect: [x + w - widths.right, y, widths.right, h],
            color: colors[1].to_f32_array(),
        });
    }
    // Bottom border.
    if widths.bottom > 0.0 && styles[2] != RenderBorderStyle::None {
        out.push(RectInstance {
            rect: [x, y + h - widths.bottom, w, widths.bottom],
            color: colors[2].to_f32_array(),
        });
    }
    // Left border.
    if widths.left > 0.0 && styles[3] != RenderBorderStyle::None {
        out.push(RectInstance {
            rect: [x, y, widths.left, h],
            color: colors[3].to_f32_array(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display_list::DisplayList;
    use vex_core::color::Color;
    use vex_core::geometry::{Insets, Rect};

    #[test]
    fn collect_fill_rects() {
        let mut dl = DisplayList::new();
        dl.push(DisplayCommand::FillRect {
            rect: Rect::new(10.0, 20.0, 100.0, 50.0),
            color: Color::rgb(255, 0, 0),
        });
        dl.push(DisplayCommand::FillRect {
            rect: Rect::new(50.0, 50.0, 200.0, 100.0),
            color: Color::rgba(0, 255, 0, 128),
        });

        let instances = collect_rect_instances(&dl);
        assert_eq!(instances.len(), 2);

        // First rect.
        assert_eq!(instances[0].rect, [10.0, 20.0, 100.0, 50.0]);
        assert!((instances[0].color[0] - 1.0).abs() < 0.01); // red = 255 → ~1.0

        // Second rect — alpha.
        assert!((instances[1].color[3] - 0.502).abs() < 0.01); // 128/255 ≈ 0.502
    }

    #[test]
    fn collect_border_as_rects() {
        let mut dl = DisplayList::new();
        dl.push(DisplayCommand::DrawBorder {
            rect: Rect::new(0.0, 0.0, 100.0, 80.0),
            widths: Insets::new(2.0, 2.0, 2.0, 2.0),
            colors: [Color::BLACK; 4],
            styles: [RenderBorderStyle::Solid; 4],
        });

        let instances = collect_rect_instances(&dl);
        assert_eq!(instances.len(), 4, "4 border sides → 4 rects");
    }

    #[test]
    fn skips_none_border_style() {
        let mut dl = DisplayList::new();
        dl.push(DisplayCommand::DrawBorder {
            rect: Rect::new(0.0, 0.0, 100.0, 80.0),
            widths: Insets::new(2.0, 0.0, 2.0, 0.0),
            colors: [Color::BLACK; 4],
            styles: [
                RenderBorderStyle::Solid,
                RenderBorderStyle::None,
                RenderBorderStyle::Solid,
                RenderBorderStyle::None,
            ],
        });

        let instances = collect_rect_instances(&dl);
        assert_eq!(instances.len(), 2, "only top and bottom borders");
    }

    #[test]
    fn empty_display_list() {
        let dl = DisplayList::new();
        let instances = collect_rect_instances(&dl);
        assert!(instances.is_empty());
    }

    #[test]
    fn non_rect_commands_are_skipped() {
        let mut dl = DisplayList::new();
        dl.push(DisplayCommand::PushOpacity { opacity: 0.5 });
        dl.push(DisplayCommand::PopOpacity);
        dl.push(DisplayCommand::PushClip {
            rect: Rect::new(0.0, 0.0, 100.0, 100.0),
        });
        dl.push(DisplayCommand::PopClip);

        let instances = collect_rect_instances(&dl);
        assert!(instances.is_empty(), "non-rect commands produce no instances");
    }
}
