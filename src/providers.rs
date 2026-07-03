// Compression algorithm providers
//
// These are built-in implementations using Rust crates.
// In the future, these will be replaced with WIT imports from Layer 1 components.

use std::io::{Read, Write};

/// Compression algorithm selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Algorithm {
    /// No compression (pass-through)
    Store,
    /// DEFLATE compression (RFC 1951)
    Deflate,
    /// BZIP2 compression
    Bzip2,
    /// LZMA compression
    Lzma,
    /// Zstandard compression
    Zstd,
    /// LZ4 compression (extremely fast)
    Lz4,
    /// OpenZL compression (Meta's format-aware compression)
    Openzl,
}

/// Compression provider trait
///
/// Each algorithm implements this trait to provide compress/decompress functionality
pub trait CompressionProvider {
    fn compress(&self, input: &[u8], level: u8) -> Result<Vec<u8>, String>;
    fn decompress(&self, input: &[u8]) -> Result<Vec<u8>, String>;

    /// Decompress AND report how many input bytes the stream actually
    /// consumed. Default impl claims all input as consumed — providers
    /// whose underlying library exposes a "bytes read" counter (e.g.,
    /// flate2's `DeflateDecoder::total_in`) should override to report
    /// the true stream end so callers can recover trailing bytes
    /// (gzip trailer, next member, etc.) at the right offset.
    fn decompress_counted(&self, input: &[u8]) -> Result<(Vec<u8>, u64), String> {
        let out = self.decompress(input)?;
        Ok((out, input.len() as u64))
    }
}

/// Store provider (no compression, pass-through)
pub struct StoreProvider;

impl CompressionProvider for StoreProvider {
    fn compress(&self, input: &[u8], _level: u8) -> Result<Vec<u8>, String> {
        Ok(input.to_vec())
    }

    fn decompress(&self, input: &[u8]) -> Result<Vec<u8>, String> {
        Ok(input.to_vec())
    }
}

/// DEFLATE provider (RFC 1951)
pub struct DeflateProvider;

impl CompressionProvider for DeflateProvider {
    fn compress(&self, input: &[u8], level: u8) -> Result<Vec<u8>, String> {
        use flate2::write::DeflateEncoder;
        use flate2::Compression;

        let level = level.min(9);
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(level as u32));
        encoder
            .write_all(input)
            .map_err(|e| format!("DEFLATE compression failed: {}", e))?;
        encoder
            .finish()
            .map_err(|e| format!("DEFLATE finish failed: {}", e))
    }

    fn decompress(&self, input: &[u8]) -> Result<Vec<u8>, String> {
        use flate2::read::DeflateDecoder;

        let mut decoder = DeflateDecoder::new(input);
        let mut output = Vec::new();
        decoder
            .read_to_end(&mut output)
            .map_err(|e| format!("DEFLATE decompression failed: {}", e))?;
        Ok(output)
    }

    fn decompress_counted(&self, input: &[u8]) -> Result<(Vec<u8>, u64), String> {
        // flate2's DeflateDecoder exposes `total_in` post-read — the exact
        // number of input bytes consumed by the deflate stream. That's
        // what gzip's chunked-transfer parser needs to locate the
        // 8-byte CRC32+length trailer.
        use flate2::read::DeflateDecoder;

        let mut decoder = DeflateDecoder::new(input);
        let mut output = Vec::new();
        decoder
            .read_to_end(&mut output)
            .map_err(|e| format!("DEFLATE decompression failed: {}", e))?;
        let consumed = decoder.total_in();
        Ok((output, consumed))
    }
}

/// BZIP2 provider
pub struct Bzip2Provider;

impl CompressionProvider for Bzip2Provider {
    fn compress(&self, input: &[u8], level: u8) -> Result<Vec<u8>, String> {
        use banzai::encode;
        use std::io::BufWriter;

        let level = if level == 0 { 1 } else { level.min(9) };
        let mut output = Vec::new();
        let writer = BufWriter::new(&mut output);

        encode(input, writer, level as usize)
            .map_err(|e| format!("BZIP2 compression failed: {}", e))?;

        Ok(output)
    }

    fn decompress(&self, input: &[u8]) -> Result<Vec<u8>, String> {
        use bzip2_rs::DecoderReader;

        let mut decoder = DecoderReader::new(input);
        let mut output = Vec::new();

        decoder
            .read_to_end(&mut output)
            .map_err(|e| format!("BZIP2 decompression failed: {}", e))?;

        Ok(output)
    }
}

