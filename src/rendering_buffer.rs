//! Rendering buffer — row-oriented access to pixel data.
//!
//! Port of `agg_rendering_buffer.h` — provides two strategies for accessing
//! rows in a rectangular pixel buffer:
//!
//! - [`RowAccessor`]: computes row pointers on demand (multiplication per access).
//!   Cheap to create; good default.
//! - [`RowPtrCache`]: pre-computes and caches row pointers in a `Vec`.
//!   Faster per-row access; requires allocation.
//!
//! Both support positive strides (top-down) and negative strides (bottom-up,
//! e.g. BMP format). The `RenderingBuffer` type alias defaults to `RowAccessor`.

/// Row data returned by `row()` — a slice range and pointer into a row.
#[derive(Debug, Clone, Copy)]
pub struct RowData<'a> {
    pub x1: i32,
    pub x2: i32,
    pub ptr: &'a [u8],
}

// ============================================================================
// RowAccessor
// ============================================================================

/// Row accessor that computes row pointers via base + y * stride.
///
/// Port of C++ `agg::row_accessor<int8u>`.
pub struct RowAccessor {
    buf: *mut u8,
    start: *mut u8,
    width: u32,
    height: u32,
    stride: i32,
}

impl RowAccessor {
    /// Create an empty (unattached) row accessor.
    pub fn new() -> Self {
        Self {
            buf: std::ptr::null_mut(),
            start: std::ptr::null_mut(),
            width: 0,
            height: 0,
            stride: 0,
        }
    }

    /// Create and attach to a buffer.
    ///
    /// # Safety
    /// `buf` must point to a valid buffer of at least `height * stride.abs()` bytes.
    /// The buffer must remain valid for the lifetime of this accessor.
    pub unsafe fn new_with_buf(buf: *mut u8, width: u32, height: u32, stride: i32) -> Self {
        let mut ra = Self::new();
        ra.attach(buf, width, height, stride);
        ra
    }

    /// Attach to a buffer.
    ///
    /// # Safety
    /// Same requirements as `new_with_buf`.
    pub unsafe fn attach(&mut self, buf: *mut u8, width: u32, height: u32, stride: i32) {
        self.buf = buf;
        self.start = buf;
        self.width = width;
        self.height = height;
        self.stride = stride;
        if stride < 0 {
            self.start = buf.offset(-((height as i64 - 1) * stride as i64) as isize);
        }
    }

    /// Raw buffer pointer.
    pub fn buf(&self) -> *mut u8 {
        self.buf
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn stride(&self) -> i32 {
        self.stride
    }

    pub fn stride_abs(&self) -> u32 {
        self.stride.unsigned_abs()
    }

    /// Get a raw mutable pointer to a row.
    ///
    /// # Safety
    /// `y` must be in `[0, height)`.
    #[inline]
    pub unsafe fn row_ptr(&self, y: i32) -> *mut u8 {
        self.start.offset((y as i64 * self.stride as i64) as isize)
    }

    /// Get a safe immutable slice for row `y`.
    ///
    /// Returns the full row of `stride_abs()` bytes.
    pub fn row_slice(&self, y: u32) -> &[u8] {
        assert!(
            y < self.height,
            "row {} out of bounds (height={})",
            y,
            self.height
        );
        unsafe {
            let ptr = self.row_ptr(y as i32);
            std::slice::from_raw_parts(ptr, self.stride_abs() as usize)
        }
    }

    /// Get a safe mutable slice for row `y`.
    pub fn row_slice_mut(&mut self, y: u32) -> &mut [u8] {
        assert!(
            y < self.height,
            "row {} out of bounds (height={})",
            y,
            self.height
        );
        unsafe {
            let ptr = self.row_ptr(y as i32);
            std::slice::from_raw_parts_mut(ptr, self.stride_abs() as usize)
        }
    }

    /// Get row data (x1, x2 range + pointer).
    pub fn row(&self, y: u32) -> RowData<'_> {
        RowData {
            x1: 0,
            x2: self.width as i32 - 1,
            ptr: self.row_slice(y),
        }
    }

    /// Copy pixel data from another buffer (min of both dimensions).
    pub fn copy_from<T: RenderingBufferAccess>(&mut self, src: &T) {
        let h = self.height.min(src.height());
        let l = self.stride_abs().min(src.stride_abs()) as usize;
        for y in 0..h {
            unsafe {
                let dst = self.row_ptr(y as i32);
                let src_ptr = src.row_ptr_const(y as i32);
                std::ptr::copy_nonoverlapping(src_ptr, dst, l);
            }
        }
    }

