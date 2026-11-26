//! Rendering buffer — row-oriented access to pixel data.
//!
//! Port of `agg_rendering_buffer.h` — provides two strategies for accessing
//! rows in a rectangular pixel buffer the caller owns and lends as a `&mut [u8]`:
//!
//! - [`RowAccessor`]: computes a row's offset on demand (one multiply-add per access).
//!   Cheap to create; good default.
//! - [`RowPtrCache`]: pre-computes and caches every row's offset in a `Vec`.
//!   One table lookup per row access; requires allocation.
//!
//! Both support positive strides (top-down) and negative strides (bottom-up,
//! e.g. BMP format). The `RenderingBuffer` type alias defaults to `RowAccessor`.
//!
//! Both borrow the caller's bytes for their lifetime `'a`, so attaching is a safe fn: it
//! checks that the slice holds `height * |stride|` bytes and panics otherwise, and every row
//! access afterwards is an ordinary checked slice of those bytes. C++ AGG attaches a raw
//! pointer and trusts its size.

/// Row data returned by `row()` — a slice range and pointer into a row.
#[derive(Debug, Clone, Copy)]
pub struct RowData<'a> {
    pub x1: i32,
    pub x2: i32,
    pub ptr: &'a [u8],
}

// ============================================================================
// Attach checks, shared by both buffer types
// ============================================================================

/// Checks the geometry `attach` is given against `buf` and returns `height * |stride|`, the
/// number of bytes the attached rows span.
///
/// # Panics
/// When `width` or `height` exceeds `i32::MAX` (see [`check_dimensions`]), or when `buf` is
/// shorter than `height * |stride|`. The product is computed in `u64`, where it cannot
/// overflow (both factors are below `2^32`), so on a 32-bit target a product past
/// `usize::MAX` is reported as too long for any buffer rather than wrapping to a small size.
#[inline]
fn attached_len(buf_len: usize, width: u32, height: u32, stride: i32) -> usize {
    check_dimensions(width, height);
    let needed = height as u64 * stride.unsigned_abs() as u64;
    if needed > buf_len as u64 {
        buffer_too_short(buf_len, height, stride, needed);
    }
    // `needed <= buf_len`, so it fits in `usize`.
    needed as usize
}

#[cold]
#[inline(never)]
fn buffer_too_short(buf_len: usize, height: u32, stride: i32, needed: u64) -> ! {
    panic!(
        "rendering buffer of {buf_len} bytes is shorter than {height} rows of stride {stride} ({needed} bytes)"
    );
}

/// Panics unless `width` and `height` fit in `i32`, the type AGG addresses rows and columns
/// with: a row `>= 2^31` would turn negative as an `i32` row index.
#[inline]
fn check_dimensions(width: u32, height: u32) {
    if width > i32::MAX as u32 || height > i32::MAX as u32 {
        dimensions_too_big(width, height);
    }
}

#[cold]
#[inline(never)]
fn dimensions_too_big(width: u32, height: u32) -> ! {
    if height > i32::MAX as u32 {
        panic!("rendering buffer height {height} exceeds i32::MAX");
    }
    panic!("rendering buffer width {width} exceeds i32::MAX");
}

/// Offset in the attached bytes of row 0: the first row in memory for a positive stride, the
/// last one for a negative stride. `len` is `height * |stride|` (`attached_len`); with no rows
/// it is 0, and so is the offset.
#[inline]
fn row0_offset(len: usize, stride: i32) -> usize {
    if stride < 0 && len > 0 {
        len - stride.unsigned_abs() as usize
    } else {
        0
    }
}

#[cold]
#[inline(never)]
fn row_index_out_of_bounds(y: u32, height: u32) -> ! {
    panic!("row {y} out of bounds (height={height})");
}

// ============================================================================
// RowAccessor
// ============================================================================

/// Row accessor over a borrowed pixel buffer: row `y` starts at byte `row0 + y * stride` of
/// the attached slice.
///
/// Port of C++ `agg::row_accessor<int8u>`. The bytes are borrowed for `'a`, so the caller
/// gets its buffer back once the accessor (and every pixel format or view built on it) is
/// dropped. [`reborrow`](Self::reborrow) lends the same rows for a shorter lifetime.
pub struct RowAccessor<'a> {
    /// Exactly the attached bytes, `height * |stride|` of them; empty when unattached.
    buf: &'a mut [u8],
    /// Offset of row 0 in `buf` (see `row0_offset`), so a row's offset is one signed
    /// multiply-add whatever the stride's sign.
    row0: usize,
    width: u32,
    height: u32,
    stride: i32,
}

impl<'a> RowAccessor<'a> {
    /// Create an empty (unattached) row accessor: no bytes, width, height and stride 0.
    pub fn new() -> Self {
        Self {
            buf: Default::default(),
            row0: 0,
            width: 0,
            height: 0,
            stride: 0,
        }
    }

