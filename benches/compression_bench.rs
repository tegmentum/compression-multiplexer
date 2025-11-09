use compression_multiplexer::{Algorithm, CompressionProvider};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

// Test data generators
fn generate_random_data(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i % 256) as u8).collect()
}

fn generate_repetitive_data(size: usize) -> Vec<u8> {
    b"The quick brown fox jumps over the lazy dog. "
        .iter()
        .cycle()
        .take(size)
        .copied()
        .collect()
}

fn generate_text_data(size: usize) -> Vec<u8> {
    let text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
                Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. \
                Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris. ";
    text.as_bytes()
        .iter()
        .cycle()
        .take(size)
        .copied()
        .collect()
}

// Get provider function
fn get_provider(algorithm: Algorithm) -> Box<dyn CompressionProvider> {
    compression_multiplexer::providers::get_provider(algorithm).unwrap()
}

fn benchmark_compression_algorithms(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression-algorithms");

    let sizes = vec![1024, 10 * 1024, 100 * 1024]; // 1KB, 10KB, 100KB
    let algorithms = vec![
        (Algorithm::Store, "store"),
        (Algorithm::Deflate, "deflate"),
        (Algorithm::Bzip2, "bzip2"),
        (Algorithm::Lzma, "lzma"),
    ];

    for size in sizes.iter() {
        let data = generate_repetitive_data(*size);
        group.throughput(Throughput::Bytes(*size as u64));

        for (algo, name) in algorithms.iter() {
            let provider = get_provider(*algo);

            group.bench_with_input(
                BenchmarkId::new(format!("{}-compress", name), size),
                &data,
                |b, data| {
                    b.iter(|| {
                        let result = provider.compress(black_box(data), 6);
                        black_box(result)
                    })
                },
            );
        }
    }

    group.finish();
}

fn benchmark_compression_levels(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression-levels");

    let data = generate_repetitive_data(10 * 1024); // 10KB
    group.throughput(Throughput::Bytes(data.len() as u64));

    let algorithms = vec![
        (Algorithm::Deflate, "deflate"),
        (Algorithm::Bzip2, "bzip2"),
        (Algorithm::Lzma, "lzma"),
    ];

    let levels = vec![1, 3, 6, 9];

    for (algo, name) in algorithms.iter() {
        for level in levels.iter() {
            let provider = get_provider(*algo);

            group.bench_with_input(
                BenchmarkId::new(format!("{}-level", name), level),
                &data,
                |b, data| {
                    b.iter(|| {
                        let result = provider.compress(black_box(data), *level);
                        black_box(result)
                    })
                },
            );
        }
    }

    group.finish();
}

fn benchmark_decompression(c: &mut Criterion) {
    let mut group = c.benchmark_group("decompression");

    let data = generate_repetitive_data(10 * 1024); // 10KB
    let algorithms = vec![
        (Algorithm::Store, "store"),
        (Algorithm::Deflate, "deflate"),
        (Algorithm::Bzip2, "bzip2"),
        (Algorithm::Lzma, "lzma"),
    ];

    for (algo, name) in algorithms.iter() {
        let provider = get_provider(*algo);
        let compressed = provider.compress(&data, 6).unwrap();

        group.throughput(Throughput::Bytes(compressed.len() as u64));

        group.bench_with_input(
            BenchmarkId::new(format!("{}-decompress", name), compressed.len()),
            &compressed,
            |b, compressed_data| {
                b.iter(|| {
                    let result = provider.decompress(black_box(compressed_data));
                    black_box(result)
                })
            },
        );
    }

    group.finish();
}

fn benchmark_data_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("data-types");

    let size = 10 * 1024; // 10KB
    let data_types = vec![
        ("random", generate_random_data(size)),
        ("repetitive", generate_repetitive_data(size)),
        ("text", generate_text_data(size)),
    ];

    let algorithms = vec![
        (Algorithm::Deflate, "deflate"),
        (Algorithm::Bzip2, "bzip2"),
        (Algorithm::Lzma, "lzma"),
    ];

    for (data_name, data) in data_types.iter() {
        group.throughput(Throughput::Bytes(data.len() as u64));

        for (algo, algo_name) in algorithms.iter() {
            let provider = get_provider(*algo);

            group.bench_with_input(
                BenchmarkId::new(format!("{}-{}", algo_name, data_name), data.len()),
                data,
                |b, input_data| {
                    b.iter(|| {
                        let result = provider.compress(black_box(input_data), 6);
                        black_box(result)
                    })
                },
            );
        }
    }

    group.finish();
}

fn benchmark_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip");

    let data = generate_repetitive_data(10 * 1024); // 10KB
    group.throughput(Throughput::Bytes(data.len() as u64));

    let algorithms = vec![
        (Algorithm::Deflate, "deflate"),
        (Algorithm::Bzip2, "bzip2"),
        (Algorithm::Lzma, "lzma"),
    ];

    for (algo, name) in algorithms.iter() {
        let provider = get_provider(*algo);

        group.bench_with_input(
            BenchmarkId::new(format!("{}-roundtrip", name), data.len()),
            &data,
            |b, input_data| {
                b.iter(|| {
                    let compressed = provider.compress(black_box(input_data), 6).unwrap();
                    let decompressed = provider.decompress(black_box(&compressed)).unwrap();
                    black_box(decompressed)
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_compression_algorithms,
    benchmark_compression_levels,
    benchmark_decompression,
    benchmark_data_types,
    benchmark_roundtrip
);
criterion_main!(benches);
