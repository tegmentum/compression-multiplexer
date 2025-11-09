// CLI compression tool example
//
// A simple command-line tool for compressing and decompressing files
// using the compression multiplexer.
//
// Usage:
//   cargo run --example compress_cli --release -- compress input.txt -o output.bin -a deflate -l 6
//   cargo run --example compress_cli --release -- decompress output.bin -o restored.txt -a deflate
//   cargo run --example compress_cli --release -- info output.bin

use compression_multiplexer::providers::{get_provider, supported_algorithms, Algorithm};
use std::env;
use std::fs;

fn parse_algorithm(s: &str) -> Result<Algorithm, String> {
    match s.to_lowercase().as_str() {
        "store" => Ok(Algorithm::Store),
        "deflate" => Ok(Algorithm::Deflate),
        "bzip2" => Ok(Algorithm::Bzip2),
        "lzma" => Ok(Algorithm::Lzma),
        "zstd" => Ok(Algorithm::Zstd),
        _ => Err(format!("Unknown algorithm: {}", s)),
    }
}

fn compress_file(
    input: &str,
    output: &str,
    algorithm: Algorithm,
    level: u8,
) -> Result<(), String> {
    println!("Compressing {} → {}", input, output);
    println!("  Algorithm: {:?}", algorithm);
    println!("  Level: {}", level);

    // Read input file
    let data = fs::read(input).map_err(|e| format!("Failed to read input: {}", e))?;
    println!("  Input size: {} bytes", data.len());

    // Get provider and compress
    let provider = get_provider(algorithm)?;
    let compressed = provider.compress(&data, level)?;

    println!("  Output size: {} bytes", compressed.len());
    println!(
        "  Compression ratio: {:.2}x",
        data.len() as f64 / compressed.len() as f64
    );
    println!(
        "  Space saved: {:.1}%",
        (1.0 - compressed.len() as f64 / data.len() as f64) * 100.0
    );

    // Write output file
    fs::write(output, compressed).map_err(|e| format!("Failed to write output: {}", e))?;

    println!("✅ Compression successful!");
    Ok(())
}

fn decompress_file(input: &str, output: &str, algorithm: Algorithm) -> Result<(), String> {
    println!("Decompressing {} → {}", input, output);
    println!("  Algorithm: {:?}", algorithm);

    // Read compressed file
    let compressed = fs::read(input).map_err(|e| format!("Failed to read input: {}", e))?;
    println!("  Compressed size: {} bytes", compressed.len());

    // Get provider and decompress
    let provider = get_provider(algorithm)?;
    let decompressed = provider.decompress(&compressed)?;

    println!("  Decompressed size: {} bytes", decompressed.len());
    println!(
        "  Expansion ratio: {:.2}x",
        decompressed.len() as f64 / compressed.len() as f64
    );

    // Write output file
    fs::write(output, decompressed).map_err(|e| format!("Failed to write output: {}", e))?;

    println!("✅ Decompression successful!");
    Ok(())
}

fn show_info() {
    println!("Compression Multiplexer CLI Tool");
    println!();
    println!("Supported algorithms:");
    for algo in supported_algorithms() {
        println!("  • {:?}", algo);
    }
    println!();
    println!("Usage:");
    println!("  compress <input> -o <output> -a <algorithm> [-l <level>]");
    println!("  decompress <input> -o <output> -a <algorithm>");
    println!("  info");
    println!();
    println!("Examples:");
    println!("  compress file.txt -o file.deflate -a deflate -l 6");
    println!("  decompress file.deflate -o restored.txt -a deflate");
    println!();
    println!("Compression levels: 0-9 (higher = better compression, slower)");
    println!("  1-3: Fast compression");
    println!("  6:   Balanced (default)");
    println!("  9:   Best compression");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        show_info();
        return;
    }

    let result = match args[1].as_str() {
        "compress" => {
            if args.len() < 6 {
                Err("Usage: compress <input> -o <output> -a <algorithm> [-l <level>]".to_string())
            } else {
                let input = &args[2];
                let output = &args[4];
                let algorithm = parse_algorithm(&args[6])?;
                let level = if args.len() >= 9 && args[7] == "-l" {
                    args[8].parse::<u8>().unwrap_or(6)
                } else {
                    6
                };

                compress_file(input, output, algorithm, level)
            }
        }
        "decompress" => {
            if args.len() < 7 {
                Err("Usage: decompress <input> -o <output> -a <algorithm>".to_string())
            } else {
                let input = &args[2];
                let output = &args[4];
                let algorithm = parse_algorithm(&args[6])?;

                decompress_file(input, output, algorithm)
            }
        }
        "info" => {
            show_info();
            Ok(())
        }
        _ => {
            show_info();
            Err(format!("Unknown command: {}", args[1]))
        }
    };

    if let Err(e) = result {
        eprintln!("❌ Error: {}", e);
        std::process::exit(1);
    }
}