    /// Create an accessor attached to `buf`, `height` rows of `width` pixels, `stride` bytes
    /// apart. With a negative stride row 0 is the last row in memory.
    ///
    /// Only the first `height * |stride|` bytes of `buf` are attached; a longer slice is
    /// accepted and its remaining bytes are never touched.
    ///
    /// # Panics
    /// When `width` or `height` exceeds `i32::MAX` (AGG addresses rows and columns as `i32`,
    /// so a larger buffer would have rows no `i32` can name), or when `buf` holds fewer than
    /// `height * |stride|` bytes.
    pub fn new_with_buf(buf: &'a mut [u8], width: u32, height: u32, stride: i32) -> Self {
        let mut ra = Self::new();
        ra.attach(buf, width, height, stride);
        ra
    }

    /// Attach to `buf`; same geometry and checks as [`new_with_buf`](Self::new_with_buf).
    ///
    /// # Panics
    /// As `new_with_buf`, before anything is stored, so on a panic the accessor keeps its
    /// previous attachment.
    pub fn attach(&mut self, buf: &'a mut [u8], width: u32, height: u32, stride: i32) {
        let len = attached_len(buf.len(), width, height, stride);
        self.buf = &mut buf[..len];
        self.row0 = row0_offset(len, stride);
        self.width = width;
        self.height = height;
        self.stride = stride;
    }

    /// A shorter-lived accessor over the same bytes with the same geometry (what `&mut *`
    /// is for a reference). This accessor is usable again once the returned one is dropped.
    #[inline]
    pub fn reborrow(&mut self) -> RowAccessor<'_> {
        RowAccessor {
            buf: &mut *self.buf,
            row0: self.row0,
            width: self.width,
            height: self.height,
            stride: self.stride,
        }
    }

    /// The attached bytes, `height * |stride|` of them, in memory order (with a negative
    /// stride row 0 is the last row).
    pub fn buf(&self) -> &[u8] {
        self.buf
    }

    /// The attached bytes, writable; see [`buf`](Self::buf).
    pub fn buf_mut(&mut self) -> &mut [u8] {
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

    /// Offset in the attached bytes of row `y`, which the caller has checked is in
    /// `[0, height)`. For such a row the offset is in `[0, len - |stride|]`: with a positive
    /// stride it is `y * stride`, with a negative one `(height - 1 - y) * |stride|`, so the
    /// signed arithmetic cannot overflow (`len <= isize::MAX`).
    #[inline]
    fn row_offset(&self, y: i32) -> usize {
        (self.row0 as isize + y as isize * self.stride as isize) as usize
    }

    /// The first `len` bytes of row `y`: the checked row access every pixel format and
    /// `ChannelView` reads through.
    ///
    /// # Panics
    /// Unless `0 <= y < height` and `len <= stride_abs()`. An unattached accessor has
    /// height 0, so every `y` panics.
    ///
    /// Deliberate divergence from C++: AGG's `pixfmt` methods do no bounds checking and rely
    /// on `renderer_base` clipping, so an out-of-range `y` (or a stride shorter than
    /// `width * bytes-per-pixel`) is undefined behaviour there. In Rust the `PixelFormat`
    /// methods are safe fns, so an out-of-range row panics, the same way an out-of-range `x`
    /// already panics through slice indexing. Locked in by `tests/pixfmt_row_bounds_tests.rs`.
    #[inline]
    pub(crate) fn row_bytes(&self, y: i32, len: usize) -> &[u8] {
        self.check_row(y, len);
        let off = self.row_offset(y);
        &self.buf[off..off + len]
    }

    /// The first `len` bytes of row `y`, writable. Same checks and divergence as
    /// [`row_bytes`](Self::row_bytes).
    #[inline]
    pub(crate) fn row_bytes_mut(&mut self, y: i32, len: usize) -> &mut [u8] {
        self.check_row(y, len);
        let off = self.row_offset(y);
        &mut self.buf[off..off + len]
    }

    /// Panics unless row `y`'s first `len` bytes lie inside the attached buffer, with the
    /// messages the pixel formats' bounds tests pin. `y < 0` is rejected on its own: `y as u32`
    /// would wrap it to a huge row.
    #[inline]
    fn check_row(&self, y: i32, len: usize) {
        if !self.row_fits(y, len) {
            row_out_of_bounds(y, len, self.height, self.stride);
        }
    }

    /// Whether row `y`'s first `len` bytes lie inside the attached buffer, so
    /// [`row_bytes`](Self::row_bytes) returns them without panicking. For a caller with a
    /// fallback (the image accessors' fast path).
    #[inline]
    pub(crate) fn row_fits(&self, y: i32, len: usize) -> bool {
        y >= 0 && (y as u32) < self.height && len <= self.stride_abs() as usize
    }

    /// Row `y`, all `stride_abs()` bytes of it.
    ///
    /// # Panics
    /// Unless `y < height`.
    #[inline]
    pub fn row_slice(&self, y: u32) -> &[u8] {
        if y >= self.height {
            row_index_out_of_bounds(y, self.height);
        }
        // `y < height <= i32::MAX`, so `y as i32` is the same row.
        let off = self.row_offset(y as i32);
        &self.buf[off..off + self.stride_abs() as usize]
    }

    /// Row `y`, writable; see [`row_slice`](Self::row_slice).
    #[inline]
    pub fn row_slice_mut(&mut self, y: u32) -> &mut [u8] {
        if y >= self.height {
            row_index_out_of_bounds(y, self.height);
        }
        let off = self.row_offset(y as i32);
        let len = self.stride_abs() as usize;
        &mut self.buf[off..off + len]
    }

    /// Get row data (x1, x2 range + the row's bytes).
    pub fn row(&self, y: u32) -> RowData<'_> {
        RowData {
            x1: 0,
            x2: self.width as i32 - 1,
            ptr: self.row_slice(y),
        }
    }

    /// Copy pixel data from another buffer (min of both dimensions).
    ///
    /// Copies rows `0..min(self.height(), src.height())`, and of each row its first
    /// `min(self.stride_abs(), src.stride_abs())` bytes; row `y` of `src` goes to row `y` of
    /// `self`, whatever the sign of either stride.
    ///
    /// # Panics
    /// When `src` breaks the [`RenderingBufferAccess`] contract (a `row_slice` that panics for
    /// a row below its `height()`, or returns fewer than its `stride_abs()` bytes).
    pub fn copy_from<T: RenderingBufferAccess>(&mut self, src: &T) {
        let h = self.height.min(src.height());
        let l = self.stride_abs().min(src.stride_abs()) as usize;
        for y in 0..h {
            let src_row = &src.row_slice(y)[..l];
            self.row_slice_mut(y)[..l].copy_from_slice(src_row);
        }
    }

    /// Fill every attached byte with `value`.
    pub fn clear(&mut self, value: u8) {
        self.buf.fill(value);
    }
}

