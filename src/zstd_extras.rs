// Zstd-extras implementation
//
// Implements the `zstd-extras` WIT interface: dictionary-aware compress/
// decompress and dictionary training. Lives in its own module because
// dictionary semantics don't generalize across the other algorithms;
// trying to fold this into the generic compression-dispatcher would mean
// dead variants for bzip2/lzma/etc.

use crate::bindings::exports::tegmentum::compression_multiplexer::zstd_extras::{
    Guest, GuestZstdDict, ZstdDictBorrow,
};
use crate::dispatcher::MultiplexerImpl;

/// A zstd dictionary, wrapping the raw bytes. We keep them owned (rather
/// than caching a prepared CDict/DDict pair) because the WIT resource has
/// to be `'static`-safe and zstd's prepared dicts hold borrows into the
/// raw bytes; storing a self-referential struct would require unsafe or
/// owning_ref. Re-preparing the dict per call is fast (~µs); the win from
/// prepared dicts is mostly noticeable when you compress the same payload
/// 1000s of times, which isn't the python-shim use case.
pub struct ZstdDict {
    bytes: Vec<u8>,
}

impl GuestZstdDict for ZstdDict {
    fn new(bytes: Vec<u8>) -> Self {
        ZstdDict { bytes }
    }

    fn id(&self) -> u32 {
        // libzstd reads the dictID from the dictionary header. `zstd_safe`
        // (re-exported by the `zstd` crate) provides `get_dict_id_from_dict`;
        // returns 0 for raw-content dicts that don't embed an ID.
        zstd::zstd_safe::get_dict_id_from_dict(&self.bytes)
            .map(|nz| nz.get())
            .unwrap_or(0)
    }

    fn as_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }
}

impl Guest for MultiplexerImpl {
    type ZstdDict = ZstdDict;

    fn compress_with_dict(
        input: Vec<u8>,
        dict: ZstdDictBorrow<'_>,
        level: i32,
    ) -> Result<Vec<u8>, String> {
        let dict_ref: &ZstdDict = dict.get();
        let mut comp = zstd::bulk::Compressor::with_dictionary(level, &dict_ref.bytes)
            .map_err(|e| format!("zstd compressor (with-dict) init failed: {}", e))?;
        comp.compress(&input)
            .map_err(|e| format!("zstd compress-with-dict failed: {}", e))
    }

    fn decompress_with_dict(
        input: Vec<u8>,
        dict: ZstdDictBorrow<'_>,
    ) -> Result<Vec<u8>, String> {
        let dict_ref: &ZstdDict = dict.get();
        let mut decomp = zstd::bulk::Decompressor::with_dictionary(&dict_ref.bytes)
            .map_err(|e| format!("zstd decompressor (with-dict) init failed: {}", e))?;
        // bulk::Decompressor::decompress(src, capacity) — we don't know the
        // uncompressed size in advance for arbitrary frames, so over-estimate
        // generously. The crate truncates the output Vec to the actual size.
        // Cap at 64 MB to bound runaway allocations from malicious input.
        let estimated = (input.len().saturating_mul(20)).min(64 * 1024 * 1024).max(4096);
        decomp
            .decompress(&input, estimated)
            .map_err(|e| format!("zstd decompress-with-dict failed: {}", e))
    }

    fn train_dict(
        samples: Vec<Vec<u8>>,
        dict_size: u32,
    ) -> Result<Vec<u8>, String> {
        if samples.is_empty() {
            return Err("train-dict: no samples provided".into());
        }
        zstd::dict::from_samples(&samples, dict_size as usize)
            .map_err(|e| format!("zstd train-dict failed: {}", e))
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dict_roundtrip() {
        // Train a dict on synthetic samples sharing structure.
        let samples: Vec<Vec<u8>> = (0..20)
            .map(|i| format!("{{\"id\":{},\"name\":\"sample-{}\"}}", i, i).into_bytes())
            .collect();
        let dict_bytes = MultiplexerImpl::train_dict(samples, 4096)
            .expect("training should succeed");
        assert!(!dict_bytes.is_empty(), "dict should not be empty");

        let dict = ZstdDict::new(dict_bytes);
        let dict_id = dict.id();
        assert!(dict_id != 0, "trained dict should have a non-zero ID");

        let payload = br#"{"id":99,"name":"sample-99"}"#.to_vec();
        // Re-wrap so we have a ZstdDictBorrow-compatible call path
        // (the public API takes a borrow, but in unit tests we have the
        // owned ZstdDict directly; exercise the with_dictionary path on
        // zstd::bulk directly instead).
        let mut comp = zstd::bulk::Compressor::with_dictionary(3, dict.as_bytes().as_slice())
            .expect("compressor with dict");
        let compressed = comp.compress(&payload).expect("compress");

        let mut decomp = zstd::bulk::Decompressor::with_dictionary(dict.as_bytes().as_slice())
            .expect("decompressor with dict");
        let decompressed = decomp.decompress(&compressed, 1024).expect("decompress");
        assert_eq!(decompressed, payload);
    }
}
