// Zstd-extras implementation
//
// Implements the `zstd-extras` WIT interface: dictionary-aware compress/
// decompress and dictionary training. Lives in its own module because
// dictionary semantics don't generalize across the other algorithms;
// trying to fold this into the generic compression-dispatcher would mean
// dead variants for bzip2/lzma/etc.

use crate::bindings::exports::tegmentum::compression_multiplexer::zstd_extras::{
    Guest, GuestZstdDict, ZstdDictBorrow, ZstdParam,
};
use crate::dispatcher::MultiplexerImpl;

// Raw FFI to libzstd for the operations zstd-safe doesn't wrap cleanly:
//   * ZDICT_finalizeDictionary (experimental in zstd-safe, behind a feature)
//   * ZSTD_CCtx_setParameter + ZSTD_compress2 (parameter API; zstd-safe wraps
//     them but via typed enums that don't pass through the Python integer IDs
//     stdlib expects). We use raw FFI to keep the WIT contract "(id, value)
//     pair" tied directly to libzstd's stable C enum values.
use zstd::zstd_safe::zstd_sys;

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
        // Allocate based on the frame's declared content size when available;
        // fall back to a generous over-estimate. A static heuristic (e.g.
        // 20× input.len) can severely under-size when a dictionary lets the
        // frame compress at 100:1+, so reading the header is necessary for
        // correctness, not just efficiency.
        let cap = decompress_capacity(&input);
        decomp
            .decompress(&input, cap)
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

    /// Refine a custom content dictionary with sample statistics
    /// (libzstd's ZDICT_finalizeDictionary). zstd-safe doesn't wrap this
    /// non-experimentally, so we go through the raw FFI.
    fn finalize_dict(
        dict_content: Vec<u8>,
        samples: Vec<Vec<u8>>,
        dict_size: u32,
        level: i32,
    ) -> Result<Vec<u8>, String> {
        if samples.is_empty() {
            return Err("finalize-dict: no samples provided".into());
        }
        let dict_size = dict_size as usize;
        // libzstd requires dict_size >= max(dict_content_len, ZDICT_DICTSIZE_MIN=256).
        if dict_size < dict_content.len() {
            return Err(format!(
                "finalize-dict: dict_size ({}) must be >= dict_content size ({})",
                dict_size, dict_content.len()
            ));
        }
        // Concatenate samples + collect sizes in a parallel Vec.
        let nb_samples = samples.len();
        let total: usize = samples.iter().map(|s| s.len()).sum();
        let mut samples_buf = Vec::with_capacity(total);
        let mut sizes = Vec::with_capacity(nb_samples);
        for s in &samples {
            samples_buf.extend_from_slice(s);
            sizes.push(s.len());
        }
        let mut dst = vec![0u8; dict_size];
        // ZDICT_params_t: compressionLevel, notificationLevel=0, dictID=0 (auto).
        let params = zstd_sys::ZDICT_params_t {
            compressionLevel: level,
            notificationLevel: 0,
            dictID: 0,
        };
        let result = unsafe {
            zstd_sys::ZDICT_finalizeDictionary(
                dst.as_mut_ptr() as *mut core::ffi::c_void,
                dict_size,
                dict_content.as_ptr() as *const core::ffi::c_void,
                dict_content.len(),
                samples_buf.as_ptr() as *const core::ffi::c_void,
                sizes.as_ptr(),
                nb_samples as core::ffi::c_uint,
                params,
            )
        };
        // ZDICT_isError(): top bits set indicate error; the magic check matches
        // the C macro `if (ZDICT_isError(rc)) ...`. (Actually it's
        // `code > -ZSTD_error_maxCode` after casting to ssize_t, but the
        // simpler test that always works is the official ZDICT_isError function.)
        let is_err = unsafe { zstd_sys::ZDICT_isError(result) };
        if is_err != 0 {
            let name_ptr = unsafe { zstd_sys::ZDICT_getErrorName(result) };
            let name = if name_ptr.is_null() {
                "unknown".to_string()
            } else {
                unsafe { core::ffi::CStr::from_ptr(name_ptr) }
                    .to_string_lossy()
                    .into_owned()
            };
            return Err(format!("finalize-dict failed: {}", name));
        }
        dst.truncate(result);
        Ok(dst)
    }

    /// libzstd's ZSTD_findFrameCompressedSize — size of one frame in bytes.
    fn get_frame_size(frame: Vec<u8>) -> Result<u64, String> {
        zstd::zstd_safe::find_frame_compressed_size(&frame)
            .map_err(|code| format!("find_frame_compressed_size error code: {}", code))
            .map(|sz| sz as u64)
    }

    /// Advanced compress: build a CCtx, push each (id, value) param,
    /// then ZSTD_compress2. Level applied last so it wins over a same-named
    /// param entry.
    fn compress_advanced(
        input: Vec<u8>,
        level: i32,
        params: Vec<ZstdParam>,
    ) -> Result<Vec<u8>, String> {
        unsafe {
            let cctx = zstd_sys::ZSTD_createCCtx();
            if cctx.is_null() {
                return Err("compress-advanced: ZSTD_createCCtx returned null".into());
            }
            // RAII via a scope guard pattern (manual free on every return).
            let result = (|| -> Result<Vec<u8>, String> {
                for p in &params {
                    let rc = zstd_sys::ZSTD_CCtx_setParameter(
                        cctx,
                        core::mem::transmute(p.id),
                        p.value,
                    );
                    if zstd_sys::ZSTD_isError(rc) != 0 {
                        return Err(format!(
                            "compress-advanced: setParameter(id={}, value={}) failed: {}",
                            p.id, p.value, zstd_error_name(rc)
                        ));
                    }
                }
                // Level applied last so it overrides any same-named entry.
                let rc = zstd_sys::ZSTD_CCtx_setParameter(
                    cctx,
                    zstd_sys::ZSTD_cParameter::ZSTD_c_compressionLevel,
                    level,
                );
                if zstd_sys::ZSTD_isError(rc) != 0 {
                    return Err(format!(
                        "compress-advanced: setParameter(compressionLevel={}) failed: {}",
                        level, zstd_error_name(rc)
                    ));
                }
                let bound = zstd_sys::ZSTD_compressBound(input.len());
                let mut dst = vec![0u8; bound];
                let written = zstd_sys::ZSTD_compress2(
                    cctx,
                    dst.as_mut_ptr() as *mut core::ffi::c_void,
                    dst.len(),
                    input.as_ptr() as *const core::ffi::c_void,
                    input.len(),
                );
                if zstd_sys::ZSTD_isError(written) != 0 {
                    return Err(format!(
                        "compress-advanced: ZSTD_compress2 failed: {}",
                        zstd_error_name(written)
                    ));
                }
                dst.truncate(written);
                Ok(dst)
            })();
            zstd_sys::ZSTD_freeCCtx(cctx);
            result
        }
    }

    /// Advanced compress combined with a dictionary loaded into the CCtx.
    /// Delegates to the shared `compress_advanced_with_dict_impl` below.
    fn compress_advanced_with_dict(
        input: Vec<u8>,
        level: i32,
        params: Vec<ZstdParam>,
        dict: ZstdDictBorrow<'_>,
    ) -> Result<Vec<u8>, String> {
        let dict_ref: &ZstdDict = dict.get();
        MultiplexerImpl::compress_advanced_with_dict_impl(
            input, level, params, &dict_ref.bytes)
    }

    /// Advanced decompress combined with a dictionary loaded into the DCtx.
    fn decompress_advanced_with_dict(
        input: Vec<u8>,
        params: Vec<ZstdParam>,
        dict: ZstdDictBorrow<'_>,
    ) -> Result<Vec<u8>, String> {
        let dict_ref: &ZstdDict = dict.get();
        MultiplexerImpl::decompress_advanced_with_dict_impl(
            input, params, &dict_ref.bytes)
    }

    /// Advanced decompress: build a DCtx, push each param, then
    /// ZSTD_decompressDCtx. The DCtx is required (not the streaming one)
    /// because parameters are per-context.
    fn decompress_advanced(
        input: Vec<u8>,
        params: Vec<ZstdParam>,
    ) -> Result<Vec<u8>, String> {
        unsafe {
            let dctx = zstd_sys::ZSTD_createDCtx();
            if dctx.is_null() {
                return Err("decompress-advanced: ZSTD_createDCtx returned null".into());
            }
            let result = (|| -> Result<Vec<u8>, String> {
                for p in &params {
                    let rc = zstd_sys::ZSTD_DCtx_setParameter(
                        dctx,
                        core::mem::transmute(p.id),
                        p.value,
                    );
                    if zstd_sys::ZSTD_isError(rc) != 0 {
                        return Err(format!(
                            "decompress-advanced: setParameter(id={}, value={}) failed: {}",
                            p.id, p.value, zstd_error_name(rc)
                        ));
                    }
                }
                let mut dst = vec![0u8; decompress_capacity(&input)];
                let written = zstd_sys::ZSTD_decompressDCtx(
                    dctx,
                    dst.as_mut_ptr() as *mut core::ffi::c_void,
                    dst.len(),
                    input.as_ptr() as *const core::ffi::c_void,
                    input.len(),
                );
                if zstd_sys::ZSTD_isError(written) != 0 {
                    return Err(format!(
                        "decompress-advanced: ZSTD_decompressDCtx failed: {}",
                        zstd_error_name(written)
                    ));
                }
                dst.truncate(written);
                Ok(dst)
            })();
            zstd_sys::ZSTD_freeDCtx(dctx);
            result
        }
    }
}