/// The panic behind `RowAccessor::check_row`, kept out of line so the formatting code is not
/// inlined into every pixel access.
#[cold]
#[inline(never)]
fn row_out_of_bounds(y: i32, len: usize, height: u32, stride: i32) -> ! {
    if y < 0 || y as u32 >= height {
        panic!("row {y} out of bounds (height={height})");
    }
    panic!("row of {len} bytes does not fit in stride {stride}");
}

impl Default for RowAccessor<'_> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// RowPtrCache
// ============================================================================

/// Row cache over a borrowed pixel buffer: the offset of every row in the attached slice is
/// computed once, at attach, so a row access is a table lookup.
///
/// Port of C++ `agg::row_ptr_cache<int8u>`, which caches row pointers; this caches offsets
/// into the borrowed slice instead.
pub struct RowPtrCache<'a> {
    /// Exactly the attached bytes, `height * |stride|` of them; empty when unattached.
    buf: &'a mut [u8],
    /// `rows[y]` is the offset of row `y` in `buf`; `height` entries.
    rows: Vec<usize>,
    width: u32,
    height: u32,
    stride: i32,
}

impl<'a> RowPtrCache<'a> {
    /// Create an empty (unattached) row cache: no bytes, width, height and stride 0.
    pub fn new() -> Self {
        Self {
            buf: Default::default(),
            rows: Vec::new(),
            width: 0,
            height: 0,
            stride: 0,
        }
    }

    /// Create a cache attached to `buf`, `height` rows of `width` pixels, `stride` bytes
    /// apart. With a negative stride row 0 is the last row in memory.
    ///
    /// Only the first `height * |stride|` bytes of `buf` are attached; a longer slice is
    /// accepted and its remaining bytes are never touched.
    ///
    /// # Panics
    /// When `width` or `height` exceeds `i32::MAX` (AGG addresses rows and columns as `i32`,
    /// so a larger buffer would have rows no `i32` can name), or when `buf` holds fewer than
    /// `height * |stride|` bytes.
    pub fn new_with_buf(buf: &'a mut [u8], width: u32, height: u32, stride: i32) -> Self {
        let mut rpc = Self::new();
        rpc.attach(buf, width, height, stride);
        rpc
    }

    /// Attach to `buf`, building the row offset cache; same geometry and checks as
    /// [`new_with_buf`](Self::new_with_buf). The cache's allocation is reused.
    ///
    /// # Panics
    /// As `new_with_buf`, before anything is stored or allocated, so on a panic the cache
    /// keeps its previous attachment.
    pub fn attach(&mut self, buf: &'a mut [u8], width: u32, height: u32, stride: i32) {
        let len = attached_len(buf.len(), width, height, stride);
        self.buf = &mut buf[..len];
        self.width = width;
        self.height = height;
        self.stride = stride;

        // Bottom-up buffers keep row 0 last in memory. Each offset is derived from the row
        // index, so it is at most `(height - 1) * |stride|`, inside the attached bytes.
        let stride_abs = stride.unsigned_abs() as usize;
        let height = height as usize;
        self.rows.clear();
        self.rows.extend((0..height).map(|y| {
            let mem_row = if stride < 0 { height - 1 - y } else { y };
            mem_row * stride_abs
        }));
    }

    /// The attached bytes, `height * |stride|` of them, in memory order (with a negative
    /// stride row 0 is the last row).
    pub fn buf(&self) -> &[u8] {
        self.buf
    }