/// LZMA provider.
///
/// Output container is `.xz` (RFC-7878 xz format) so the bytes interoperate
/// with `xz`/`tar -J`/Python stdlib `lzma.FORMAT_XZ` and `.tar.xz` tarballs.
/// Previously this used `LzmaWriter::new_use_header` which produces the
/// legacy `.lzma` "alone" container; that's nearly extinct in the modern
/// ecosystem (pip wheel source dists, system tarballs, GitHub release
/// assets are all `.xz`), so the shape change is a net improvement.
/// Consumers get the LZMA codec for the price of an industry-standard
/// container.
pub struct LzmaProvider;

impl CompressionProvider for LzmaProvider {
    fn compress(&self, input: &[u8], level: u8) -> Result<Vec<u8>, String> {
        use lzma_rust2::{XzOptions, XzWriter};

        let level = level.min(9);
        let options = XzOptions::with_preset(level as u32);

        let mut encoder = XzWriter::new(Vec::new(), options)
            .map_err(|e| format!("XZ encoder creation failed: {}", e))?;

        encoder
            .write_all(input)
            .map_err(|e| format!("XZ compression failed: {}", e))?;

        encoder
            .finish()
            .map_err(|e| format!("XZ finish failed: {}", e))
    }

    fn decompress(&self, input: &[u8]) -> Result<Vec<u8>, String> {
        use lzma_rust2::XzReader;

        // allow_multiple_streams=true matches stdlib lzma's FORMAT_AUTO
        // semantic (xz files may contain concatenated streams; .tar.xz
        // tarballs sometimes do).
        let mut decoder = XzReader::new(input, true);
        let mut output = Vec::new();

        decoder
            .read_to_end(&mut output)
            .map_err(|e| format!("XZ decompression failed: {}", e))?;

        Ok(output)
    }
}

/// LZ4 provider (extremely fast compression)
pub struct Lz4Provider;

impl CompressionProvider for Lz4Provider {
    fn compress(&self, input: &[u8], _level: u8) -> Result<Vec<u8>, String> {
        // LZ4 doesn't use compression levels
        // Prepend the uncompressed size for decompression
        let mut result = Vec::with_capacity(4 + lz4_flex::block::get_maximum_output_size(input.len()));
        let size_bytes = (input.len() as u32).to_le_bytes();
        result.extend_from_slice(&size_bytes);

        let compressed = lz4_flex::compress(input);
        result.extend_from_slice(&compressed);

        Ok(result)
    }

    fn decompress(&self, input: &[u8]) -> Result<Vec<u8>, String> {
        if input.len() < 4 {
            return Err("LZ4 decompression failed: input too short".to_string());
        }

        // Read the prepended size
        let mut size_bytes = [0u8; 4];
        size_bytes.copy_from_slice(&input[0..4]);
        let uncompressed_size = u32::from_le_bytes(size_bytes) as usize;

        lz4_flex::decompress(&input[4..], uncompressed_size)
            .map_err(|e| format!("LZ4 decompression failed: {}", e))
    }
}

/// Zstd provider (Zstandard compression). Gated behind the `zstd` feature so
/// consumers that don't need zstd (the `compress` extension) don't link
/// libzstd; `get_provider(Zstd)` returns an error when the feature is off.
#[cfg(feature = "zstd")]
pub struct ZstdProvider;

#[cfg(feature = "zstd")]
impl CompressionProvider for ZstdProvider {
    fn compress(&self, input: &[u8], level: u8) -> Result<Vec<u8>, String> {
        // Zstd supports levels 1-22, map 0-9 to 1-19
        let zstd_level = if level == 0 { 1 } else { (level as i32 * 2).min(19) };

        zstd::encode_all(input, zstd_level)
            .map_err(|e| format!("Zstd compression failed: {}", e))
    }

    fn decompress(&self, input: &[u8]) -> Result<Vec<u8>, String> {
        zstd::decode_all(input)
            .map_err(|e| format!("Zstd decompression failed: {}", e))
    }
}

/// OpenZL provider (Meta's format-aware compression)
pub struct OpenZlProvider;

