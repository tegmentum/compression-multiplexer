// Compression Multiplexer
//
// Provides unified compression dispatcher that routes to multiple algorithms.
// Currently uses built-in implementations until Component Model supports
// importing the same interface multiple times with different names.

mod bindings;
mod dispatcher;
mod providers;

// Re-export for convenience
pub use dispatcher::{Compressor, Decompressor, MultiplexerImpl};
pub use providers::{Algorithm, CompressionProvider};

// Export the WIT bindings
bindings::export!(MultiplexerImpl with_types_in bindings);

#[cfg(test)]
mod tests {
    use super::*;
    use bindings::exports::tegmentum::compression_multiplexer::compression_dispatcher::{
        Guest, GuestCompressor, GuestDecompressor,
    };

    #[test]
    fn test_supported_algorithms() {
        let algos = MultiplexerImpl::supported_algorithms();
        assert!(algos.len() >= 3); // At minimum: store, deflate, bzip2, lzma
        assert!(algos.contains(&Algorithm::Store));
        assert!(algos.contains(&Algorithm::Deflate));
    }

    #[test]
    fn test_algorithm_info() {
        let info = MultiplexerImpl::algorithm_info(Algorithm::Deflate);
        assert!(info.is_some());
        assert!(info.unwrap().contains("DEFLATE"));
    }

    #[test]
    fn test_store_passthrough() {
        let compressor = Compressor::new(Algorithm::Store, 0);
        let data = b"Hello, World!";
        let compressed = compressor.compress(data.to_vec()).unwrap();
        assert_eq!(compressed, data); // Store is pass-through

        let decompressor = Decompressor::new(Algorithm::Store);
        let decompressed = decompressor.decompress(compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_deflate_roundtrip() {
        let compressor = Compressor::new(Algorithm::Deflate, 6);
        let data = b"Hello, World! ".repeat(100); // Repetitive data compresses well
        let compressed = compressor.compress(data.to_vec()).unwrap();
        assert!(compressed.len() < data.len()); // Should be smaller

        let decompressor = Decompressor::new(Algorithm::Deflate);
        let decompressed = decompressor.decompress(compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_bzip2_roundtrip() {
        let compressor = Compressor::new(Algorithm::Bzip2, 6);
        let data = b"BZIP2 compression test data. ".repeat(50);
        let compressed = compressor.compress(data.to_vec()).unwrap();
        assert!(compressed.len() < data.len());

        let decompressor = Decompressor::new(Algorithm::Bzip2);
        let decompressed = decompressor.decompress(compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_lzma_roundtrip() {
        let compressor = Compressor::new(Algorithm::Lzma, 6);
        let data = b"LZMA compression test. ".repeat(50);
        let compressed = compressor.compress(data.to_vec()).unwrap();
        assert!(compressed.len() < data.len());

        let decompressor = Decompressor::new(Algorithm::Lzma);
        let decompressed = decompressor.decompress(compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_unsupported_algorithm() {
        // Zstd is not implemented yet (C dependencies)
        let compressor = Compressor::new(Algorithm::Zstd, 6);
        let result = compressor.compress(vec![1, 2, 3]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not supported"));
    }

    #[test]
    fn test_compression_levels() {
        let data = b"Test data for compression levels. ".repeat(50);

        // Level 0 (fastest)
        let comp0 = Compressor::new(Algorithm::Deflate, 0);
        let compressed0 = comp0.compress(data.to_vec()).unwrap();

        // Level 9 (best)
        let comp9 = Compressor::new(Algorithm::Deflate, 9);
        let compressed9 = comp9.compress(data.to_vec()).unwrap();

        // Level 9 should generally produce smaller output
        // (though not guaranteed for all data)
        println!(
            "Level 0: {} bytes, Level 9: {} bytes",
            compressed0.len(),
            compressed9.len()
        );

        // Both should decompress correctly
        let decomp = Decompressor::new(Algorithm::Deflate);
        assert_eq!(decomp.decompress(compressed0).unwrap(), data);
        assert_eq!(decomp.decompress(compressed9).unwrap(), data);
    }
}