    /// The attached bytes, writable; see [`buf`](Self::buf).
    pub fn buf_mut(&mut self) -> &mut [u8] {
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

    /// Row `y`, all `stride_abs()` bytes of it.
    ///
    /// # Panics
    /// Unless `y < height`.
    #[inline]
    pub fn row_slice(&self, y: u32) -> &[u8] {
        if y >= self.height {
            row_index_out_of_bounds(y, self.height);
        }
        let off = self.rows[y as usize];
        &self.buf[off..off + self.stride_abs() as usize]
    }

    /// Row `y`, writable; see [`row_slice`](Self::row_slice).
    #[inline]
    pub fn row_slice_mut(&mut self, y: u32) -> &mut [u8] {
        if y >= self.height {
            row_index_out_of_bounds(y, self.height);
        }
        let off = self.rows[y as usize];
        let len = self.stride_abs() as usize;
        &mut self.buf[off..off + len]
    }

    /// Get row data (x1, x2 range + the row's bytes).
    pub fn row(&self, y: u32) -> RowData<'_> {
        RowData {
            x1: 0,
            x2: self.width as i32 - 1,
            ptr: self.row_slice(y),
        }
    }

    /// Copy pixel data from another buffer (min of both dimensions).
    ///
    /// Copies rows `0..min(self.height(), src.height())`, and of each row its first
    /// `min(self.stride_abs(), src.stride_abs())` bytes; row `y` of `src` goes to row `y` of
    /// `self`, whatever the sign of either stride.
    ///
    /// # Panics
    /// When `src` breaks the [`RenderingBufferAccess`] contract (a `row_slice` that panics for
    /// a row below its `height()`, or returns fewer than its `stride_abs()` bytes).
    pub fn copy_from<T: RenderingBufferAccess>(&mut self, src: &T) {
        let h = self.height.min(src.height());
        let l = self.stride_abs().min(src.stride_abs()) as usize;
        for y in 0..h {
            let src_row = &src.row_slice(y)[..l];
            self.row_slice_mut(y)[..l].copy_from_slice(src_row);
        }
    }

    /// Fill every attached byte with `value`.
    pub fn clear(&mut self, value: u8) {
        self.buf.fill(value);
    }
}

impl Default for RowPtrCache<'_> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Common trait for both buffer types
// ============================================================================

/// Common interface for rendering buffer access: the source side of `copy_from`.
///
/// A plain contract, checked rather than trusted: for every `y` in `[0, height())`,
/// `row_slice(y)` returns row `y`'s bytes, `stride_abs()` of them, in the order they appear
/// in the row (left to right); for any other `y` it panics. `width()` is the row's width in
/// pixels. `copy_from` checks every slice it is given, so an implementation that breaks this
/// contract makes it panic, never read or write out of bounds.
pub trait RenderingBufferAccess {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn stride_abs(&self) -> u32;
    /// The `stride_abs()` bytes of row `y`.
    ///
    /// # Panics
    /// Unless `y < height()`.
    fn row_slice(&self, y: u32) -> &[u8];
}

impl RenderingBufferAccess for RowAccessor<'_> {
    fn width(&self) -> u32 {
        self.width
    }
    fn height(&self) -> u32 {
        self.height
    }
    fn stride_abs(&self) -> u32 {
        self.stride_abs()
    }
    fn row_slice(&self, y: u32) -> &[u8] {
        self.row_slice(y)
    }
}

impl RenderingBufferAccess for RowPtrCache<'_> {
    fn width(&self) -> u32 {
        self.width
    }
    fn height(&self) -> u32 {
        self.height
    }
    fn stride_abs(&self) -> u32 {
        self.stride_abs()
    }
    fn row_slice(&self, y: u32) -> &[u8] {
        self.row_slice(y)
    }
}

