//! Smooth polygon converter.
//!
//! Port of `agg_conv_smooth_poly1.h` wrapper around `VcgenSmoothPoly1`.

use crate::basics::VertexSource;
use crate::conv_adaptor_vcgen::ConvAdaptorVcgen;
use crate::vcgen_smooth_poly1::VcgenSmoothPoly1;

/// Port of C++ `conv_smooth_poly1<VertexSource>`.
pub struct ConvSmoothPoly1<VS: VertexSource> {
    base: ConvAdaptorVcgen<VS, VcgenSmoothPoly1>,
}

impl<VS: VertexSource> ConvSmoothPoly1<VS> {
    pub fn new(source: VS) -> Self {
        Self {
            base: ConvAdaptorVcgen::new(source, VcgenSmoothPoly1::new()),
        }
    }

    pub fn set_smooth_value(&mut self, v: f64) {
        self.base.generator_mut().set_smooth_value(v);
    }

    pub fn smooth_value(&self) -> f64 {
        self.base.generator().smooth_value()
    }

    pub fn source(&self) -> &VS {
        self.base.source()
    }

    pub fn source_mut(&mut self) -> &mut VS {
        self.base.source_mut()
    }
}

impl<VS: VertexSource> VertexSource for ConvSmoothPoly1<VS> {
    fn rewind(&mut self, path_id: u32) {
        self.base.rewind(path_id);
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        self.base.vertex(x, y)
    }
}
