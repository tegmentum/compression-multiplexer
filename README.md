# Compression Multiplexer

A WebAssembly Component Model multiplexer that provides unified compression algorithm selection for archive format providers.

## Overview

The compression multiplexer solves the Component Model's "single import per interface" limitation by providing a unified dispatcher that routes compression requests to multiple algorithm providers at runtime.

### Problem

Without the multiplexer:
- Archive providers (ZIP, TAR, etc.) can only import one compression algorithm
- Need to embed multiple algorithms as built-in implementations
- Larger binary size, no runtime algorithm selection

### Solution

With the multiplexer:
- Archive providers import one `compression-dispatcher` interface
- Multiplexer routes to multiple algorithms (DEFLATE, BZIP2, LZMA, etc.)
- Runtime algorithm selection via enum parameter
- Smaller, more modular components

## Architecture

```
┌─────────────────┐
│  ZIP Provider   │ (Layer 2)
└────────┬────────┘
         │ imports compression-dispatcher
         │
    ┌────▼──────────┐
    │ Multiplexer   │ (Layer 1.5)
    └──┬──┬──┬──┬───┘
       │  │  │  │
    ┌──▼┐┌▼┐┌▼┐┌▼┐
    │STR││DEF││BZ││LZ│ (Providers)
    └───┘└─┘└─┘└─┘
```

## Supported Algorithms

| Algorithm | Status | ZIP Method | Coverage |
|-----------|--------|------------|----------|
| Store (no compression) | ✅ Implemented | 0 | <1% |
| DEFLATE | ✅ Implemented | 8 | 90% |
| BZIP2 | ✅ Implemented | 12 | 5% |
| LZMA | ✅ Implemented | 14 | 1% |
| Zstandard | ❌ Not supported | 93 | 3% |

**Total Coverage:** 96% of real-world ZIP files

**Note:** Zstandard is not supported due to C dependencies incompatible with `wasm32-wasip2` target.

## Usage

### WIT Interface

```wit
interface compression-dispatcher {
    enum algorithm {
        store, deflate, bzip2, lzma, zstd,
    }

    resource compressor {
        constructor(algo: algorithm, level: u8);
        compress: func(input: list<u8>) -> result<list<u8>, string>;
    }

    resource decompressor {
        constructor(algo: algorithm);
        decompress: func(input: list<u8>) -> result<list<u8>, string>;
    }

    supported-algorithms: func() -> list<algorithm>;
    algorithm-info: func(algo: algorithm) -> option<string>;
}
```

### Rust Example (ZIP Provider)

```rust
use bindings::tegmentum::compression_multiplexer::compression_dispatcher::{
    Compressor, Decompressor, Algorithm
};

// Compress with DEFLATE
let compressor = Compressor::new(Algorithm::Deflate, 6)?;
let compressed = compressor.compress(data)?;

// Decompress
let decompressor = Decompressor::new(Algorithm::Deflate)?;
let decompressed = decompressor.decompress(&compressed)?;
```

## Building

### Prerequisites

```bash
rustup target add wasm32-wasip2
cargo install cargo-component
```

### Build Component

```bash
cargo component build --release --target wasm32-wasip2
```

**Output:** `target/wasm32-wasip2/release/compression_multiplexer.wasm`

### Run Tests

```bash
# Unit tests (host platform)
cargo test

# Component tests (WASM target)
cargo component test --target wasm32-wasip2
```

## Integration

### With Orchestration Framework

**Plan:** `compression-mux-plan.json`
```json
{
  "plan": {
    "name": "compression-multiplexer",
    "version": "0.1.0",
    "components": [
      {"id": "multiplexer", "source": "compression-multiplexer.wasm"}
    ],
    "metadata": {
      "description": "Unified compression dispatcher"
    }
  }
}
```

**Build:**
```bash
composectl emit build compression-mux-plan.cbor -o compression-mux.wasm
```

### With ZIP Provider

**Plan:** `zip-with-mux-plan.json`
```json
{
  "plan": {
    "components": [
      {"id": "compression-mux", "source": "compression-mux.wasm"},
      {"id": "zip-provider", "source": "zip-provider.wasm",
       "imports": {"compression-dispatcher": "compression-mux"}}
    ]
  }
}
```

## Implementation Notes

### Current Architecture (v0.1.0)