/// Pick an output buffer size for one-shot decompress.
///
/// libzstd's `ZSTD_getFrameContentSize` reads the frame header. Three
/// outcomes:
///   * Known size: use exactly that (truncated to a 64 MB ceiling to keep
///     malicious inputs from triggering huge allocations).
///   * ZSTD_CONTENTSIZE_UNKNOWN (u64::MAX): encoder didn't embed FCS;
///     fall back to a generous 20×input.len estimate with a 64 KB floor.
///   * ZSTD_CONTENTSIZE_ERROR (u64::MAX-1): malformed header; return a
///     small default so the actual decompress() call raises a clean error
///     instead of allocating gigabytes.
fn decompress_capacity(input: &[u8]) -> usize {
    let known = unsafe {
        zstd_sys::ZSTD_getFrameContentSize(
            input.as_ptr() as *const core::ffi::c_void,
            input.len(),
        )
    };
    const CONTENTSIZE_ERROR: u64 = u64::MAX - 1;
    const CAP: usize = 64 * 1024 * 1024;
    if known == u64::MAX {
        (input.len().saturating_mul(20)).min(CAP).max(64 * 1024)
    } else if known == CONTENTSIZE_ERROR {
        4096
    } else {
        (known as usize).min(CAP)
    }
}

/// Lift a libzstd error code into a printable name.
fn zstd_error_name(code: usize) -> String {
    unsafe {
        let name_ptr = zstd_sys::ZSTD_getErrorName(code);
        if name_ptr.is_null() {
            format!("unknown error code {}", code)
        } else {
            core::ffi::CStr::from_ptr(name_ptr)
                .to_string_lossy()
                .into_owned()
        }
    }
}

