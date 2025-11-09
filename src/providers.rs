// Compression algorithm providers
//
// These are built-in implementations using Rust crates.
// In the future, these will be replaced with WIT imports from Layer 1 components.

use std::io::{Read, Write};

// Re-export Algorithm from WIT bindings
pub use crate::bindings::exports::tegmentum::compression_multiplexer::compression_dispatcher::Algorithm;

/// Compression provider trait
///
/// Each algorithm implements this trait to provide compress/decompress functionality
pub trait CompressionProvider {
    fn compress(&self, input: &[u8], level: u8) -> Result<Vec<u8>, String>;
    fn decompress(&self, input: &[u8]) -> Result<Vec<u8>, String>;
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

/// LZMA provider
pub struct LzmaProvider;

impl CompressionProvider for LzmaProvider {
    fn compress(&self, input: &[u8], level: u8) -> Result<Vec<u8>, String> {
        use lzma_rust2::{LzmaOptions, LzmaWriter};

        let level = level.min(9);
        let output = Vec::new();
        let options = LzmaOptions::with_preset(level as u32);

        let mut encoder = LzmaWriter::new_use_header(output, &options, Some(input.len() as u64))
            .map_err(|e| format!("LZMA encoder creation failed: {}", e))?;

        encoder
            .write_all(input)
            .map_err(|e| format!("LZMA compression failed: {}", e))?;

        encoder
            .finish()
            .map_err(|e| format!("LZMA finish failed: {}", e))
    }

    fn decompress(&self, input: &[u8]) -> Result<Vec<u8>, String> {
        use lzma_rust2::LzmaReader;

        let mut decoder = LzmaReader::new_mem_limit(input, 64 * 1024, None)
            .map_err(|e| format!("LZMA decoder creation failed: {}", e))?;
        let mut output = Vec::new();

        decoder
            .read_to_end(&mut output)
            .map_err(|e| format!("LZMA decompression failed: {}", e))?;

        Ok(output)
    }
}

/// Get provider for algorithm
pub fn get_provider(algorithm: Algorithm) -> Result<Box<dyn CompressionProvider>, String> {
    match algorithm {
        Algorithm::Store => Ok(Box::new(StoreProvider)),
        Algorithm::Deflate => Ok(Box::new(DeflateProvider)),
        Algorithm::Bzip2 => Ok(Box::new(Bzip2Provider)),
        Algorithm::Lzma => Ok(Box::new(LzmaProvider)),
        Algorithm::Zstd => Err("Zstd algorithm is not supported (C dependencies incompatible with WASM)".to_string()),
    }
}

/// Get algorithm information
pub fn algorithm_description(algorithm: Algorithm) -> Option<String> {
    match algorithm {
        Algorithm::Store => Some("Store: No compression (pass-through)".to_string()),
        Algorithm::Deflate => Some("DEFLATE: Fast general-purpose compression (RFC 1951), used in ZIP, GZIP, PNG".to_string()),
        Algorithm::Bzip2 => Some("BZIP2: High compression ratio, good for repetitive data".to_string()),
        Algorithm::Lzma => Some("LZMA: Excellent compression ratio, used in 7-Zip and XZ".to_string()),
        Algorithm::Zstd => Some("Zstandard: Modern fast compression (not currently supported in WASM)".to_string()),
    }
}

/// Get list of supported algorithms
pub fn supported_algorithms() -> Vec<Algorithm> {
    vec![
        Algorithm::Store,
        Algorithm::Deflate,
        Algorithm::Bzip2,
        Algorithm::Lzma,
        // Note: Zstd is NOT included as it's not supported
    ]
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
        assert!(get_provider(Algorithm::Zstd).is_err());
    }

    #[test]
    fn test_supported_algorithms_list() {
        let algos = supported_algorithms();
        assert_eq!(algos.len(), 4); // store, deflate, bzip2, lzma
        assert!(!algos.contains(&Algorithm::Zstd)); // Not supported
    }
}
