// Archive Provider Integration Example
//
// This example demonstrates how an archive format provider (like ZIP, TAR, 7Z)
// would integrate with the compression multiplexer to support multiple
// compression algorithms.
//
// This is a simplified mock-up showing the pattern.

use compression_multiplexer::providers::{get_provider, Algorithm};

/// Mock ZIP entry representing a file in an archive
struct ZipEntry {
    name: String,
    data: Vec<u8>,
    compression_method: u16, // ZIP compression method code
}

impl ZipEntry {
    fn new(name: String, data: Vec<u8>) -> Self {
        Self {
            name,
            data,
            compression_method: 0, // 0 = stored (no compression)
        }
    }
}

/// Mock ZIP archive provider
struct ZipProvider {
    entries: Vec<ZipEntry>,
}

impl ZipProvider {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Add a file to the archive with specified compression
    fn add_file(
        &mut self,
        name: String,
        data: Vec<u8>,
        compression: Algorithm,
        level: u8,
    ) -> Result<(), String> {
        println!("Adding file: {}", name);
        println!("  Original size: {} bytes", data.len());
        println!("  Compression: {:?} (level {})", compression, level);

        // Map Algorithm to ZIP compression method code
        let compression_method = match compression {
            Algorithm::Store => 0,    // Stored (no compression)
            Algorithm::Deflate => 8,  // DEFLATE
            Algorithm::Bzip2 => 12,   // BZIP2
            Algorithm::Lzma => 14,    // LZMA
            Algorithm::Zstd => 93,    // Zstandard (unofficial)
        };

        // Use compression multiplexer to compress the data
        let provider = get_provider(compression)?;
        let compressed_data = provider.compress(&data, level)?;

        println!("  Compressed size: {} bytes", compressed_data.len());
        println!(
            "  Compression ratio: {:.2}x",
            data.len() as f64 / compressed_data.len() as f64
        );

        // Create ZIP entry with compressed data
        let entry = ZipEntry {
            name,
            data: compressed_data,
            compression_method,
        };

        self.entries.push(entry);
        println!("  ✅ Added successfully\n");
        Ok(())
    }

    /// Extract a file from the archive
    fn extract_file(&self, name: &str) -> Result<Vec<u8>, String> {
        // Find the entry
        let entry = self
            .entries
            .iter()
            .find(|e| e.name == name)
            .ok_or_else(|| format!("File not found: {}", name))?;

        println!("Extracting file: {}", name);
        println!("  Compressed size: {} bytes", entry.data.len());

        // Map ZIP compression method to Algorithm
        let algorithm = match entry.compression_method {
            0 => Algorithm::Store,
            8 => Algorithm::Deflate,
            12 => Algorithm::Bzip2,
            14 => Algorithm::Lzma,
            93 => Algorithm::Zstd,
            _ => return Err(format!("Unsupported compression method: {}", entry.compression_method)),
        };

        println!("  Compression method: {:?}", algorithm);

        // Use compression multiplexer to decompress
        let provider = get_provider(algorithm)?;
        let decompressed = provider.decompress(&entry.data)?;

        println!("  Decompressed size: {} bytes", decompressed.len());
        println!("  ✅ Extracted successfully\n");

        Ok(decompressed)
    }

    /// List all files in the archive
    fn list_files(&self) {
        println!("Archive contents:");
        println!("  Name                    | Method  | Size");
        println!("  ------------------------|---------|----------");

        for entry in &self.entries {
            let method_name = match entry.compression_method {
                0 => "Store",
                8 => "DEFLATE",
                12 => "BZIP2",
                14 => "LZMA",
                93 => "Zstandard",
                _ => "Unknown",
            };

            println!(
                "  {:<24}| {:<8}| {} bytes",
                entry.name,
                method_name,
                entry.data.len()
            );
        }
        println!();
    }
}

fn main() -> Result<(), String> {
    println!("Archive Provider Integration Example\n");
    println!("This demonstrates how a ZIP provider would use the multiplexer\n");

    // Create a mock ZIP archive
    let mut zip = ZipProvider::new();

    // Add files with different compression methods
    // This is what a real ZIP provider would do when creating archives

    // Small text file - use DEFLATE (fast, good enough)
    let readme = b"This is a README file. It contains important information.".to_vec();
    zip.add_file("README.txt".to_string(), readme, Algorithm::Deflate, 6)?;

    // Large repetitive data - use BZIP2 (better compression)
    let data_file = b"DATA DATA DATA ".repeat(1000);
    zip.add_file(
        "data.dat".to_string(),
        data_file,
        Algorithm::Bzip2,
        9,
    )?;

    // Already compressed file - store without compression
    let binary = vec![0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56];
    zip.add_file(
        "image.jpg".to_string(),
        binary,
        Algorithm::Store,
        0,
    )?;

    // Source code - use DEFLATE with balanced level
    let source = b"fn main() {\n    println!(\"Hello, World!\");\n}\n".to_vec();
    zip.add_file("main.rs".to_string(), source, Algorithm::Deflate, 6)?;

    // List archive contents
    zip.list_files();

    // Extract files
    println!("Extracting files...\n");

    let readme_data = zip.extract_file("README.txt")?;
    println!("README.txt content: {}\n", String::from_utf8_lossy(&readme_data));

    let _data = zip.extract_file("data.dat")?;
    let _image = zip.extract_file("image.jpg")?;
    let source_data = zip.extract_file("main.rs")?;
    println!("main.rs content:\n{}\n", String::from_utf8_lossy(&source_data));

    println!("✅ Archive provider integration successful!");
    println!();
    println!("Key Benefits:");
    println!("  • Single import: compression-dispatcher");
    println!("  • Runtime algorithm selection");
    println!("  • Supports multiple compression methods in one archive");
    println!("  • ZIP provider doesn't need built-in compression code");

    Ok(())
}