/// Shared CCtx setup: apply each (id, value) param, then apply `level`
/// (which always wins over a same-id entry in `params`). Returns the
/// first error code encountered, or 0 on success.
unsafe fn apply_cctx_params(
    cctx: *mut zstd_sys::ZSTD_CCtx,
    level: i32,
    params: &[ZstdParam],
) -> Result<(), String> {
    for p in params {
        let rc = zstd_sys::ZSTD_CCtx_setParameter(
            cctx,
            core::mem::transmute(p.id),
            p.value,
        );
        if zstd_sys::ZSTD_isError(rc) != 0 {
            return Err(format!(
                "setParameter(id={}, value={}) failed: {}",
                p.id, p.value, zstd_error_name(rc)
            ));
        }
    }
    let rc = zstd_sys::ZSTD_CCtx_setParameter(
        cctx,
        zstd_sys::ZSTD_cParameter::ZSTD_c_compressionLevel,
        level,
    );
    if zstd_sys::ZSTD_isError(rc) != 0 {
        return Err(format!(
            "setParameter(compressionLevel={}) failed: {}",
            level, zstd_error_name(rc)
        ));
    }
    Ok(())
}

unsafe fn apply_dctx_params(
    dctx: *mut zstd_sys::ZSTD_DCtx,
    params: &[ZstdParam],
) -> Result<(), String> {
    for p in params {
        let rc = zstd_sys::ZSTD_DCtx_setParameter(
            dctx,
            core::mem::transmute(p.id),
            p.value,
        );
        if zstd_sys::ZSTD_isError(rc) != 0 {
            return Err(format!(
                "setParameter(id={}, value={}) failed: {}",
                p.id, p.value, zstd_error_name(rc)
            ));
        }
    }
    Ok(())
}

