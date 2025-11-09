# Compression Multiplexer Examples

This directory contains comprehensive examples demonstrating how to use the compression multiplexer in various scenarios.

## Running Examples

### Basic Usage

```bash
# Run a specific example
cargo run --example basic_usage --release

# Run with native speed (recommended)
cargo run --example basic_usage --release

# List all examples
cargo run --example
```

## Available Examples

### 1. Basic Usage (`basic_usage.rs`)

**What it demonstrates:**
- How to use the compression multiplexer API
- Compressing data with different algorithms
- Decompressing data
- Verifying roundtrip integrity

**Run:**
```bash
cargo run --example basic_usage
```

**Output:**
- Compression results for Store, DEFLATE, BZIP2, LZMA
- Compression ratios and space saved
- Roundtrip verification

**When to use this:**
- Learning the basic API
- First-time users
- Quick testing

### 2. Algorithm Selection (`algorithm_selection.rs`)

**What it demonstrates:**
- How to choose the right algorithm for your data
- Performance differences between algorithms
- Data type impact on compression

**Run:**
```bash
cargo run --example algorithm_selection --release
```

**Output:**
- Compression results for different data types:
  - Random data (worst case)
  - Repetitive data (best case)
  - Text data (realistic case)
- Recommendations for each use case

**When to use this:**
- Choosing algorithm for production
- Understanding trade-offs
- Optimizing for specific data types

### 3. Compression Levels (`compression_levels.rs`)

**What it demonstrates:**
- Impact of compression level on ratio and speed
- Trade-offs between levels 1, 3, 6, and 9
- Performance comparison across algorithms

**Run:**
```bash
cargo run --example compression_levels --release
```

**Output:**
- Compression size, time, and ratio for each level
- Speed in MB/s
- Recommendations for level selection

**When to use this:**
- Tuning compression performance
- Balancing speed vs ratio
- Understanding level impact

### 4. CLI Tool (`compress_cli.rs`)

**What it demonstrates:**
- Practical command-line tool implementation
- File-based compression/decompression
- Real-world usage pattern

**Run:**
```bash
# Compress a file
cargo run --example compress_cli --release -- compress input.txt -o output.bin -a deflate -l 6

# Decompress a file
cargo run --example compress_cli --release -- decompress output.bin -o restored.txt -a deflate

# Show info
cargo run --example compress_cli --release -- info
```

**When to use this:**
- Building a compression tool
- Testing with real files
- Understanding file I/O integration

**Try it out:**
```bash
# Create a test file
echo "Hello, World!" > test.txt

# Compress with DEFLATE
cargo run --example compress_cli --release -- \
  compress test.txt -o test.deflate -a deflate -l 6

# Compress with BZIP2
cargo run --example compress_cli --release -- \
  compress test.txt -o test.bzip2 -a bzip2 -l 9

# Compress with LZMA
cargo run --example compress_cli --release -- \
  compress test.txt -o test.lzma -a lzma -l 6

# Compare sizes
ls -lh test.*

# Decompress
cargo run --example compress_cli --release -- \
  decompress test.deflate -o restored.txt -a deflate

# Verify
diff test.txt restored.txt
```

### 5. Archive Provider Integration (`archive_provider_integration.rs`)

**What it demonstrates:**
- How archive formats (ZIP, TAR, 7Z) integrate with multiplexer
- Runtime compression method selection
- Handling different compression methods in one archive
- Real-world integration pattern

**Run:**
```bash
cargo run --example archive_provider_integration --release
```

**Output:**
- Mock ZIP archive creation
- Files added with different compression methods
- Archive listing
- File extraction

**When to use this:**
- Building an archive provider (Layer 2)
- Understanding the integration pattern
- Implementing format-specific logic

**Key Pattern:**
```rust
// Archive provider imports multiplexer
use compression_multiplexer::providers::{get_provider, Algorithm};

// Map file type to algorithm
let algorithm = match file_type {
    FileType::Text => Algorithm::Deflate,
    FileType::Repetitive => Algorithm::Bzip2,
    FileType::AlreadyCompressed => Algorithm::Store,
};

// Use multiplexer to compress
let provider = get_provider(algorithm)?;
let compressed = provider.compress(&data, level)?;
```

## Example Use Cases

### Quick Compression Test

```rust
use compression_multiplexer::providers::{get_provider, Algorithm};

let data = b"test data";
let provider = get_provider(Algorithm::Deflate)?;
let compressed = provider.compress(data, 6)?;
let decompressed = provider.decompress(&compressed)?;
assert_eq!(data, &decompressed[..]);
```

