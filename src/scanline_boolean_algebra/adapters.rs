//! The boolean algebra's traits (crate root of this module) for agg-rust's own types: the
//! rasterizer and the storages as [`ScanlineGenerator`]s, the scanlines as
//! [`ScanlineSpans`] / [`ScanlineCovers`], the storages as [`ScanlineSink`]s. Each method
//! forwards to the type's own (inherent) method of the same C++ name.

use super::{ScanlineCovers, ScanlineGenerator, ScanlineSink, ScanlineSpans};
use crate::rasterizer_scanline_aa::{RasterizerScanlineAa, Scanline};
use crate::scanline_bin::ScanlineBin;
use crate::scanline_p::ScanlineP8;
use crate::scanline_storage_aa::ScanlineStorageAa;
use crate::scanline_storage_bin::ScanlineStorageBin;
use crate::scanline_u::ScanlineU8;

macro_rules! generator {
    ($t:ty) => {
        impl ScanlineGenerator for $t {
            fn rewind_scanlines(&mut self) -> bool {
                <$t>::rewind_scanlines(self)
            }
            fn min_x(&self) -> i32 {
                <$t>::min_x(self)
            }
            fn min_y(&self) -> i32 {
                <$t>::min_y(self)
            }
            fn max_x(&self) -> i32 {
                <$t>::max_x(self)
            }
            fn max_y(&self) -> i32 {
                <$t>::max_y(self)
            }
            fn sweep_scanline<SL: Scanline>(&mut self, sl: &mut SL) -> bool {
                <$t>::sweep_scanline(self, sl)
            }
        }
    };
}

generator!(RasterizerScanlineAa);
generator!(ScanlineStorageAa);
generator!(ScanlineStorageBin);

macro_rules! spans {
    ($t:ty) => {
        impl ScanlineSpans for $t {
            fn reset(&mut self, min_x: i32, max_x: i32) {
                <$t>::reset(self, min_x, max_x)
            }
            fn span_x(&self, i: usize) -> i32 {
                self.begin()[i].x
            }
            fn span_len(&self, i: usize) -> i32 {
                self.begin()[i].len
            }
        }
    };
}

spans!(ScanlineU8);
spans!(ScanlineP8);
spans!(ScanlineBin);

impl ScanlineCovers for ScanlineU8 {
    fn span_covers(&self, i: usize) -> &[u8] {
        &self.covers()[self.begin()[i].cover_offset..]
    }
}

impl ScanlineCovers for ScanlineP8 {
    fn span_covers(&self, i: usize) -> &[u8] {
        &self.covers()[self.begin()[i].cover_offset..]
    }
}

impl ScanlineSink<ScanlineU8> for ScanlineStorageAa {
    fn prepare(&mut self) {
        ScanlineStorageAa::prepare(self)
    }
    fn render(&mut self, sl: &ScanlineU8) {
        self.render_scanline_u8(sl)
    }
}

impl ScanlineSink<ScanlineP8> for ScanlineStorageAa {
    fn prepare(&mut self) {
        ScanlineStorageAa::prepare(self)
    }
    fn render(&mut self, sl: &ScanlineP8) {
        self.render_scanline_p8(sl)
    }
}

impl ScanlineSink<ScanlineBin> for ScanlineStorageBin {
    fn prepare(&mut self) {
        ScanlineStorageBin::prepare(self)
    }
    fn render(&mut self, sl: &ScanlineBin) {
        self.render_scanline_bin(sl)
    }
}

impl ScanlineSink<ScanlineU8> for ScanlineStorageBin {
    fn prepare(&mut self) {
        ScanlineStorageBin::prepare(self)
    }
    fn render(&mut self, sl: &ScanlineU8) {
        self.render_scanline_u8(sl)
    }
}
