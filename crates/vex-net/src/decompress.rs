// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! HTTP content decompression (gzip, brotli, zstd).

use std::io::Read;

use vex_core::{VexError, VexResult};

/// Decompress an HTTP response body based on `Content-Encoding`.
///
/// Supported encodings: `gzip`, `br`, `zstd`, `identity` (passthrough).
pub fn decompress(encoding: &str, data: &[u8]) -> VexResult<Vec<u8>> {
    match encoding {
        "gzip" | "x-gzip" => decompress_gzip(data),
        "br" => decompress_brotli(data),
        "zstd" => decompress_zstd(data),
        "identity" | "" => Ok(data.to_vec()),
        other => Err(VexError::Network(format!(
            "unsupported Content-Encoding: {other}"
        ))),
    }
}

fn decompress_gzip(data: &[u8]) -> VexResult<Vec<u8>> {
    let mut decoder = flate2::read::GzDecoder::new(data);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|e| VexError::Network(format!("gzip decompression failed: {e}")))?;
    Ok(out)
}

fn decompress_brotli(data: &[u8]) -> VexResult<Vec<u8>> {
    let mut out = Vec::new();
    brotli::BrotliDecompress(&mut &data[..], &mut out)
        .map_err(|e| VexError::Network(format!("brotli decompression failed: {e}")))?;
    Ok(out)
}

fn decompress_zstd(data: &[u8]) -> VexResult<Vec<u8>> {
    let out = zstd::stream::decode_all(data)
        .map_err(|e| VexError::Network(format!("zstd decompression failed: {e}")))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn identity_passthrough() {
        let data = b"hello world";
        let result = decompress("identity", data).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn empty_encoding_passthrough() {
        let data = b"raw bytes";
        let result = decompress("", data).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn gzip_roundtrip() {
        let original = b"The quick brown fox jumps over the lazy dog";
        // Compress with flate2
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(original).unwrap();
        let compressed = encoder.finish().unwrap();

        let decompressed = decompress("gzip", &compressed).unwrap();
        assert_eq!(decompressed, original);
    }

    #[test]
    fn brotli_roundtrip() {
        let original = b"Brotli compression test data for Vex engine";
        let mut compressed = Vec::new();
        let mut params = brotli::enc::BrotliEncoderParams::default();
        params.quality = 4;
        brotli::BrotliCompress(&mut &original[..], &mut compressed, &params).unwrap();

        let decompressed = decompress("br", &compressed).unwrap();
        assert_eq!(decompressed, original);
    }

    #[test]
    fn zstd_roundtrip() {
        let original = b"Zstandard compression test for network responses";
        let compressed = zstd::stream::encode_all(&original[..], 3).unwrap();

        let decompressed = decompress("zstd", &compressed).unwrap();
        assert_eq!(decompressed, original);
    }

    #[test]
    fn unsupported_encoding_errors() {
        let result = decompress("deflate-raw", b"data");
        assert!(result.is_err());
    }
}