/// Default rendering buffer type (matches C++ `typedef row_accessor<int8u> rendering_buffer`).
pub type RenderingBuffer<'a> = RowAccessor<'a>;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_row_accessor_basic() {
        let mut data = vec![0u8; 40]; // 10 wide × 4 high, RGBA
        let rb = RowAccessor::new_with_buf(&mut data, 10, 4, 10);
        assert_eq!(rb.width(), 10);
        assert_eq!(rb.height(), 4);
        assert_eq!(rb.stride(), 10);
        assert_eq!(rb.stride_abs(), 10);
    }

    #[test]
    fn test_row_accessor_write_read() {
        let mut data = vec![0u8; 30]; // 10 wide × 3 high
        let mut rb = RowAccessor::new_with_buf(&mut data, 10, 3, 10);

        // Write to row 1
        rb.row_slice_mut(1)[0] = 42;
        rb.row_slice_mut(1)[9] = 99;

        // Read back
        assert_eq!(rb.row_slice(1)[0], 42);
        assert_eq!(rb.row_slice(1)[9], 99);
        assert_eq!(rb.row_slice(0)[0], 0); // row 0 unchanged
        assert_eq!(data[10], 42);
        assert_eq!(data[19], 99);
    }

    #[test]
    fn test_row_accessor_negative_stride() {
        let mut data = vec![0u8; 30]; // 10 wide × 3 high, bottom-up
        data[0..10].fill(0);
        data[10..20].fill(1);
        data[20..30].fill(2);

        let mut rb = RowAccessor::new_with_buf(&mut data, 10, 3, -10);

        // With negative stride, row 0 is the LAST 10 bytes
        assert_eq!(rb.row_slice(0), &[2; 10]);
        assert_eq!(rb.row_slice(1), &[1; 10]);
        assert_eq!(rb.row_slice(2), &[0; 10]);
        // The checked crate-internal access agrees.
        assert_eq!(rb.row_bytes(0, 4), &[2; 4]);
        assert_eq!(rb.row_bytes(2, 10), &[0; 10]);
        rb.row_bytes_mut(0, 1)[0] = 7;
        assert_eq!(data[20], 7);
    }

    #[test]
    fn test_row_accessor_clear() {
        let mut data = vec![0u8; 20];
        let mut rb = RowAccessor::new_with_buf(&mut data, 5, 4, 5);
        rb.clear(0xFF);
        for byte in &data {
            assert_eq!(*byte, 0xFF);
        }
    }

    #[test]
    fn test_row_accessor_row_data() {
        let mut data = vec![0u8; 30];
        data[10] = 55; // row 1, pixel 0
        let rb = RowAccessor::new_with_buf(&mut data, 10, 3, 10);
        let rd = rb.row(1);
        assert_eq!(rd.x1, 0);
        assert_eq!(rd.x2, 9);
        assert_eq!(rd.ptr[0], 55);
    }

    #[test]
    fn test_row_ptr_cache_basic() {
        let mut data = vec![0u8; 40];
        let rpc = RowPtrCache::new_with_buf(&mut data, 10, 4, 10);
        assert_eq!(rpc.width(), 10);
        assert_eq!(rpc.height(), 4);
        assert_eq!(rpc.stride(), 10);
    }

    #[test]
    fn test_row_ptr_cache_write_read() {
        let mut data = vec![0u8; 30];
        let mut rpc = RowPtrCache::new_with_buf(&mut data, 10, 3, 10);

        rpc.row_slice_mut(2)[5] = 77;
        assert_eq!(rpc.row_slice(2)[5], 77);
        assert_eq!(rpc.row_slice(0)[5], 0);
        assert_eq!(data[25], 77);
    }

    /// Pins the row order of `RowPtrCache::attach`'s index-derived offsets: row 0 is the last
    /// row in memory, row `height - 1` the first.
    #[test]
    fn test_row_ptr_cache_negative_stride() {
        let mut data = vec![0u8; 30];
        data[0..10].fill(0);
        data[10..20].fill(1);
        data[20..30].fill(2);

        let rpc = RowPtrCache::new_with_buf(&mut data, 10, 3, -10);

        assert_eq!(rpc.row_slice(0), &[2; 10]);
        assert_eq!(rpc.row_slice(1), &[1; 10]);
        assert_eq!(rpc.row_slice(2), &[0; 10]);
    }

    /// Re-attaching a cache to a shorter buffer rebuilds the table for the new height only.
    #[test]
    fn test_row_ptr_cache_reattach_shorter() {
        let mut big = vec![0u8; 40];
        let mut small = vec![0u8; 6];
        small[0..3].fill(1);
        small[3..6].fill(2);
        let mut rpc = RowPtrCache::new_with_buf(&mut big, 10, 4, 10);
        rpc.attach(&mut small, 3, 2, -3);
        assert_eq!(rpc.height(), 2);
        assert_eq!(rpc.row_slice(0), &[2; 3]);
        assert_eq!(rpc.row_slice(1), &[1; 3]);
    }

    #[test]
    #[should_panic(expected = "row 2 out of bounds (height=2)")]
    fn test_row_ptr_cache_reattach_shorter_rejects_old_rows() {
        let mut big = vec![0u8; 40];
        let mut small = vec![0u8; 6];
        let mut rpc = RowPtrCache::new_with_buf(&mut big, 10, 4, 10);
        rpc.attach(&mut small, 3, 2, 3);
        let _ = rpc.row_slice(2);
    }

    #[test]
    fn test_row_ptr_cache_clear() {
        let mut data = vec![0u8; 20];
        let mut rpc = RowPtrCache::new_with_buf(&mut data, 5, 4, 5);
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
        let expected = src_data.clone();
        let src = RowAccessor::new_with_buf(&mut src_data, 10, 3, 10);

        let mut dst_data = vec![0u8; 30];
        let mut dst = RowAccessor::new_with_buf(&mut dst_data, 10, 3, 10);
        dst.copy_from(&src);

        assert_eq!(dst_data, expected);
    }

    #[test]
    fn test_copy_from_cache_to_cache() {
        let mut src_data = vec![0u8; 30];
        for (i, byte) in src_data.iter_mut().enumerate() {
            *byte = (i * 2) as u8;
        }
        let expected = src_data.clone();
        let src = RowPtrCache::new_with_buf(&mut src_data, 10, 3, 10);

        let mut dst_data = vec![0u8; 30];
        let mut dst = RowPtrCache::new_with_buf(&mut dst_data, 10, 3, 10);
        dst.copy_from(&src);

        assert_eq!(dst_data, expected);
    }

    /// Row `y` goes to row `y` across opposite stride signs, so the memory order flips.
    #[test]
    fn test_copy_from_opposite_strides() {
        let mut src_data = vec![1u8, 1, 2, 2, 3, 3];
        let src = RowAccessor::new_with_buf(&mut src_data, 2, 3, 2);
        let mut dst_data = vec![0u8; 6];
        let mut dst = RowAccessor::new_with_buf(&mut dst_data, 2, 3, -2);
        dst.copy_from(&src);
        assert_eq!(dst_data, [3, 3, 2, 2, 1, 1]);
    }

    #[test]
    fn test_copy_from_different_sizes() {
        // Source is smaller than destination
        let mut src_data = vec![42u8; 10]; // 5×2
        let src = RowAccessor::new_with_buf(&mut src_data, 5, 2, 5);

        let mut dst_data = vec![0u8; 30]; // 10×3
        let mut dst = RowAccessor::new_with_buf(&mut dst_data, 10, 3, 10);
        dst.copy_from(&src);

        // Only the first 5 bytes of the first 2 rows should be copied
        assert_eq!(dst_data[0..5], [42, 42, 42, 42, 42]);
        assert_eq!(dst_data[5..10], [0, 0, 0, 0, 0]);
        assert_eq!(dst_data[10..15], [42, 42, 42, 42, 42]);
        assert_eq!(dst_data[15..20], [0, 0, 0, 0, 0]);
        assert_eq!(dst_data[20..30], [0; 10]); // row 2 untouched
    }

    /// A `RenderingBufferAccess` written as a user of the library would: rows held in a
    /// `Vec<Vec<u8>>`. `claimed_height` and `claimed_stride` let a test make it lie about its
    /// size.
    struct VecRows {
        rows: Vec<Vec<u8>>,
        claimed_height: u32,
        claimed_stride: u32,
    }

    impl VecRows {
        fn honest(rows: Vec<Vec<u8>>) -> Self {
            let claimed_height = rows.len() as u32;
            let claimed_stride = rows[0].len() as u32;
            Self {
                rows,
                claimed_height,
                claimed_stride,
            }
        }
    }

    impl RenderingBufferAccess for VecRows {
        fn width(&self) -> u32 {
            self.claimed_stride
        }
        fn height(&self) -> u32 {
            self.claimed_height
        }
        fn stride_abs(&self) -> u32 {
            self.claimed_stride
        }
        fn row_slice(&self, y: u32) -> &[u8] {
            &self.rows[y as usize]
        }
    }

    #[test]
    fn test_copy_from_safe_outside_implementor() {
        let src = VecRows::honest(vec![
            vec![1, 2, 3, 4],
            vec![5, 6, 7, 8],
            vec![9, 10, 11, 12],
        ]);

        // Destination 3 bytes wide and 2 rows high: rows 0..2, 3 bytes each are copied.
        let mut dst_data = vec![0u8; 6];
        let mut dst = RowAccessor::new_with_buf(&mut dst_data, 3, 2, 3);
        dst.copy_from(&src);
        assert_eq!(dst_data, [1, 2, 3, 5, 6, 7]);

        // Bottom-up cache: row `y` still goes to row `y`, which sits last in memory.
        let mut cache_data = vec![0u8; 12];
        let mut cache = RowPtrCache::new_with_buf(&mut cache_data, 4, 3, -4);
        cache.copy_from(&src);
        assert_eq!(cache_data, [9, 10, 11, 12, 5, 6, 7, 8, 1, 2, 3, 4]);
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn test_copy_from_panics_on_implementor_claiming_extra_rows() {
        let mut src = VecRows::honest(vec![vec![7; 4], vec![7; 4]]);
        src.claimed_height = 3;
        let mut dst_data = vec![0u8; 12];
        let mut dst = RowAccessor::new_with_buf(&mut dst_data, 4, 3, 4);
        dst.copy_from(&src);
    }

    #[test]
    #[should_panic(expected = "out of range for slice of length 4")]
    fn test_copy_from_panics_on_implementor_claiming_longer_rows() {
        let mut src = VecRows::honest(vec![vec![7; 4], vec![7; 4]]);
        src.claimed_stride = 8;
        let mut dst_data = vec![0u8; 16];
        let mut dst = RowPtrCache::new_with_buf(&mut dst_data, 8, 2, 8);
        dst.copy_from(&src);
    }

    #[test]
    fn test_row_accessor_default() {
        let rb = RowAccessor::new();
        assert_eq!(rb.width(), 0);
        assert_eq!(rb.height(), 0);
        assert_eq!(rb.stride(), 0);
        assert!(rb.buf().is_empty());
        let rb = RowAccessor::default();
        assert_eq!(rb.height(), 0);
    }

    #[test]
    fn test_row_ptr_cache_default() {
        let rpc = RowPtrCache::new();
        assert_eq!(rpc.width(), 0);
        assert_eq!(rpc.height(), 0);
        assert_eq!(rpc.stride(), 0);
        assert!(rpc.buf().is_empty());
        let rpc = RowPtrCache::default();
        assert_eq!(rpc.height(), 0);
    }

    #[test]
    #[should_panic(expected = "row 0 out of bounds (height=0)")]
    fn test_row_accessor_unattached_row_slice_panics() {
        let _ = RowAccessor::new().row_slice(0);
    }

    #[test]
    #[should_panic(expected = "row 0 out of bounds (height=0)")]
    fn test_row_accessor_unattached_row_bytes_panics() {
        let _ = RowAccessor::new().row_bytes(0, 0);
    }

    #[test]
    #[should_panic(expected = "row 0 out of bounds (height=0)")]
    fn test_row_ptr_cache_unattached_row_slice_panics() {
        let _ = RowPtrCache::new().row_slice(0);
    }

    #[test]
    #[should_panic(expected = "row -1 out of bounds (height=3)")]
    fn test_row_bytes_rejects_negative_row() {
        let mut data = vec![0u8; 30];
        let rb = RowAccessor::new_with_buf(&mut data, 10, 3, -10);
        let _ = rb.row_bytes(-1, 1);
    }

    #[test]
    #[should_panic(expected = "row of 11 bytes does not fit in stride -10")]
    fn test_row_bytes_rejects_row_longer_than_stride() {
        let mut data = vec![0u8; 30];
        let rb = RowAccessor::new_with_buf(&mut data, 10, 3, -10);
        let _ = rb.row_bytes(1, 11);
    }

    #[test]
    fn test_row_fits() {
        let mut data = vec![0u8; 30];
        let rb = RowAccessor::new_with_buf(&mut data, 10, 3, -10);
        assert!(rb.row_fits(0, 10));
        assert!(rb.row_fits(2, 10));
        assert!(!rb.row_fits(3, 10));
        assert!(!rb.row_fits(-1, 10));
        assert!(!rb.row_fits(0, 11));
    }

    /// One past the largest row or column `i32` coordinates can address.
    const TOO_BIG: u32 = i32::MAX as u32 + 1;

    #[test]
    #[should_panic(expected = "rendering buffer height 2147483648 exceeds i32::MAX")]
    fn test_row_accessor_rejects_height_past_i32() {
        let _ = RowAccessor::new_with_buf(&mut [], 1, TOO_BIG, 0);
    }

    #[test]
    #[should_panic(expected = "rendering buffer width 2147483648 exceeds i32::MAX")]
    fn test_row_accessor_rejects_width_past_i32() {
        let _ = RowAccessor::new_with_buf(&mut [], TOO_BIG, 1, 0);
    }

    #[test]
    #[should_panic(expected = "rendering buffer height 2147483648 exceeds i32::MAX")]
    fn test_row_ptr_cache_rejects_height_past_i32() {
        let _ = RowPtrCache::new_with_buf(&mut [], 1, TOO_BIG, 0);
    }

    #[test]
    #[should_panic(expected = "rendering buffer width 2147483648 exceeds i32::MAX")]
    fn test_row_ptr_cache_rejects_width_past_i32() {
        let _ = RowPtrCache::new_with_buf(&mut [], TOO_BIG, 1, 0);
    }

    #[test]
    #[should_panic(
        expected = "rendering buffer of 29 bytes is shorter than 3 rows of stride 10 (30 bytes)"
    )]
    fn test_row_accessor_rejects_short_buffer() {
        let mut data = vec![0u8; 29];
        let _ = RowAccessor::new_with_buf(&mut data, 10, 3, 10);
    }

    #[test]
    #[should_panic(
        expected = "rendering buffer of 29 bytes is shorter than 3 rows of stride -10 (30 bytes)"
    )]
    fn test_row_accessor_rejects_short_buffer_negative_stride() {
        let mut data = vec![0u8; 29];
        let _ = RowAccessor::new_with_buf(&mut data, 10, 3, -10);
    }

    #[test]
    #[should_panic(
        expected = "rendering buffer of 29 bytes is shorter than 3 rows of stride 10 (30 bytes)"
    )]
    fn test_row_ptr_cache_rejects_short_buffer() {
        let mut data = vec![0u8; 29];
        let _ = RowPtrCache::new_with_buf(&mut data, 10, 3, 10);
    }

    #[test]
    #[should_panic(
        expected = "rendering buffer of 29 bytes is shorter than 3 rows of stride -10 (30 bytes)"
    )]
    fn test_row_ptr_cache_rejects_short_buffer_negative_stride() {
        let mut data = vec![0u8; 29];
        let _ = RowPtrCache::new_with_buf(&mut data, 10, 3, -10);
    }

    /// `i32::MAX` rows of `|i32::MIN| = 2^31` bytes: about `2^62` bytes, past a 32-bit
    /// `usize` (where a wrapping product would come out small) and past any real buffer.
    #[test]
    #[should_panic(
        expected = "rendering buffer of 16 bytes is shorter than 2147483647 rows of stride -2147483648 (4611686016279904256 bytes)"
    )]
    fn test_row_accessor_rejects_overflowing_size() {
        let mut data = [0u8; 16];
        let _ = RowAccessor::new_with_buf(&mut data, 1, i32::MAX as u32, i32::MIN);
    }

    #[test]
    #[should_panic(
        expected = "rendering buffer of 16 bytes is shorter than 2147483647 rows of stride 2147483647 (4611686014132420609 bytes)"
    )]
    fn test_row_ptr_cache_rejects_overflowing_size() {
        let mut data = [0u8; 16];
        let _ = RowPtrCache::new_with_buf(&mut data, 1, i32::MAX as u32, i32::MAX);
    }

    /// A failed attach keeps the previous attachment.
    #[test]
    fn test_failed_attach_keeps_previous() {
        let mut data = vec![5u8; 6];
        let mut short_data = [0u8; 2];
        let short = &mut short_data[..];
        let mut rb = RowAccessor::new_with_buf(&mut data, 3, 2, 3);
        let rb_ref = &mut rb;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            // Moving the borrows out of the captures makes the closure `FnOnce`, so `short`
            // can be stored for the accessor's lifetime.
            let (rb, short) = (rb_ref, short);
            rb.attach(short, 3, 2, 3);
        }));
        assert!(result.is_err());
        assert_eq!(rb.height(), 2);
        assert_eq!(rb.row_slice(1), &[5; 3]);
    }

    /// A longer slice is accepted; only its first `height * |stride|` bytes are attached, and
    /// with a negative stride row 0 is the last attached row, not the slice's last bytes.
    #[test]
    fn test_longer_buffer_uses_only_attached_bytes() {
        for stride in [10, -10] {
            let mut data = vec![0xEEu8; 45];
            let mut rb = RowAccessor::new_with_buf(&mut data, 10, 3, stride);
            assert_eq!(rb.buf().len(), 30);
            rb.row_slice_mut(0).fill(1);
            assert_eq!(rb.row_slice(0).len(), 10);
            rb.clear(0x11);
            rb.row_slice_mut(0)[0] = 9;
            assert!(data[30..].iter().all(|&b| b == 0xEE), "stride {stride}");
            let row0 = if stride < 0 { 20 } else { 0 };
            assert_eq!(data[row0], 9, "stride {stride}");

            let mut data = vec![0xEEu8; 45];
            let mut rpc = RowPtrCache::new_with_buf(&mut data, 10, 3, stride);
            assert_eq!(rpc.buf().len(), 30);
            rpc.clear(0x11);
            rpc.row_slice_mut(0)[0] = 9;
            assert!(data[30..].iter().all(|&b| b == 0xEE), "stride {stride}");
            assert_eq!(data[row0], 9, "stride {stride}");
        }
    }

    #[test]
    fn test_buf_exposes_attached_bytes() {
        let mut data: Vec<u8> = (0..12).collect();
        data.extend_from_slice(&[0xEE; 4]);
        let mut rb = RowAccessor::new_with_buf(&mut data, 2, 3, -4);
        assert_eq!(rb.buf(), &(0..12).collect::<Vec<u8>>()[..]);
        rb.buf_mut()[0] = 100;
        // Memory byte 0 is the first byte of the last row.
        assert_eq!(rb.row_slice(2)[0], 100);

        let mut data: Vec<u8> = (0..16).collect();
        let mut rpc = RowPtrCache::new_with_buf(&mut data, 2, 3, 4);
        assert_eq!(rpc.buf(), &(0..12).collect::<Vec<u8>>()[..]);
        rpc.buf_mut()[11] = 200;
        assert_eq!(rpc.row_slice(2)[3], 200);
        assert_eq!(data[12..], [12, 13, 14, 15]);
    }

    #[test]
    fn test_reborrow_same_rows_and_returns_accessor() {
        let mut data: Vec<u8> = (0..12).collect();
        let mut rb = RowAccessor::new_with_buf(&mut data, 4, 3, -4);
        {
            let mut short = rb.reborrow();
            assert_eq!(short.width(), 4);
            assert_eq!(short.height(), 3);
            assert_eq!(short.stride(), -4);
            assert_eq!(short.row_slice(0), &[8, 9, 10, 11]);
            assert_eq!(short.row_slice(2), &[0, 1, 2, 3]);
            short.row_slice_mut(1)[0] = 77;
        }
        // The original accessor is usable again and sees the write.
        assert_eq!(rb.row_slice(1), &[77, 5, 6, 7]);
        rb.row_slice_mut(0)[0] = 66;
        assert_eq!(data[8], 66);
        assert_eq!(data[4], 77);
    }

    /// Row slices borrowed through `&'a RowAccessor<'a>` live for `'a`, and a reference to a
    /// longer-lived accessor coerces to that type (the accessor is covariant).
    #[test]
    fn test_shared_borrow_row_lifetime() {
        fn first_row<'a>(rb: &'a RowAccessor<'a>) -> &'a [u8] {
            rb.row_bytes(0, 2)
        }
        let mut data = vec![3u8; 8];
        let rb = RowAccessor::new_with_buf(&mut data, 2, 2, 4);
        let row = first_row(&rb);
        assert_eq!(row, &[3, 3]);
    }

    #[test]
    fn test_zero_height_and_zero_stride() {
        let mut data = vec![0u8; 4];
        let rb = RowAccessor::new_with_buf(&mut data, 4, 0, -4);
        assert_eq!(rb.height(), 0);
        assert!(rb.buf().is_empty());
        let rb = RowAccessor::new_with_buf(&mut [], 0, 5, 0);
        assert!(rb.row_slice(4).is_empty());
        let rpc = RowPtrCache::new_with_buf(&mut [], 0, 5, 0);
        assert!(rpc.row_slice(4).is_empty());
    }

    #[test]
    fn test_rendering_buffer_alias() {
        let mut data = vec![0u8; 20];
        let rb: RenderingBuffer = RenderingBuffer::new_with_buf(&mut data, 5, 4, 5);
        assert_eq!(rb.height(), 4);
    }
}