impl CompressionProvider for OpenZlProvider {
    fn compress(&self, input: &[u8], level: u8) -> Result<Vec<u8>, String> {
        use crate::openzl_ffi::{
            zl_compress_bound, get_error_name, ZlCParam, ZlReport, ZL_MAX_FORMAT_VERSION,
            ZL_CCtx_compress, ZL_CCtx_create, ZL_CCtx_free, ZL_CCtx_setParameter,
        };

        if input.is_empty() {
            return Ok(Vec::new());
        }

        unsafe {
            let cctx = ZL_CCtx_create();
            if cctx.is_null() {
                return Err("Failed to create OpenZL compression context".to_string());
            }

            // Set format version (required)
            let mut result = ZlReport { code: 0, value: 0 };
            ZL_CCtx_setParameter(&mut result, cctx, ZlCParam::FormatVersion, ZL_MAX_FORMAT_VERSION);

            // Set compression level
            let level = level.clamp(0, 9) as i32;
            ZL_CCtx_setParameter(&mut result, cctx, ZlCParam::CompressionLevel, level);

            // Calculate max output size and compress
            let max_size = zl_compress_bound(input.len());
            let mut output = vec![0u8; max_size];

            ZL_CCtx_compress(
                &mut result,
                cctx,
                output.as_mut_ptr() as *mut std::ffi::c_void,
                output.len(),
                input.as_ptr() as *const std::ffi::c_void,
                input.len(),
            );

            ZL_CCtx_free(cctx);

            if result.is_error() {
                return Err(format!("OpenZL compression failed: {}", get_error_name(result.code)));
            }

            output.truncate(result.get_value());
            Ok(output)
        }
    }

    fn decompress(&self, input: &[u8]) -> Result<Vec<u8>, String> {
        use crate::openzl_ffi::{
            get_error_name, ZlReport, ZL_DCtx_create, ZL_DCtx_decompress, ZL_DCtx_free,
            ZL_getDecompressedSize,
        };

        if input.is_empty() {
            return Ok(Vec::new());
        }

        unsafe {
            // Get decompressed size
            let mut size_result = ZlReport { code: 0, value: 0 };
            ZL_getDecompressedSize(
                &mut size_result,
                input.as_ptr() as *const std::ffi::c_void,
                input.len(),
            );

            if size_result.is_error() {
                return Err(format!(
                    "Failed to get OpenZL decompressed size: {}",
                    get_error_name(size_result.code)
                ));
            }

            let decompressed_size = size_result.get_value();
            let dctx = ZL_DCtx_create();
            if dctx.is_null() {
                return Err("Failed to create OpenZL decompression context".to_string());
            }

            let mut output = vec![0u8; decompressed_size];

            let mut result = ZlReport { code: 0, value: 0 };
            ZL_DCtx_decompress(
                &mut result,
                dctx,
                output.as_mut_ptr() as *mut std::ffi::c_void,
                output.len(),
                input.as_ptr() as *const std::ffi::c_void,
                input.len(),
            );

            ZL_DCtx_free(dctx);

            if result.is_error() {
                return Err(format!("OpenZL decompression failed: {}", get_error_name(result.code)));
            }

            output.truncate(result.get_value());
            Ok(output)
        }
    }
}

/// Get provider for algorithm
pub fn get_provider(algorithm: Algorithm) -> Result<Box<dyn CompressionProvider>, String> {
    match algorithm {
        Algorithm::Store => Ok(Box::new(StoreProvider)),
        Algorithm::Deflate => Ok(Box::new(DeflateProvider)),
        Algorithm::Bzip2 => Ok(Box::new(Bzip2Provider)),
        Algorithm::Lzma => Ok(Box::new(LzmaProvider)),
        #[cfg(feature = "zstd")]
        Algorithm::Zstd => Ok(Box::new(ZstdProvider)),
        #[cfg(not(feature = "zstd"))]
        Algorithm::Zstd => {
            Err("Zstandard support not compiled in (enable the `zstd` feature)".to_string())
        }
        Algorithm::Lz4 => Ok(Box::new(Lz4Provider)),
        #[cfg(target_family = "wasm")]
        Algorithm::Openzl => Ok(Box::new(OpenZlProvider)),
        #[cfg(not(target_family = "wasm"))]
        Algorithm::Openzl => Err("OpenZL is only available on WASM targets".to_string()),
    }
}

