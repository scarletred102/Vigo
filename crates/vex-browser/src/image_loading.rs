// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Async image loading pattern for browser pipeline integration.
//!
//! Pattern:
//! 1) fetch image bytes via `vex-net`
//! 2) decode on a background thread with `tokio::task::spawn_blocking`
//! 3) consume decoded frames on main/render path and upload into atlas

use tokio::task::JoinHandle;
use vex_render::display_list::ImageId;
use vex_render::image_decode::DecodedImage;

/// Result of a background image decode task.
#[derive(Debug)]
pub enum ImageLoadResult {
    Ready { id: ImageId, image: DecodedImage },
    Failed { id: ImageId, error: String },
}

/// Tracks pending image decode tasks and drains completed results.
#[derive(Default)]
pub struct AsyncImageLoader {
    pending: Vec<JoinHandle<ImageLoadResult>>,
}

impl AsyncImageLoader {
    /// Create an empty async loader queue.
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
        }
    }

    /// Queue a decode job for fetched image bytes.
    pub fn request_decode(&mut self, id: ImageId, bytes: Vec<u8>) {
        let task =
            tokio::task::spawn_blocking(move || {
                match vex_render::image_decode::decode_image(&bytes) {
                    Ok(image) => ImageLoadResult::Ready { id, image },
                    Err(error) => ImageLoadResult::Failed {
                        id,
                        error: error.to_string(),
                    },
                }
            });
        self.pending.push(task);
    }

    /// Number of decode tasks still pending completion.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Drain all finished decode tasks without blocking on unfinished ones.
    pub async fn drain_finished(&mut self) -> Vec<ImageLoadResult> {
        let mut finished = Vec::new();
        let mut i = 0;

        while i < self.pending.len() {
            if self.pending[i].is_finished() {
                let handle = self.pending.swap_remove(i);
                match handle.await {
                    Ok(result) => finished.push(result),
                    Err(error) => {
                        tracing::error!(
                            target: "vex_browser::image_loading",
                            "image decode task join error: {error}"
                        );
                    }
                }
            } else {
                i += 1;
            }
        }

        finished
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_png() -> Vec<u8> {
        // Minimal 1×1 PNG.
        vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ]
    }

    #[tokio::test]
    async fn decode_success_path() {
        let mut loader = AsyncImageLoader::new();
        loader.request_decode(ImageId(1), tiny_png());

        let mut result = Vec::new();
        for _ in 0..200 {
            result = loader.drain_finished().await;
            if !result.is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }

        assert_eq!(result.len(), 1);
        match &result[0] {
            ImageLoadResult::Ready { id, image } => {
                assert_eq!(*id, ImageId(1));
                assert_eq!(image.width, 1);
                assert_eq!(image.height, 1);
            }
            ImageLoadResult::Failed { error, .. } => panic!("unexpected decode failure: {error}"),
        }
    }

    #[tokio::test]
    async fn decode_failure_path() {
        let mut loader = AsyncImageLoader::new();
        loader.request_decode(ImageId(2), b"not-an-image".to_vec());

        let mut result = Vec::new();
        for _ in 0..200 {
            result = loader.drain_finished().await;
            if !result.is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }

        assert_eq!(result.len(), 1);
        match &result[0] {
            ImageLoadResult::Failed { id, .. } => assert_eq!(*id, ImageId(2)),
            ImageLoadResult::Ready { .. } => panic!("expected decode failure"),
        }
    }
}
