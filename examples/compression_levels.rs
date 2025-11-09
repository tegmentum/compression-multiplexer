// Compression levels example
//
// This example demonstrates the trade-offs between compression levels:
// - Higher levels = better compression ratio but slower
// - Lower levels = faster but larger output
//
// Run with: cargo run --example compression_levels --release

use compression_multiplexer::providers::{get_provider, Algorithm};
use std::time::Instant;

fn benchmark_level(
    data: &[u8],
    algorithm: Algorithm,
    level: u8,
) -> Result<(usize, u128), String> {
    let provider = get_provider(algorithm)?;

    let start = Instant::now();
    let compressed = provider.compress(data, level)?;
    let duration = start.elapsed().as_micros();

    Ok((compressed.len(), duration))
}

fn main() -> Result<(), String> {
    println!("Compression Multiplexer - Compression Levels Comparison\n");

    // Test data (10KB of repetitive text - good for showing differences)
    let data = "The quick brown fox jumps over the lazy dog. ".repeat(220).into_bytes();
    println!("Original data: {} bytes\n", data.len());

    let algorithms = vec![
        (Algorithm::Deflate, "DEFLATE"),
        (Algorithm::Bzip2, "BZIP2"),
        (Algorithm::Lzma, "LZMA"),
    ];

    let levels = vec![1, 3, 6, 9];

    for (algorithm, name) in algorithms {
        println!("{}:", name);
        println!("  Level | Size (bytes) | Time (µs) | Ratio | Speed (MB/s)");
        println!("  ------|--------------|-----------|-------|-------------");

        for &level in &levels {
            let (size, time_us) = benchmark_level(&data, algorithm, level)?;
            let ratio = data.len() as f64 / size as f64;
            let speed_mbps = if time_us > 0 {
                (data.len() as f64 / 1_048_576.0) / (time_us as f64 / 1_000_000.0)
            } else {
                0.0
            };

            println!(
                "  {:5} | {:12} | {:9} | {:5.2}x | {:11.1}",
                level, size, time_us, ratio, speed_mbps
            );
        }
        println!();
    }

    println!("📊 Level Selection Guide:");
    println!();
    println!("Level 1-3 (Fast):");
    println!("  ✓ Use for frequently accessed data");
    println!("  ✓ Real-time compression (network, caching)");
    println!("  ✓ When CPU time is limited");
    println!("  ✓ Temporary files");
    println!();
    println!("Level 6 (Balanced - DEFAULT):");
    println!("  ✓ Good all-around choice");
    println!("  ✓ Reasonable speed and compression");
    println!("  ✓ Most ZIP files use this");
    println!("  ✓ When unsure, use this");
    println!();
    println!("Level 9 (Best Compression):");
    println!("  ✓ Long-term archival");
    println!("  ✓ Rarely modified files");
    println!("  ✓ Network bandwidth is expensive");
    println!("  ✓ One-time compression, many reads");
    println!();
    println!("⚠️  Note: LZMA levels make less difference than DEFLATE/BZIP2");

    Ok(())
}
