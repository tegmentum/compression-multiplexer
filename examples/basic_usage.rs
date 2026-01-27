// Basic usage example for compression-multiplexer
//
// This example demonstrates how to use the compression multiplexer
// to compress and decompress data with different algorithms.

use compression_multiplexer::providers::{get_provider, Algorithm};

fn main() -> Result<(), String> {
    println!("Compression Multiplexer - Basic Usage Example\n");

    // Test data
    let original_data = b"Hello, World! This is a test of the compression multiplexer. ".repeat(100);
    println!("Original data size: {} bytes\n", original_data.len());

    // Test all available algorithms
    let algorithms = vec![
        (Algorithm::Store, "Store (no compression)"),
        (Algorithm::Deflate, "DEFLATE"),
        (Algorithm::Bzip2, "BZIP2"),
        (Algorithm::Lzma, "LZMA"),
        (Algorithm::Openzl, "OpenZL"),
    ];

    for (algorithm, name) in algorithms {
        println!("Testing {}:", name);
        println!("  Algorithm: {:?}", algorithm);

        // Get provider for this algorithm
        let provider = get_provider(algorithm)?;

        // Compress with level 6 (balanced)
        let compressed = provider.compress(&original_data, 6)?;
        let compression_ratio = original_data.len() as f64 / compressed.len() as f64;

        println!("  Compressed size: {} bytes", compressed.len());
        println!("  Compression ratio: {:.2}x", compression_ratio);
        println!(
            "  Space saved: {:.1}%",
            (1.0 - (compressed.len() as f64 / original_data.len() as f64)) * 100.0
        );

        // Decompress
        let decompressed = provider.decompress(&compressed)?;

        // Verify roundtrip
        if decompressed == original_data {
            println!("  ✅ Roundtrip successful");
        } else {
            println!("  ❌ Roundtrip failed!");
            return Err("Data mismatch after roundtrip".to_string());
        }

        println!();
    }

    println!("All tests passed!");
    Ok(())
}
