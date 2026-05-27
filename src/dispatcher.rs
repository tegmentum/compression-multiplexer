// Compression dispatcher implementation
//
// Implements the compression-dispatcher WIT interface by routing
// requests to the appropriate compression provider.

use crate::bindings::exports::tegmentum::compression_multiplexer::compression_dispatcher::{
    Algorithm, Guest, GuestCompressor, GuestDecompressor,
};
use crate::providers::{self, CompressionProvider};

/// Convert WIT-generated Algorithm to internal providers::Algorithm
fn to_internal_algorithm(algo: Algorithm) -> providers::Algorithm {
    match algo {
        Algorithm::Store => providers::Algorithm::Store,
        Algorithm::Deflate => providers::Algorithm::Deflate,
        Algorithm::Bzip2 => providers::Algorithm::Bzip2,
        Algorithm::Lzma => providers::Algorithm::Lzma,
        Algorithm::Zstd => providers::Algorithm::Zstd,
        Algorithm::Lz4 => providers::Algorithm::Lz4,
        Algorithm::Openzl => providers::Algorithm::Openzl,
    }
}

/// Convert internal providers::Algorithm to WIT-generated Algorithm
fn to_wit_algorithm(algo: providers::Algorithm) -> Algorithm {
    match algo {
        providers::Algorithm::Store => Algorithm::Store,
        providers::Algorithm::Deflate => Algorithm::Deflate,
        providers::Algorithm::Bzip2 => Algorithm::Bzip2,
        providers::Algorithm::Lzma => Algorithm::Lzma,
        providers::Algorithm::Zstd => Algorithm::Zstd,
        providers::Algorithm::Lz4 => Algorithm::Lz4,
        providers::Algorithm::Openzl => Algorithm::Openzl,
    }
}

/// Compressor resource implementation
pub struct Compressor {
    provider: Option<Box<dyn CompressionProvider>>,
    level: u8,
    error: Option<String>,
}

impl GuestCompressor for Compressor {
    fn new(algorithm: Algorithm, level: u8) -> Self {
        // Validate compression level
        if level > 9 {
            return Compressor {
                provider: None,
                level,
                error: Some(format!("Invalid compression level: {}. Must be 0-9.", level)),
            };
        }

        // Get provider for algorithm
        match providers::get_provider(to_internal_algorithm(algorithm)) {
            Ok(provider) => Compressor {
                provider: Some(provider),
                level,
                error: None,
            },
            Err(e) => Compressor {
                provider: None,
                level,
                error: Some(e),
            },
        }
    }

    fn compress(&self, input: Vec<u8>) -> Result<Vec<u8>, String> {
        // Return error if construction failed
        if let Some(ref error) = self.error {
            return Err(error.clone());
        }

        // Provider must exist if no error
        self.provider
            .as_ref()
            .unwrap()
            .compress(&input, self.level)
    }
}

/// Decompressor resource implementation
pub struct Decompressor {
    provider: Option<Box<dyn CompressionProvider>>,
    error: Option<String>,
}

impl GuestDecompressor for Decompressor {
    fn new(algorithm: Algorithm) -> Self {
        // Get provider for algorithm
        match providers::get_provider(to_internal_algorithm(algorithm)) {
            Ok(provider) => Decompressor {
                provider: Some(provider),
                error: None,
            },
            Err(e) => Decompressor {
                provider: None,
                error: Some(e),
            },
        }
    }

    fn decompress(&self, input: Vec<u8>) -> Result<Vec<u8>, String> {
        // Return error if construction failed
        if let Some(ref error) = self.error {
            return Err(error.clone());
        }

        // Provider must exist if no error
        self.provider.as_ref().unwrap().decompress(&input)
    }
}

/// Multiplexer implementation (for Guest trait functions)
pub struct MultiplexerImpl;

impl Guest for MultiplexerImpl {
    type Compressor = Compressor;
    type Decompressor = Decompressor;

    fn supported_algorithms() -> Vec<Algorithm> {
        providers::supported_algorithms()
            .into_iter()
            .map(to_wit_algorithm)
            .collect()
    }

    fn algorithm_info(algo: Algorithm) -> Option<String> {
        providers::algorithm_description(to_internal_algorithm(algo))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compressor_invalid_level() {
        let compressor = Compressor::new(Algorithm::Deflate, 10);
        let result = compressor.compress(vec![1, 2, 3]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid compression level"));
    }

    #[test]
    fn test_lz4_compression() {
        let compressor = Compressor::new(Algorithm::Lz4, 0);
        let data = b"LZ4 fast compression! ".repeat(100).to_vec();
        let compressed = compressor.compress(data.clone()).unwrap();
        assert!(compressed.len() < data.len());

        let decompressor = Decompressor::new(Algorithm::Lz4);
        let decompressed = decompressor.decompress(compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_zstd_compression() {
        let compressor = Compressor::new(Algorithm::Zstd, 6);
        let data = b"Zstandard compression! ".repeat(100).to_vec();
        let compressed = compressor.compress(data.clone()).unwrap();
        assert!(compressed.len() < data.len());

        let decompressor = Decompressor::new(Algorithm::Zstd);
        let decompressed = decompressor.decompress(compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_store_compression() {
        let compressor = Compressor::new(Algorithm::Store, 0);
        let data = vec![1, 2, 3, 4, 5];
        let compressed = compressor.compress(data.clone()).unwrap();
        assert_eq!(compressed, data);
    }

    #[test]
    fn test_deflate_compression() {
        let compressor = Compressor::new(Algorithm::Deflate, 6);
        let data = b"Hello, World! ".repeat(100).to_vec();
        let compressed = compressor.compress(data.clone()).unwrap();
        assert!(compressed.len() < data.len());

        let decompressor = Decompressor::new(Algorithm::Deflate);
        let decompressed = decompressor.decompress(compressed).unwrap();
        assert_eq!(decompressed, data);
    }
}