/// Get algorithm information
pub fn algorithm_description(algorithm: Algorithm) -> Option<String> {
    match algorithm {
        Algorithm::Store => Some("Store: No compression (pass-through)".to_string()),
        Algorithm::Deflate => Some("DEFLATE: Fast general-purpose compression (RFC 1951), used in ZIP, GZIP, PNG".to_string()),
        Algorithm::Bzip2 => Some("BZIP2: High compression ratio, good for repetitive data".to_string()),
        Algorithm::Lzma => Some("LZMA: Excellent compression ratio, used in 7-Zip and XZ".to_string()),
        Algorithm::Zstd => Some("Zstandard: Modern fast compression with excellent ratio, used in Linux kernel and databases".to_string()),
        Algorithm::Lz4 => Some("LZ4: Extremely fast compression and decompression, ideal for real-time applications".to_string()),
        Algorithm::Openzl => Some("OpenZL: Meta's format-aware compression, excellent for structured data (JSON, XML, Protobuf)".to_string()),
    }
}

/// Get list of supported algorithms
pub fn supported_algorithms() -> Vec<Algorithm> {
    #[cfg(target_family = "wasm")]
    {
        vec![
            Algorithm::Store,
            Algorithm::Deflate,
            Algorithm::Bzip2,
            Algorithm::Lzma,
            Algorithm::Zstd,
            Algorithm::Lz4,
            Algorithm::Openzl,
        ]
    }
    #[cfg(not(target_family = "wasm"))]
    {
        vec![
            Algorithm::Store,
            Algorithm::Deflate,
            Algorithm::Bzip2,
            Algorithm::Lzma,
            Algorithm::Zstd,
            Algorithm::Lz4,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_provider() {
        let provider = StoreProvider;
        let data = b"test data";
        let compressed = provider.compress(data, 0).unwrap();
        assert_eq!(compressed, data);
        let decompressed = provider.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_deflate_provider() {
        let provider = DeflateProvider;
        let data = b"Hello, World! ".repeat(100);
        let compressed = provider.compress(&data, 6).unwrap();
        assert!(compressed.len() < data.len());
        let decompressed = provider.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_bzip2_provider() {
        let provider = Bzip2Provider;
        let data = b"BZIP2 test data. ".repeat(50);
        let compressed = provider.compress(&data, 6).unwrap();
        assert!(compressed.len() < data.len());
        let decompressed = provider.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_lzma_provider() {
        let provider = LzmaProvider;
        let data = b"LZMA test. ".repeat(50);
        let compressed = provider.compress(&data, 6).unwrap();
        assert!(compressed.len() < data.len());
        let decompressed = provider.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_get_provider() {
        assert!(get_provider(Algorithm::Store).is_ok());
        assert!(get_provider(Algorithm::Deflate).is_ok());
        assert!(get_provider(Algorithm::Bzip2).is_ok());
        assert!(get_provider(Algorithm::Lzma).is_ok());
        #[cfg(feature = "zstd")]
        assert!(get_provider(Algorithm::Zstd).is_ok());
        assert!(get_provider(Algorithm::Lz4).is_ok());
        // OpenZL is only available on WASM
        #[cfg(target_family = "wasm")]
        assert!(get_provider(Algorithm::Openzl).is_ok());
        #[cfg(not(target_family = "wasm"))]
        assert!(get_provider(Algorithm::Openzl).is_err());
    }

    #[test]
    fn test_supported_algorithms_list() {
        let algos = supported_algorithms();
        // store, deflate, bzip2, lzma, zstd, lz4 (+ openzl on wasm)
        #[cfg(target_family = "wasm")]
        assert_eq!(algos.len(), 7);
        #[cfg(not(target_family = "wasm"))]
        assert_eq!(algos.len(), 6);
        assert!(algos.contains(&Algorithm::Zstd));
        assert!(algos.contains(&Algorithm::Lz4));
    }

    #[test]
    fn test_lz4_provider() {
        let provider = Lz4Provider;
        let data = b"LZ4 is extremely fast! ".repeat(50);
        let compressed = provider.compress(&data, 0).unwrap();
        assert!(compressed.len() < data.len());
        let decompressed = provider.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[cfg(feature = "zstd")]
    #[test]
    fn test_zstd_provider() {
        let provider = ZstdProvider;
        let data = b"Zstandard provides excellent compression! ".repeat(50);
        let compressed = provider.compress(&data, 6).unwrap();
        assert!(compressed.len() < data.len());
        let decompressed = provider.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    #[cfg(target_family = "wasm")]
    fn test_openzl_provider() {
        let provider = OpenZlProvider;
        let data = b"OpenZL test data for structured compression. ".repeat(50);
        let compressed = provider.compress(&data, 6).unwrap();
        assert!(compressed.len() < data.len());
        let decompressed = provider.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }
}