/// Compress with both advanced parameters AND a dictionary loaded into
/// the CCtx (libzstd's ZSTD_CCtx_loadDictionary). Sits in the same
/// trait impl block as the other Guest methods.
impl MultiplexerImpl {
    fn compress_advanced_with_dict_impl(
        input: Vec<u8>,
        level: i32,
        params: Vec<ZstdParam>,
        dict_bytes: &[u8],
    ) -> Result<Vec<u8>, String> {
        unsafe {
            let cctx = zstd_sys::ZSTD_createCCtx();
            if cctx.is_null() {
                return Err("ZSTD_createCCtx returned null".into());
            }
            let result = (|| -> Result<Vec<u8>, String> {
                apply_cctx_params(cctx, level, &params)
                    .map_err(|e| format!("compress-advanced-with-dict: {}", e))?;
                let rc = zstd_sys::ZSTD_CCtx_loadDictionary(
                    cctx,
                    dict_bytes.as_ptr() as *const core::ffi::c_void,
                    dict_bytes.len(),
                );
                if zstd_sys::ZSTD_isError(rc) != 0 {
                    return Err(format!(
                        "compress-advanced-with-dict: loadDictionary failed: {}",
                        zstd_error_name(rc)
                    ));
                }
                let bound = zstd_sys::ZSTD_compressBound(input.len());
                let mut dst = vec![0u8; bound];
                let written = zstd_sys::ZSTD_compress2(
                    cctx,
                    dst.as_mut_ptr() as *mut core::ffi::c_void,
                    dst.len(),
                    input.as_ptr() as *const core::ffi::c_void,
                    input.len(),
                );
                if zstd_sys::ZSTD_isError(written) != 0 {
                    return Err(format!(
                        "compress-advanced-with-dict: ZSTD_compress2 failed: {}",
                        zstd_error_name(written)
                    ));
                }
                dst.truncate(written);
                Ok(dst)
            })();
            zstd_sys::ZSTD_freeCCtx(cctx);
            result
        }
    }

    fn decompress_advanced_with_dict_impl(
        input: Vec<u8>,
        params: Vec<ZstdParam>,
        dict_bytes: &[u8],
    ) -> Result<Vec<u8>, String> {
        unsafe {
            let dctx = zstd_sys::ZSTD_createDCtx();
            if dctx.is_null() {
                return Err("ZSTD_createDCtx returned null".into());
            }
            let result = (|| -> Result<Vec<u8>, String> {
                apply_dctx_params(dctx, &params)
                    .map_err(|e| format!("decompress-advanced-with-dict: {}", e))?;
                let rc = zstd_sys::ZSTD_DCtx_loadDictionary(
                    dctx,
                    dict_bytes.as_ptr() as *const core::ffi::c_void,
                    dict_bytes.len(),
                );
                if zstd_sys::ZSTD_isError(rc) != 0 {
                    return Err(format!(
                        "decompress-advanced-with-dict: loadDictionary failed: {}",
                        zstd_error_name(rc)
                    ));
                }
                let mut dst = vec![0u8; decompress_capacity(&input)];
                let written = zstd_sys::ZSTD_decompressDCtx(
                    dctx,
                    dst.as_mut_ptr() as *mut core::ffi::c_void,
                    dst.len(),
                    input.as_ptr() as *const core::ffi::c_void,
                    input.len(),
                );
                if zstd_sys::ZSTD_isError(written) != 0 {
                    return Err(format!(
                        "decompress-advanced-with-dict: ZSTD_decompressDCtx failed: {}",
                        zstd_error_name(written)
                    ));
                }
                dst.truncate(written);
                Ok(dst)
            })();
            zstd_sys::ZSTD_freeDCtx(dctx);
            result
        }
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