    /// Fill every byte in the buffer with `value`.
    pub fn clear(&mut self, value: u8) {
        let stride = self.stride_abs() as usize;
        for y in 0..self.height {
            let row = self.row_slice_mut(y);
            for byte in row[..stride].iter_mut() {
                *byte = value;
            }
        }
    }
}

impl Default for RowAccessor {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// RowPtrCache
// ============================================================================

/// Row pointer cache that pre-computes pointers for O(1) row access.
///
/// Port of C++ `agg::row_ptr_cache<int8u>`.
pub struct RowPtrCache {
    buf: *mut u8,
    rows: Vec<*mut u8>,
    width: u32,
    height: u32,
    stride: i32,
}

impl RowPtrCache {
    /// Create an empty (unattached) row pointer cache.
    pub fn new() -> Self {
        Self {
            buf: std::ptr::null_mut(),
            rows: Vec::new(),
            width: 0,
            height: 0,
            stride: 0,
        }
    }

    /// Create and attach to a buffer.
    ///
    /// # Safety
    /// `buf` must point to a valid buffer of at least `height * stride.abs()` bytes.
    /// The buffer must remain valid for the lifetime of this cache.
    pub unsafe fn new_with_buf(buf: *mut u8, width: u32, height: u32, stride: i32) -> Self {
        let mut rpc = Self::new();
        rpc.attach(buf, width, height, stride);
        rpc
    }

    /// Attach to a buffer, building the row pointer cache.
    ///
    /// # Safety
    /// Same requirements as `new_with_buf`.
    pub unsafe fn attach(&mut self, buf: *mut u8, width: u32, height: u32, stride: i32) {
        self.buf = buf;
        self.width = width;
        self.height = height;
        self.stride = stride;

        if (height as usize) > self.rows.len() {
            self.rows.resize(height as usize, std::ptr::null_mut());
        }

        let mut row_ptr = buf;
        if stride < 0 {
            row_ptr = buf.offset(-((height as i64 - 1) * stride as i64) as isize);
        }

        for y in 0..height as usize {
            self.rows[y] = row_ptr;
            row_ptr = row_ptr.offset(stride as isize);
        }
    }

    /// Raw buffer pointer.
    pub fn buf(&self) -> *mut u8 {
        self.buf
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn stride(&self) -> i32 {
        self.stride
    }

    pub fn stride_abs(&self) -> u32 {
        self.stride.unsigned_abs()
    }

    /// Get a raw mutable pointer to a row (O(1) via cached pointers).
    ///
    /// # Safety
    /// `y` must be in `[0, height)`.
    #[inline]
    pub unsafe fn row_ptr(&self, y: i32) -> *mut u8 {
        *self.rows.get_unchecked(y as usize)
    }

    /// Get a safe immutable slice for row `y`.
    pub fn row_slice(&self, y: u32) -> &[u8] {
        assert!(
            y < self.height,
            "row {} out of bounds (height={})",
            y,
            self.height
        );
        unsafe {
            let ptr = self.row_ptr(y as i32);
            std::slice::from_raw_parts(ptr, self.stride_abs() as usize)
        }
    }

    /// Get a safe mutable slice for row `y`.
    pub fn row_slice_mut(&mut self, y: u32) -> &mut [u8] {
        assert!(
            y < self.height,
            "row {} out of bounds (height={})",
            y,
            self.height
        );
        unsafe {
            let ptr = self.row_ptr(y as i32);
            std::slice::from_raw_parts_mut(ptr, self.stride_abs() as usize)
        }
    }

    /// Get row data (x1, x2 range + pointer).
    pub fn row(&self, y: u32) -> RowData<'_> {
        RowData {
            x1: 0,
            x2: self.width as i32 - 1,
            ptr: self.row_slice(y),
        }
    }

    /// Copy pixel data from another buffer.
    pub fn copy_from<T: RenderingBufferAccess>(&mut self, src: &T) {
        let h = self.height.min(src.height());
        let l = self.stride_abs().min(src.stride_abs()) as usize;
        for y in 0..h {
            unsafe {
                let dst = self.row_ptr(y as i32);
                let src_ptr = src.row_ptr_const(y as i32);
                std::ptr::copy_nonoverlapping(src_ptr, dst, l);
            }
        }
    }

