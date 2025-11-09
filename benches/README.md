# Compression Multiplexer Benchmarks

Performance benchmarks for the compression multiplexer component.

## Running Benchmarks

### Run All Benchmarks

```bash
cargo bench
```

### Run Specific Benchmark Group

```bash
# Compression algorithms comparison
cargo bench --bench compression_bench -- compression-algorithms

# Compression levels comparison
cargo bench --bench compression_bench -- compression-levels

# Decompression performance
cargo bench --bench compression_bench -- decompression

# Different data types
cargo bench --bench compression_bench -- data-types

# Full roundtrip (compress + decompress)
cargo bench --bench compression_bench -- roundtrip
```

### Generate HTML Reports

```bash
cargo bench
# Reports generated in target/criterion/
open target/criterion/report/index.html
```

## Benchmark Groups

### 1. Compression Algorithms

**What it measures:** Compression throughput for different algorithms

**Test sizes:** 1KB, 10KB, 100KB

**Algorithms tested:**
- Store (baseline - no compression)
- DEFLATE
- BZIP2
- LZMA

**Use case:** Choose the right algorithm for your data size

### 2. Compression Levels

**What it measures:** Impact of compression level on performance

**Levels tested:** 1 (fastest), 3, 6 (balanced), 9 (best)

**Algorithms:** DEFLATE, BZIP2, LZMA

**Use case:** Balance compression ratio vs speed

### 3. Decompression

**What it measures:** Decompression throughput

**Algorithms:** All (Store, DEFLATE, BZIP2, LZMA)

**Use case:** Understand read performance in production

### 4. Data Types

**What it measures:** Algorithm performance on different data patterns

**Data patterns:**
- Random (worst case for compression)
- Repetitive (best case for compression)
- Text (realistic case for compression)

**Use case:** Choose algorithm based on your data characteristics

### 5. Roundtrip

**What it measures:** Full compress + decompress cycle

**Use case:** End-to-end performance testing

## Expected Results

### Compression Speed (Relative)

Based on typical benchmarks with repetitive data:

```
Store:    ~10 GB/s  (baseline - no compression)
DEFLATE:  ~50 MB/s  (level 6)
BZIP2:    ~10 MB/s  (level 6)
LZMA:     ~5 MB/s   (level 6)
```

### Decompression Speed (Relative)

```
Store:    ~10 GB/s  (baseline)
DEFLATE:  ~200 MB/s
BZIP2:    ~30 MB/s
LZMA:     ~50 MB/s
```

### Compression Ratio (Typical)

For repetitive text data:

```
Store:    1.0x  (no compression)
DEFLATE:  10-15x
BZIP2:    15-20x
LZMA:     20-30x
```

## Interpreting Results

### Throughput

Higher is better. Measured in bytes/second.

Example:
```
compression-algorithms/deflate-compress/10240
                        time:   [204.23 µs 205.45 µs 206.89 µs]
                        thrpt:  [47.36 MiB/s 47.68 MiB/s 47.96 MiB/s]
```

This means DEFLATE compresses at ~47 MiB/s for 10KB input.

### Compression Level Trade-offs

Example results:
```
Level 1:  Fast compression  (100 MB/s), lower ratio (8x)
Level 6:  Balanced         (50 MB/s),  good ratio (12x)
Level 9:  Best compression (20 MB/s),  best ratio (15x)
```

**Recommendation:**
- Use level 1-3 for frequently accessed data
- Use level 6 for balanced performance (default)
- Use level 9 for long-term storage

### Data Type Impact

Example results:
```
Random data:      Poor compression (1.2x ratio, slow)
Repetitive data:  Excellent compression (20x ratio, fast)
Text data:        Good compression (10x ratio, medium)
```

## Baseline Comparisons

### vs. Built-in ZIP (no multiplexer)

The multiplexer adds minimal overhead:

```
Multiplexer overhead: <1% (enum dispatch + trait call)
```

### vs. Native Libraries

WASM performance is typically:

```
~60-80% of native speed (depends on host)
```

## Performance Tips

### For Best Compression Speed

1. Use DEFLATE algorithm (fastest of the real compressors)
2. Use lower compression levels (1-3)
3. Process larger chunks when possible

### For Best Compression Ratio

1. Use LZMA algorithm (best compression)
2. Use higher compression levels (6-9)
3. Use BZIP2 for repetitive data as a balance

### For Read-Heavy Workloads

1. Decompression is typically 3-10x faster than compression
2. All algorithms decompress reasonably fast
3. Choose based on compression ratio needs

### For Specific Data Types

**Random/Binary Data:**
- Use Store or DEFLATE level 1
- Compression won't help much

**Repetitive Data:**
- Use BZIP2 or LZMA
- Excellent compression ratios

**Text/Source Code:**
- Use DEFLATE level 6 (balanced)
- Good ratio, reasonable speed

## Continuous Monitoring

### Regression Testing

Run benchmarks before releases:

```bash
# Baseline
git checkout main
cargo bench --bench compression_bench -- --save-baseline main

# Your changes
git checkout feature-branch
cargo bench --bench compression_bench -- --baseline main
```

### CI Integration

Add to `.github/workflows/bench.yml`:

```yaml
name: Benchmarks

on:
  push:
    branches: [main]
  pull_request:

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run benchmarks
        run: cargo bench --bench compression_bench
      - name: Upload results
        uses: actions/upload-artifact@v4
        with:
          name: benchmark-results
          path: target/criterion/
```

## Troubleshooting

### Benchmarks are unstable

1. Close other applications
2. Disable CPU frequency scaling
3. Run multiple times and average

### Out of memory

1. Reduce test data sizes
2. Run benchmark groups separately
3. Increase swap space

### WASM build errors

These benchmarks run natively (not in WASM). They test the Rust implementations directly for maximum accuracy.

## References

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [DEFLATE RFC 1951](https://www.rfc-editor.org/rfc/rfc1951)
- [BZIP2 Specification](https://sourceware.org/bzip2/)
- [LZMA SDK](https://www.7-zip.org/sdk.html)

---

*Last Updated: 2025-11-09*
