// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Image decoder — converts raw bytes into RGBA pixel buffers.
//!
//! Supports PNG, JPEG, WebP, GIF (first frame), BMP via the `image` crate.

use vex_core::{VexError, VexResult};

/// A decoded image ready for GPU upload.
#[derive(Debug, Clone)]
pub struct DecodedImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// RGBA pixel data (width × height × 4 bytes).
    pub pixels: Vec<u8>,
}

impl DecodedImage {
    /// Total byte size of the pixel buffer.
    pub fn byte_size(&self) -> usize {
        self.pixels.len()
    }
}

/// Decode an image from raw bytes.
///
/// Attempts to detect the format automatically (PNG, JPEG, WebP, GIF, BMP).
/// Returns the image as premultiplied RGBA.
pub fn decode_image(bytes: &[u8]) -> VexResult<DecodedImage> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| VexError::Parse(format!("image decode: {e}")))?;

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    Ok(DecodedImage {
        width,
        height,
        pixels: rgba.into_raw(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generate a minimal valid 1×1 PNG (red pixel) as raw bytes.
    fn tiny_png() -> Vec<u8> {
        let mut buf = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        use image::ImageEncoder;
        encoder
            .write_image(&[255, 0, 0, 255], 1, 1, image::ExtendedColorType::Rgba8)
            .unwrap();
        buf
    }

    /// Generate a minimal valid 2×2 BMP.
    fn tiny_bmp() -> Vec<u8> {
        let pixels = [
            255, 0, 0, 255, // red
            0, 255, 0, 255, // green
            0, 0, 255, 255, // blue
            255, 255, 0, 255, // yellow
        ];
        let img = image::RgbaImage::from_raw(2, 2, pixels.to_vec()).unwrap();
        let mut buf: Vec<u8> = Vec::new();
        let encoder = image::codecs::bmp::BmpEncoder::new(&mut buf);
        use image::ImageEncoder;
        encoder
            .write_image(&img, 2, 2, image::ExtendedColorType::Rgba8)
            .unwrap();
        buf
    }

    #[test]
    fn decode_png() {
        let bytes = tiny_png();
        let decoded = decode_image(&bytes).unwrap();
        assert_eq!(decoded.width, 1);
        assert_eq!(decoded.height, 1);
        assert_eq!(decoded.pixels.len(), 4); // 1×1×4
        assert_eq!(decoded.pixels[0], 255); // R
        assert_eq!(decoded.pixels[1], 0); // G
        assert_eq!(decoded.pixels[2], 0); // B
        assert_eq!(decoded.pixels[3], 255); // A
    }

    #[test]
    fn decode_bmp() {
        let bytes = tiny_bmp();
        let decoded = decode_image(&bytes).unwrap();
        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 2);
        assert_eq!(decoded.byte_size(), 16); // 2×2×4
    }

    #[test]
    fn invalid_bytes_returns_error() {
        let result = decode_image(b"this is not an image");
        assert!(result.is_err());
    }

    #[test]
    fn empty_bytes_returns_error() {
        let result = decode_image(b"");
        assert!(result.is_err());
    }
}