    /// Fill every byte in the buffer with `value`.
    pub fn clear(&mut self, value: u8) {
        let stride = self.stride_abs() as usize;
        for y in 0..self.height {
            let row = self.row_slice_mut(y);
            for byte in row[..stride].iter_mut() {
                *byte = value;
            }
        }
    }
}

impl Default for RowPtrCache {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Common trait for both buffer types
// ============================================================================

/// Common interface for rendering buffer access (used by `copy_from`).
pub trait RenderingBufferAccess {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn stride_abs(&self) -> u32;
    /// # Safety
    /// `y` must be in `[0, height)`.
    unsafe fn row_ptr_const(&self, y: i32) -> *const u8;
}

impl RenderingBufferAccess for RowAccessor {
    fn width(&self) -> u32 {
        self.width
    }
    fn height(&self) -> u32 {
        self.height
    }
    fn stride_abs(&self) -> u32 {
        self.stride_abs()
    }
    unsafe fn row_ptr_const(&self, y: i32) -> *const u8 {
        self.row_ptr(y) as *const u8
    }
}

impl RenderingBufferAccess for RowPtrCache {
    fn width(&self) -> u32 {
        self.width
    }
    fn height(&self) -> u32 {
        self.height
    }
    fn stride_abs(&self) -> u32 {
        self.stride_abs()
    }
    unsafe fn row_ptr_const(&self, y: i32) -> *const u8 {
        self.row_ptr(y) as *const u8
    }
}

/// Default rendering buffer type (matches C++ `typedef row_accessor<int8u> rendering_buffer`).
pub type RenderingBuffer = RowAccessor;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_row_accessor_basic() {
        let mut data = vec![0u8; 40]; // 10 wide × 4 high, RGBA
        let rb = unsafe { RowAccessor::new_with_buf(data.as_mut_ptr(), 10, 4, 10) };
        assert_eq!(rb.width(), 10);
        assert_eq!(rb.height(), 4);
        assert_eq!(rb.stride(), 10);
        assert_eq!(rb.stride_abs(), 10);
    }

    #[test]
    fn test_row_accessor_write_read() {
        let mut data = vec![0u8; 30]; // 10 wide × 3 high
        let mut rb = unsafe { RowAccessor::new_with_buf(data.as_mut_ptr(), 10, 3, 10) };

        // Write to row 1
        rb.row_slice_mut(1)[0] = 42;
        rb.row_slice_mut(1)[9] = 99;

        // Read back
        assert_eq!(rb.row_slice(1)[0], 42);
        assert_eq!(rb.row_slice(1)[9], 99);
        assert_eq!(rb.row_slice(0)[0], 0); // row 0 unchanged
    }

    #[test]
    fn test_row_accessor_negative_stride() {
        let mut data = vec![0u8; 30]; // 10 wide × 3 high, bottom-up
                                      // Fill with row indices
        data[0..10].fill(0);
        data[10..20].fill(1);
        data[20..30].fill(2);

        let rb = unsafe { RowAccessor::new_with_buf(data.as_mut_ptr(), 10, 3, -10) };

        // With negative stride, row 0 should point to the LAST 10 bytes
        assert_eq!(rb.row_slice(0)[0], 2);
        assert_eq!(rb.row_slice(1)[0], 1);
        assert_eq!(rb.row_slice(2)[0], 0);
    }

    #[test]
    fn test_row_accessor_clear() {
        let mut data = vec![0u8; 20];
        let mut rb = unsafe { RowAccessor::new_with_buf(data.as_mut_ptr(), 5, 4, 5) };
        rb.clear(0xFF);
        for byte in &data {
            assert_eq!(*byte, 0xFF);
        }
    }

    #[test]
    fn test_row_accessor_row_data() {
        let mut data = vec![0u8; 30];
        data[10] = 55; // row 1, pixel 0
        let rb = unsafe { RowAccessor::new_with_buf(data.as_mut_ptr(), 10, 3, 10) };
        let rd = rb.row(1);
        assert_eq!(rd.x1, 0);
        assert_eq!(rd.x2, 9);
        assert_eq!(rd.ptr[0], 55);
    }

    #[test]
    fn test_row_ptr_cache_basic() {
        let mut data = vec![0u8; 40];
        let rpc = unsafe { RowPtrCache::new_with_buf(data.as_mut_ptr(), 10, 4, 10) };
        assert_eq!(rpc.width(), 10);
        assert_eq!(rpc.height(), 4);
        assert_eq!(rpc.stride(), 10);
    }

