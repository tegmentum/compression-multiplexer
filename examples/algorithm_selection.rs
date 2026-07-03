// Algorithm selection example
//
// This example demonstrates how to choose the right compression algorithm
// based on data characteristics and performance requirements.

use compression_multiplexer::providers::{get_provider, Algorithm};

fn compress_and_report(data: &[u8], algorithm: Algorithm, level: u8) -> Result<(usize, String), String> {
    let provider = get_provider(algorithm)?;
    let compressed = provider.compress(data, level)?;

    let ratio = data.len() as f64 / compressed.len() as f64;
    let report = format!(
        "{:?} (level {}): {} bytes → {} bytes ({:.2}x ratio)",
        algorithm, level, data.len(), compressed.len(), ratio
    );

    Ok((compressed.len(), report))
}

fn main() -> Result<(), String> {
    println!("Compression Multiplexer - Algorithm Selection Guide\n");

    // Different data types
    let random_data: Vec<u8> = (0..10000).map(|i| ((i * 7 + 13) % 256) as u8).collect();
    let repetitive_data = b"AAAABBBBCCCCDDDD".repeat(625); // 10KB
    let text_data = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(170).into_bytes(); // ~10KB

    let datasets = vec![
        ("Random data", random_data),
        ("Repetitive data", repetitive_data),
        ("Text data", text_data),
    ];

    for (name, data) in datasets {
        println!("{}:", name);
        println!("  Original size: {} bytes\n", data.len());

        // Test different algorithms
        let algorithms = vec![
            Algorithm::Store,
            Algorithm::Deflate,
            Algorithm::Bzip2,
            Algorithm::Lzma,
        ];

        for algorithm in algorithms {
            if algorithm == Algorithm::Store {
                println!("  Store: {} bytes (no compression)", data.len());
                continue;
            }

            let (_compressed_size, report) = compress_and_report(&data, algorithm, 6)?;
            println!("  {}", report);
        }

        println!();
    }

    // Recommendations
    println!("📊 Recommendations:");
    println!();
    println!("Random/Binary Data:");
    println!("  • Use Store or DEFLATE level 1");
    println!("  • Random data doesn't compress well");
    println!("  • Focus on speed over ratio");
    println!();
    println!("Repetitive Data:");
    println!("  • Use BZIP2 or LZMA");
    println!("  • Excellent compression ratios (15-30x)");
    println!("  • Worth the extra CPU time");
    println!();
    println!("Text/Source Code:");
    println!("  • Use DEFLATE level 6 (balanced)");
    println!("  • Good ratio (8-12x)");
    println!("  • Fast compression/decompression");
    println!();
    println!("Network Transfer:");
    println!("  • Use DEFLATE level 3-6");
    println!("  • Fast compression is critical");
    println!("  • Good enough ratio");
    println!();
    println!("Long-term Storage:");
    println!("  • Use LZMA level 9");
    println!("  • Maximum compression");
    println!("  • One-time cost, read many times");

    Ok(())
}
