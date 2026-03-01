// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! GPU context: wgpu instance, device, queue, and surface management.

#[cfg(target_os = "windows")]
use vex_core::{VexError, VexResult};

#[cfg(target_os = "windows")]
use crate::platform::Window;

/// Holds all GPU state needed for rendering.
#[cfg(target_os = "windows")]
pub struct GpuContext<'w> {
    pub surface: wgpu::Surface<'w>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
}

#[cfg(target_os = "windows")]
impl<'w> GpuContext<'w> {
    /// Initialise the GPU pipeline from an existing window.
    ///
    /// This is synchronous (blocks via `pollster`).
    pub fn new(window: &'w Window) -> VexResult<Self> {
        pollster::block_on(Self::init_async(window))
    }

    async fn init_async(window: &'w Window) -> VexResult<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        // SAFETY: The window outlives the surface thanks to the lifetime
        // bound on GpuContext<'w>.
        let surface = instance
            .create_surface(window)
            .map_err(|e| VexError::Platform(format!("create surface: {e}")))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| VexError::Platform("no suitable GPU adapter".into()))?;

        tracing::info!("GPU adapter: {}", adapter.get_info().name);

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("vex-device"),
                    ..Default::default()
                },
                None,
            )
            .await
            .map_err(|e| VexError::Platform(format!("request device: {e}")))?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: window.width(),
            height: window.height(),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        tracing::info!("GPU initialised — format: {format:?}");

        Ok(Self {
            surface,
            device,
            queue,
            config,
        })
    }

    /// Resize the surface (call after `WindowResize` events).
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    /// Render a single frame: clear to the given RGBA colour.
    pub fn render_clear(&self, r: f64, g: f64, b: f64) -> VexResult<()> {
        let output = self
            .surface
            .get_current_texture()
            .map_err(|e| VexError::Platform(format!("get texture: {e}")))?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("vex-clear"),
            });

        // A render pass that just clears to the given colour.
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