    #[test]
    fn test_row_ptr_cache_write_read() {
        let mut data = vec![0u8; 30];
        let mut rpc = unsafe { RowPtrCache::new_with_buf(data.as_mut_ptr(), 10, 3, 10) };

        rpc.row_slice_mut(2)[5] = 77;
        assert_eq!(rpc.row_slice(2)[5], 77);
        assert_eq!(rpc.row_slice(0)[5], 0);
    }

    #[test]
    fn test_row_ptr_cache_negative_stride() {
        let mut data = vec![0u8; 30];
        data[0..10].fill(0);
        data[10..20].fill(1);
        data[20..30].fill(2);

        let rpc = unsafe { RowPtrCache::new_with_buf(data.as_mut_ptr(), 10, 3, -10) };

        assert_eq!(rpc.row_slice(0)[0], 2);
        assert_eq!(rpc.row_slice(1)[0], 1);
        assert_eq!(rpc.row_slice(2)[0], 0);
    }

    #[test]
    fn test_row_ptr_cache_clear() {
        let mut data = vec![0u8; 20];
        let mut rpc = unsafe { RowPtrCache::new_with_buf(data.as_mut_ptr(), 5, 4, 5) };
        rpc.clear(0xAA);
        for byte in &data {
            assert_eq!(*byte, 0xAA);
        }
    }

    #[test]
    fn test_copy_from_accessor_to_accessor() {
        let mut src_data = vec![0u8; 30];
        for (i, byte) in src_data.iter_mut().enumerate() {
            *byte = i as u8;
        }
        let src = unsafe { RowAccessor::new_with_buf(src_data.as_mut_ptr(), 10, 3, 10) };

        let mut dst_data = vec![0u8; 30];
        let mut dst = unsafe { RowAccessor::new_with_buf(dst_data.as_mut_ptr(), 10, 3, 10) };
        dst.copy_from(&src);

        assert_eq!(dst_data, src_data);
    }

    #[test]
    fn test_copy_from_cache_to_cache() {
        let mut src_data = vec![0u8; 30];
        for (i, byte) in src_data.iter_mut().enumerate() {
            *byte = (i * 2) as u8;
        }
        let src = unsafe { RowPtrCache::new_with_buf(src_data.as_mut_ptr(), 10, 3, 10) };

        let mut dst_data = vec![0u8; 30];
        let mut dst = unsafe { RowPtrCache::new_with_buf(dst_data.as_mut_ptr(), 10, 3, 10) };
        dst.copy_from(&src);

        assert_eq!(dst_data, src_data);
    }

    #[test]
    fn test_copy_from_different_sizes() {
        // Source is smaller than destination
        let mut src_data = vec![42u8; 10]; // 5×2
        let src = unsafe { RowAccessor::new_with_buf(src_data.as_mut_ptr(), 5, 2, 5) };

        let mut dst_data = vec![0u8; 30]; // 10×3
        let mut dst = unsafe { RowAccessor::new_with_buf(dst_data.as_mut_ptr(), 10, 3, 10) };
        dst.copy_from(&src);

        // Only the first 5 bytes of the first 2 rows should be copied
        assert_eq!(dst_data[0..5], [42, 42, 42, 42, 42]);
        assert_eq!(dst_data[5..10], [0, 0, 0, 0, 0]);
        assert_eq!(dst_data[10..15], [42, 42, 42, 42, 42]);
        assert_eq!(dst_data[15..20], [0, 0, 0, 0, 0]);
        assert_eq!(dst_data[20..30], [0; 10]); // row 2 untouched
    }

    #[test]
    fn test_row_accessor_default() {
        let rb = RowAccessor::new();
        assert_eq!(rb.width(), 0);
        assert_eq!(rb.height(), 0);
    }

    #[test]
    fn test_row_ptr_cache_default() {
        let rpc = RowPtrCache::new();
        assert_eq!(rpc.width(), 0);
        assert_eq!(rpc.height(), 0);
    }

    #[test]
    fn test_rendering_buffer_alias() {
        // Verify the type alias compiles
        let mut data = vec![0u8; 20];
        let _rb: RenderingBuffer =
            unsafe { RenderingBuffer::new_with_buf(data.as_mut_ptr(), 5, 4, 5) };
    }
}
