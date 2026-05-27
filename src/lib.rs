// Compression Multiplexer
//
// Provides unified compression dispatcher that routes to multiple algorithms.
// On WASM targets, exports Component Model interface.
// On native targets, provides library access to compression providers.

#[cfg(target_family = "wasm")]
mod bindings;
#[cfg(target_family = "wasm")]
mod dispatcher;
#[cfg(target_family = "wasm")]
mod zstd_extras;

mod openzl_ffi;
pub mod providers;

// Re-export for convenience
pub use providers::{CompressionProvider, Algorithm};

#[cfg(target_family = "wasm")]
pub use dispatcher::{Compressor, Decompressor, MultiplexerImpl};

// Export the WIT bindings for WASM targets
#[cfg(target_family = "wasm")]
bindings::export!(MultiplexerImpl with_types_in bindings);

#[cfg(test)]
mod tests {
    use super::*;
    use providers::get_provider;

    #[test]
    fn test_supported_algorithms() {
        let algos = providers::supported_algorithms();
        // At minimum: store, deflate, bzip2, lzma, zstd, lz4
        // OpenZL is only available on WASM
        assert!(algos.len() >= 6);
    }

    #[test]
    fn test_algorithm_info() {
        let info = providers::algorithm_description(Algorithm::Deflate);
        assert!(info.is_some());
        assert!(info.unwrap().contains("DEFLATE"));
    }

    #[test]
    fn test_store_passthrough() {
        let provider = get_provider(Algorithm::Store).unwrap();
        let data = b"Hello, World!";
        let compressed = provider.compress(data, 0).unwrap();
        assert_eq!(compressed, data); // Store is pass-through

        let decompressed = provider.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_deflate_roundtrip() {
        let provider = get_provider(Algorithm::Deflate).unwrap();
        let data = b"Hello, World! ".repeat(100); // Repetitive data compresses well
        let compressed = provider.compress(&data, 6).unwrap();
        assert!(compressed.len() < data.len()); // Should be smaller

        let decompressed = provider.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_bzip2_roundtrip() {
        let provider = get_provider(Algorithm::Bzip2).unwrap();
        let data = b"BZIP2 compression test data. ".repeat(50);
        let compressed = provider.compress(&data, 6).unwrap();
        assert!(compressed.len() < data.len());

        let decompressed = provider.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_lzma_roundtrip() {
        let provider = get_provider(Algorithm::Lzma).unwrap();
        let data = b"LZMA compression test. ".repeat(50);
        let compressed = provider.compress(&data, 6).unwrap();
        assert!(compressed.len() < data.len());

        let decompressed = provider.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_lz4_roundtrip() {
        let provider = get_provider(Algorithm::Lz4).unwrap();
        let data = b"LZ4 is extremely fast! ".repeat(100);
        let compressed = provider.compress(&data, 0).unwrap();
        assert!(compressed.len() < data.len());

        let decompressed = provider.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_zstd_roundtrip() {
        let provider = get_provider(Algorithm::Zstd).unwrap();
        let data = b"Zstandard compression test data. ".repeat(100);
        let compressed = provider.compress(&data, 6).unwrap();
        assert!(compressed.len() < data.len());

        let decompressed = provider.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    #[cfg(target_family = "wasm")]
    fn test_openzl_roundtrip() {
        let provider = get_provider(Algorithm::Openzl).unwrap();
        let data = b"OpenZL structured data test. ".repeat(50);
        let compressed = provider.compress(&data, 6).unwrap();
        assert!(compressed.len() < data.len());

        let decompressed = provider.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    #[cfg(not(target_family = "wasm"))]
    fn test_openzl_not_available_on_native() {
        let result = get_provider(Algorithm::Openzl);
        assert!(result.is_err());
        match result {
            Err(msg) => assert!(msg.contains("WASM")),
            Ok(_) => panic!("Expected error for OpenZL on native"),
        }
    }

    #[test]
    fn test_compression_levels() {
        let data = b"Test data for compression levels. ".repeat(50);
        let provider = get_provider(Algorithm::Deflate).unwrap();

        // Level 0 (fastest)
        let compressed0 = provider.compress(&data, 0).unwrap();

        // Level 9 (best)
        let compressed9 = provider.compress(&data, 9).unwrap();

        // Level 9 should generally produce smaller output
        // (though not guaranteed for all data)
        println!(
            "Level 0: {} bytes, Level 9: {} bytes",
            compressed0.len(),
            compressed9.len()
        );

        // Both should decompress correctly
        assert_eq!(provider.decompress(&compressed0).unwrap(), data);
        assert_eq!(provider.decompress(&compressed9).unwrap(), data);
    }
}