### Choose Algorithm Based on Data

```rust
fn choose_algorithm(data: &[u8]) -> Algorithm {
    // Simple heuristic
    let sample_size = data.len().min(1024);
    let unique_bytes = data[..sample_size]
        .iter()
        .collect::<std::collections::HashSet<_>>()
        .len();

    if unique_bytes as f64 / sample_size as f64 < 0.1 {
        // Very repetitive
        Algorithm::Bzip2
    } else if unique_bytes as f64 / sample_size as f64 < 0.5 {
        // Somewhat repetitive (text-like)
        Algorithm::Deflate
    } else {
        // Random/binary
        Algorithm::Store
    }
}
```

### Adaptive Compression Level

```rust
fn choose_level(data_size: usize, priority: Priority) -> u8 {
    match priority {
        Priority::Speed if data_size > 1_000_000 => 1,  // Large files, fast
        Priority::Speed => 3,                            // Small files, fast
        Priority::Balanced => 6,                         // Default
        Priority::Ratio if data_size < 10_000 => 6,     // Small files, level 9 not worth it
        Priority::Ratio => 9,                            // Large files, best compression
    }
}
```

## Building Your Own Tool

### Step 1: Add Dependency

```toml
[dependencies]
compression-multiplexer = { path = "/path/to/compression-multiplexer" }
```

### Step 2: Import

```rust
use compression_multiplexer::providers::{get_provider, Algorithm, supported_algorithms};
```

### Step 3: Use

```rust
fn main() -> Result<(), String> {
    let data = std::fs::read("input.txt")
        .map_err(|e| format!("Failed to read: {}", e))?;

    let provider = get_provider(Algorithm::Deflate)?;
    let compressed = provider.compress(&data, 6)?;

    std::fs::write("output.bin", compressed)
        .map_err(|e| format!("Failed to write: {}", e))?;

    Ok(())
}
```

## Performance Tips

### For Best Speed
- Use `Algorithm::Deflate` with level 1-3
- Process data in chunks
- Use `--release` mode
- Example: `compression_levels.rs` shows speed comparison

### For Best Ratio
- Use `Algorithm::Lzma` with level 9
- Acceptable for one-time compression
- Example: `algorithm_selection.rs` shows ratio comparison

### For Balance
- Use `Algorithm::Deflate` with level 6
- Good default choice
- Example: `basic_usage.rs` uses level 6

## Common Patterns

### Error Handling

```rust
match get_provider(algorithm) {
    Ok(provider) => {
        match provider.compress(&data, level) {
            Ok(compressed) => /* use compressed data */,
            Err(e) => eprintln!("Compression failed: {}", e),
        }
    }
    Err(e) => eprintln!("Unsupported algorithm: {}", e),
}
```

### Algorithm Discovery

```rust
for algo in supported_algorithms() {
    println!("Supported: {:?}", algo);
    if let Some(info) = algorithm_info(algo) {
        println!("  {}", info);
    }
}
```

### Batch Processing

```rust
let files = vec!["file1.txt", "file2.txt", "file3.txt"];
let provider = get_provider(Algorithm::Deflate)?;

for file in files {
    let data = fs::read(file)?;
    let compressed = provider.compress(&data, 6)?;
    fs::write(format!("{}.deflate", file), compressed)?;
}
```

## Testing Examples

Run all examples as tests:

```bash
# Run all examples
for example in basic_usage algorithm_selection compression_levels compress_cli archive_provider_integration; do
    echo "Running $example..."
    cargo run --example $example --release
    echo ""
done
```

## Next Steps

After exploring these examples:

1. **Read the benchmarks** - `../benches/README.md`
2. **Check the tests** - `cargo test --lib`
3. **Build the component** - `cargo component build --release --target wasm32-wasip2`
4. **Read the main README** - `../README.md`
5. **Explore the architecture** - See `/Users/zacharywhitley/git/compressed-vfs-wasm/ARCHITECTURE.md`

## Questions?

- See the main [README.md](../README.md) for architecture overview
- See [MULTIPLEXER_DESIGN.md](../../compressed-vfs-wasm/MULTIPLEXER_DESIGN.md) for design details
- Run benchmarks to measure performance on your machine

---

*Last Updated: 2025-11-09*
*Examples tested with compression-multiplexer v0.1.0*
