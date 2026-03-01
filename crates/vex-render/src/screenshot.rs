// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Offscreen rendering and PNG screenshot capture.
//!
//! Used for visual regression testing — render a display list to an offscreen
//! texture, read back the pixels, and save/compare as PNG.

use std::path::Path;

use crate::display_list::DisplayList;
use crate::renderer::Renderer;

/// Render a display list to an RGBA pixel buffer (offscreen).
///
/// Creates a headless wgpu device, renders the display list, and reads back
/// the result. Returns the RGBA pixels as a `Vec<u8>`.
///
/// # Panics
/// Panics if wgpu cannot find a suitable adapter (e.g. no GPU available).
pub fn render_to_pixels(
    dl: &DisplayList,
    width: u32,
    height: u32,
) -> Vec<u8> {
    pollster::block_on(render_to_pixels_async(dl, width, height))
}

async fn render_to_pixels_async(
    dl: &DisplayList,
    width: u32,
    height: u32,
) -> Vec<u8> {
    // Create a headless device (no surface needed).
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .expect("no GPU adapter for offscreen rendering");

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("vex-screenshot-device"),
            ..Default::default()
        }, None)
        .await
        .expect("failed to create device for screenshot");

    let format = wgpu::TextureFormat::Rgba8UnormSrgb;

    // Create offscreen render target.
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("screenshot-target"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Render.
    let mut renderer = Renderer::new(&device, &queue, format);
    renderer.set_clear_color(0.0, 0.0, 0.0);
    renderer.prepare(&device, &queue, dl, width as f32, height as f32);

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("screenshot-encoder"),
    });
    renderer.render(&mut encoder, &view);

    // Copy texture to a read-back buffer.
    // wgpu requires rows aligned to 256 bytes.
    let bytes_per_pixel = 4u32;
    let unpadded_row = width * bytes_per_pixel;
    let padded_row = (unpadded_row + 255) & !255;

    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("screenshot-readback"),
        size: (padded_row * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &readback,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );

    queue.submit(std::iter::once(encoder.finish()));

    // Map and read.
    let slice = readback.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        tx.send(result).ok();
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv()
        .expect("map_async channel closed")
        .expect("map_async failed");

    let mapped = slice.get_mapped_range();

    // Strip row padding.
    let mut pixels = Vec::with_capacity((width * height * bytes_per_pixel) as usize);
    for row in 0..height {
        let start = (row * padded_row) as usize;
        let end = start + unpadded_row as usize;
        pixels.extend_from_slice(&mapped[start..end]);
    }
    drop(mapped);
    readback.unmap();

    pixels
}

/// Render a display list and save the result as a PNG file.
///
/// Returns `Ok(())` on success, or an error string.
pub fn save_screenshot(
    dl: &DisplayList,
    width: u32,
    height: u32,
    path: &Path,
) -> Result<(), String> {
    let pixels = render_to_pixels(dl, width, height);
    let img = image::RgbaImage::from_raw(width, height, pixels)
        .ok_or_else(|| "pixel buffer size mismatch".to_string())?;
    img.save(path)
        .map_err(|e| format!("failed to write PNG: {e}"))
}

/// Compare two RGBA pixel buffers and return the fraction of pixels that differ
/// beyond the given threshold (per-channel).
///
/// Returns 0.0 if the images are identical, 1.0 if every pixel differs.
pub fn pixel_diff(a: &[u8], b: &[u8], threshold: u8) -> f64 {
    assert_eq!(a.len(), b.len(), "pixel buffers must be the same size");
    let total_pixels = a.len() / 4;
    if total_pixels == 0 {
        return 0.0;
    }

    let mut diff_count = 0usize;
    for (pa, pb) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        let dr = (pa[0] as i16 - pb[0] as i16).unsigned_abs() as u8;
        let dg = (pa[1] as i16 - pb[1] as i16).unsigned_abs() as u8;
        let db = (pa[2] as i16 - pb[2] as i16).unsigned_abs() as u8;
        let da = (pa[3] as i16 - pb[3] as i16).unsigned_abs() as u8;
        if dr > threshold || dg > threshold || db > threshold || da > threshold {
            diff_count += 1;
        }
    }

    diff_count as f64 / total_pixels as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_diff_identical() {
        let a = vec![255, 0, 0, 255, 0, 255, 0, 255];
        let b = vec![255, 0, 0, 255, 0, 255, 0, 255];
        assert_eq!(pixel_diff(&a, &b, 0), 0.0);
    }

    #[test]
    fn pixel_diff_all_different() {
        let a = vec![255, 0, 0, 255];
        let b = vec![0, 255, 0, 255];
        assert_eq!(pixel_diff(&a, &b, 0), 1.0);
    }

    #[test]
    fn pixel_diff_within_threshold() {
        let a = vec![100, 100, 100, 255];
        let b = vec![105, 100, 100, 255];
        // Difference of 5 per channel — within threshold of 10.
        assert_eq!(pixel_diff(&a, &b, 10), 0.0);
        // But not within threshold of 3.
        assert_eq!(pixel_diff(&a, &b, 3), 1.0);
    }

    #[test]
    fn pixel_diff_half_match() {
        let a = vec![0, 0, 0, 255, 255, 255, 255, 255];
        let b = vec![0, 0, 0, 255, 0, 0, 0, 255];
        assert_eq!(pixel_diff(&a, &b, 0), 0.5);
    }
}
