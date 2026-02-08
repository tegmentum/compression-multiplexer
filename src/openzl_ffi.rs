//! FFI bindings for OpenZL C library
//!
//! OpenZL uses a Result type (ZL_Report) that's a union containing either:
//! - An error code (when _code != 0)
//! - A success value (when _code == 0)

use std::ffi::{c_int, c_void};

/// ZL_Report - Result type that contains either an error or a size_t value
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ZlReport {
    /// Error code (0 = success, non-zero = error)
    pub code: i32,
    /// Value (only valid when code == 0)
    pub value: usize,
}

impl ZlReport {
    /// Check if this report represents an error
    #[inline]
    pub fn is_error(&self) -> bool {
        self.code != 0
    }

    /// Get the value (assumes no error)
    #[inline]
    pub fn get_value(&self) -> usize {
        debug_assert!(!self.is_error());
        self.value
    }
}

/// Opaque compression context
#[repr(C)]
pub struct ZlCCtx {
    _data: [u8; 0],
    _marker: std::marker::PhantomData<(*mut u8, std::marker::PhantomPinned)>,
}

/// Opaque decompression context
#[repr(C)]
pub struct ZlDCtx {
    _data: [u8; 0],
    _marker: std::marker::PhantomData<(*mut u8, std::marker::PhantomPinned)>,
}

/// Compression parameters
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum ZlCParam {
    StickyParameters = 1,
    CompressionLevel = 2,
    DecompressionLevel = 3,
    FormatVersion = 4,
    PermissiveCompression = 5,
    CompressedChecksum = 6,
    ContentChecksum = 7,
    MinStreamSize = 11,
}

/// Maximum format version
pub const ZL_MAX_FORMAT_VERSION: c_int = 22;

unsafe extern "C" {
    // Compression context management
    pub fn ZL_CCtx_create() -> *mut ZlCCtx;
    pub fn ZL_CCtx_free(cctx: *mut ZlCCtx);
    pub fn ZL_CCtx_setParameter(
        result: *mut ZlReport,
        cctx: *mut ZlCCtx,
        param: ZlCParam,
        value: c_int,
    );
    pub fn ZL_CCtx_compress(
        result: *mut ZlReport,
        cctx: *mut ZlCCtx,
        dst: *mut c_void,
        dst_capacity: usize,
        src: *const c_void,
        src_size: usize,
    );

    // Decompression context management
    pub fn ZL_DCtx_create() -> *mut ZlDCtx;
    pub fn ZL_DCtx_free(dctx: *mut ZlDCtx);
    pub fn ZL_DCtx_decompress(
        result: *mut ZlReport,
        dctx: *mut ZlDCtx,
        dst: *mut c_void,
        dst_capacity: usize,
        src: *const c_void,
        src_size: usize,
    );

    // Utility functions
    pub fn ZL_getDecompressedSize(result: *mut ZlReport, compressed: *const c_void, c_size: usize);
    pub fn ZL_ErrorCode_toString(code: c_int) -> *const i8;
}

/// Calculate the maximum compressed size bound
#[inline]
pub fn zl_compress_bound(src_size: usize) -> usize {
    (src_size * 2) + 512 + 8
}

/// Get the error message for an error code
pub fn get_error_name(code: i32) -> &'static str {
    unsafe {
        let ptr = ZL_ErrorCode_toString(code);
        if ptr.is_null() {
            "Unknown error"
        } else {
            let len = {
                let mut l = 0;
                while *ptr.add(l) != 0 {
                    l += 1;
                }
                l
            };
            let slice = std::slice::from_raw_parts(ptr as *const u8, len);
            std::str::from_utf8_unchecked(slice)
        }
    }
}