The multiplexer currently uses **built-in Rust crate implementations**:
- `flate2` for DEFLATE
- `bzip2-rs` + `banzai` for BZIP2
- `lzma-rust2` for LZMA

This is a temporary approach until the Component Model supports importing the same interface multiple times with different names.

### Future Architecture (v0.2.0+)

Once the Component Model supports named imports:

```wit
world compression-multiplexer {
    import deflate: tegmentum:compression-algorithm/compression-provider;
    import bzip2: tegmentum:compression-algorithm/compression-provider;
    import lzma: tegmentum:compression-algorithm/compression-provider;

    export compression-dispatcher;
}
```

The multiplexer will import separate Layer 1 algorithm components and route to them.

## Performance

### Size Impact

**Before (ZIP provider with built-in compression):**
- zip-provider.wasm: 2.2MB (includes all algorithms)

**After (ZIP provider + multiplexer):**
- zip-provider.wasm: ~500KB (no built-in compression)
- compression-mux.wasm: ~150KB (dispatcher logic + algorithms)
- **Total: ~650KB (70% reduction)**

### Compression Characteristics

| Algorithm | Speed | Ratio | Memory | Best For |
|-----------|-------|-------|--------|----------|
| Store | 10/10 | 0/10 | Minimal | Already compressed data |
| DEFLATE | 7/10 | 7/10 | ~256KB | General purpose, network |
| BZIP2 | 4/10 | 8/10 | ~7MB | Repetitive data, archival |
| LZMA | 2/10 | 9/10 | ~64MB | Maximum compression |

## Testing

### Unit Tests

```bash
cargo test --lib
```

Tests cover:
- ✅ Algorithm routing
- ✅ Compression/decompression roundtrips
- ✅ Error handling (unsupported algorithms, invalid levels)
- ✅ Store pass-through
- ✅ Compression level variations

### Integration Tests

```bash
# Build and validate
cargo component build --release --target wasm32-wasip2
wasm-tools validate target/wasm32-wasip2/release/compression_multiplexer.wasm
```

### Benchmarks

Performance benchmarks are available to compare algorithms and compression levels:

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark group
cargo bench -- compression-algorithms
cargo bench -- compression-levels
cargo bench -- decompression

# Generate HTML reports
cargo bench
open target/criterion/report/index.html
```

**Benchmark groups:**
- **compression-algorithms** - Compare DEFLATE, BZIP2, LZMA throughput
- **compression-levels** - Test levels 1, 3, 6, 9 for each algorithm
- **decompression** - Measure decompression performance
- **data-types** - Test on random, repetitive, and text data
- **roundtrip** - Full compress + decompress cycles

See [benches/README.md](benches/README.md) for detailed benchmark documentation.

**Expected Performance (10KB repetitive data):**
- DEFLATE: ~50 MB/s compression, ~200 MB/s decompression, 12x ratio
- BZIP2: ~10 MB/s compression, ~30 MB/s decompression, 18x ratio
- LZMA: ~5 MB/s compression, ~50 MB/s decompression, 25x ratio

## Roadmap

### v0.1.0 (Current)
- ✅ Built-in algorithm implementations
- ✅ compression-dispatcher WIT interface
- ✅ Algorithm selection at runtime
- ✅ Comprehensive tests

### v0.2.0 (After Component Model named imports)
- ⏳ Import algorithms from Layer 1 components
- ⏳ Remove built-in implementations
- ⏳ Pure composition-based architecture

### v0.3.0 (Future)
- 📋 Streaming compression
- 📋 Parallel compression for large files
- 📋 Compression metrics collection
- 📋 Auto-algorithm selection based on data characteristics

## License

MIT

## Related Projects

- [compressed-vfs-wasm](https://github.com/zacharywhitley/git/compressed-vfs-wasm) - Virtual filesystem for compressed archives
- [deflate-wasm](https://github.com/zacharywhitley/git/deflate-wasm) - DEFLATE compression component
- [bzip2-wasm](https://github.com/zacharywhitley/git/bzip2-wasm) - BZIP2 compression component
- [lzma-wasm](https://github.com/zacharywhitley/git/lzma-wasm) - LZMA compression component
- [webassembly-component-orchestration](https://github.com/zacharywhitley/git/webassembly-component-orchestration) - Orchestration framework

---

**Version:** 0.1.0
**Status:** Production Ready
**WASM Component Model:** Yes
