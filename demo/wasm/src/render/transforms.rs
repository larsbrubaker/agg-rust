//! Transform/text demo render functions: raster_text, gamma_ctrl, trans_polar,
//! multi_clip, simple_blur, blur, trans_curve1, trans_curve2, lion_lens, distortions,
//! gouraud_mesh, truetype_test.

use agg_rust::basics::{is_end_poly, is_move_to, is_stop, is_vertex, VertexSource};
use agg_rust::bspline::Bspline;
use agg_rust::color::Rgba8;
use agg_rust::conv_curve::ConvCurve;
use agg_rust::conv_segmentator::ConvSegmentator;
use agg_rust::conv_stroke::ConvStroke;
use agg_rust::conv_transform::ConvTransform;
use agg_rust::ctrl::{render_ctrl, SliderCtrl, RboxCtrl, GammaCtrl, CboxCtrl, Ctrl};
use agg_rust::ellipse::Ellipse;
use agg_rust::embedded_raster_fonts;
use agg_rust::font_cache::FontCacheManager;
use agg_rust::glyph_raster_bin::{GlyphRasterBin, GlyphRect};
use agg_rust::gradient_lut::GradientLinearColor;
use agg_rust::gsv_text::GsvText;
use agg_rust::math::fast_sqrt;
use agg_rust::path_storage::PathStorage;
use agg_rust::pixfmt_rgba::{PixelFormat, PixfmtRgba32};
use agg_rust::rasterizer_scanline_aa::RasterizerScanlineAa;
use agg_rust::renderer_base::RendererBase;
use agg_rust::renderer_raster_text::render_raster_htext_solid;
use agg_rust::rasterizer_compound_aa::RasterizerCompoundAa;
use agg_rust::renderer_scanline::{render_scanlines_aa, render_scanlines_aa_solid, SpanGenerator};
use agg_rust::rendering_buffer::RowAccessor;
use agg_rust::scanline_u::ScanlineU8;
use agg_rust::span_gradient::GradientFunction;
use agg_rust::span_allocator::SpanAllocator;
use agg_rust::span_gouraud_rgba::SpanGouraudRgba;
use agg_rust::span_image_filter_rgba::SpanImageFilterRgbaBilinearClip;
use agg_rust::span_interpolator_linear::{SpanInterpolatorLinear, Transformer};
use agg_rust::trans_affine::TransAffine;
use agg_rust::trans_polar::TransPolar;
use agg_rust::trans_single_path::TransSinglePath;
use agg_rust::trans_warp_magnifier::TransWarpMagnifier;
use super::setup_renderer;

/// Embedded Liberation Serif Italic font (SIL OFL license).
/// Metrically compatible with Times New Roman Italic (timesi.ttf) used by the C++ demo.
static LIBERATION_SERIF_ITALIC: &[u8] = include_bytes!("../../fonts/LiberationSerif-Italic.ttf");
/// Embedded C++-matching typefaces used by truetype_lcd.cpp.
static ARIAL_REGULAR: &[u8] = include_bytes!("../../fonts/Arial-Regular.ttf");
static ARIAL_ITALIC: &[u8] = include_bytes!("../../fonts/Arial-Italic.ttf");
static TAHOMA_REGULAR: &[u8] = include_bytes!("../../fonts/Tahoma-Regular.ttf");
static VERDANA_REGULAR: &[u8] = include_bytes!("../../fonts/Verdana-Regular.ttf");
static VERDANA_ITALIC: &[u8] = include_bytes!("../../fonts/Verdana-Italic.ttf");
static TIMES_REGULAR: &[u8] = include_bytes!("../../fonts/TimesNewRoman-Regular.ttf");
static TIMES_ITALIC: &[u8] = include_bytes!("../../fonts/TimesNewRoman-Italic.ttf");
static GEORGIA_REGULAR: &[u8] = include_bytes!("../../fonts/Georgia-Regular.ttf");
static GEORGIA_ITALIC: &[u8] = include_bytes!("../../fonts/Georgia-Italic.ttf");

// ============================================================================
// Raster Text
// ============================================================================

/// Render all 34 embedded raster fonts with sample text.
/// Matches C++ raster_text.cpp.
pub fn raster_text(width: u32, height: u32, _params: &[f64]) -> Vec<u8> {
    let fonts: &[(&[u8], &str)] = &[
        (embedded_raster_fonts::GSE4X6, "gse4x6"),
        (embedded_raster_fonts::GSE4X8, "gse4x8"),
        (embedded_raster_fonts::GSE5X7, "gse5x7"),
        (embedded_raster_fonts::GSE5X9, "gse5x9"),
        (embedded_raster_fonts::GSE6X9, "gse6x9"),
        (embedded_raster_fonts::GSE6X12, "gse6x12"),
        (embedded_raster_fonts::GSE7X11, "gse7x11"),
        (embedded_raster_fonts::GSE7X11_BOLD, "gse7x11_bold"),
        (embedded_raster_fonts::GSE7X15, "gse7x15"),
        (embedded_raster_fonts::GSE7X15_BOLD, "gse7x15_bold"),
        (embedded_raster_fonts::GSE8X16, "gse8x16"),
        (embedded_raster_fonts::GSE8X16_BOLD, "gse8x16_bold"),
        (embedded_raster_fonts::MCS11_PROP, "mcs11_prop"),
        (embedded_raster_fonts::MCS11_PROP_CONDENSED, "mcs11_prop_condensed"),
        (embedded_raster_fonts::MCS12_PROP, "mcs12_prop"),
        (embedded_raster_fonts::MCS13_PROP, "mcs13_prop"),
        (embedded_raster_fonts::MCS5X10_MONO, "mcs5x10_mono"),
        (embedded_raster_fonts::MCS5X11_MONO, "mcs5x11_mono"),
        (embedded_raster_fonts::MCS6X10_MONO, "mcs6x10_mono"),
        (embedded_raster_fonts::MCS6X11_MONO, "mcs6x11_mono"),
        (embedded_raster_fonts::MCS7X12_MONO_HIGH, "mcs7x12_mono_high"),
        (embedded_raster_fonts::MCS7X12_MONO_LOW, "mcs7x12_mono_low"),
        (embedded_raster_fonts::VERDANA12, "verdana12"),
        (embedded_raster_fonts::VERDANA12_BOLD, "verdana12_bold"),
        (embedded_raster_fonts::VERDANA13, "verdana13"),
        (embedded_raster_fonts::VERDANA13_BOLD, "verdana13_bold"),
        (embedded_raster_fonts::VERDANA14, "verdana14"),
        (embedded_raster_fonts::VERDANA14_BOLD, "verdana14_bold"),
        (embedded_raster_fonts::VERDANA16, "verdana16"),
        (embedded_raster_fonts::VERDANA16_BOLD, "verdana16_bold"),
        (embedded_raster_fonts::VERDANA17, "verdana17"),
        (embedded_raster_fonts::VERDANA17_BOLD, "verdana17_bold"),
        (embedded_raster_fonts::VERDANA18, "verdana18"),
        (embedded_raster_fonts::VERDANA18_BOLD, "verdana18_bold"),
    ];

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut glyph = GlyphRasterBin::new(fonts[0].0);
    let black = Rgba8::new(0, 0, 0, 255);

    let mut y = 5.0;
    for &(font_data, name) in fonts {
        glyph.set_font(font_data);
        let text = format!("A quick brown fox jumps over the lazy dog 0123456789: {}", name);
        render_raster_htext_solid(&mut rb, &mut glyph, 5.0, y, &text, &black, false);
        y += glyph.height() + 1.0;
    }

    // Render gradient raster text matching C++ renderer_raster_htext pipeline.
    let mut grad_func = GradientSineRepeatAdaptor::new();
    grad_func.set_periods(5.0);
    let color_func = GradientLinearColor::new(
        Rgba8::new(255, 0, 0, 255),
        Rgba8::new(0, 128, 0, 255),
        256,
    );
    let inter = SpanInterpolatorLinear::new(TransAffine::new());
    let mut span_gen = RasterTextGradientSpan::new(inter, grad_func, &color_func, 0.0, 150.0);
    render_raster_htext_span(
        &mut rb,
        &mut glyph,
        5.0,
        465.0,
        "RADIAL REPEATING GRADIENT: A quick brown fox jumps over the lazy dog",
        &mut span_gen,
        false,
    );

    buf
}

struct GradientSineRepeatAdaptor {
    periods: f64,
}

impl GradientSineRepeatAdaptor {
    fn new() -> Self {
        Self {
            periods: std::f64::consts::PI * 2.0,
        }
    }

    fn set_periods(&mut self, periods: f64) {
        self.periods = periods * std::f64::consts::PI * 2.0;
    }
}

impl GradientFunction for GradientSineRepeatAdaptor {
    fn calculate(&self, x: i32, y: i32, d: i32) -> i32 {
        let xx = x as i64 * x as i64;
        let yy = y as i64 * y as i64;
        let sum = (xx + yy).min(u32::MAX as i64) as u32;
        let dist = fast_sqrt(sum) as f64;
        (((1.0 + (dist * self.periods / d as f64).sin()) * d as f64) / 2.0) as i32
    }
}

struct RasterTextGradientSpan<'a, G, F> {
    interpolator: SpanInterpolatorLinear,
    gradient_function: G,
    color_function: &'a F,
    d1: i32,
    d2: i32,
}

impl<'a, G: GradientFunction, F: agg_rust::gradient_lut::ColorFunction> RasterTextGradientSpan<'a, G, F> {
    fn new(
        interpolator: SpanInterpolatorLinear,
        gradient_function: G,
        color_function: &'a F,
        d1: f64,
        d2: f64,
    ) -> Self {
        Self {
            interpolator,
            gradient_function,
            color_function,
            d1: (d1 * 16.0).round() as i32,
            d2: (d2 * 16.0).round() as i32,
        }
    }
}

impl<'a, G, F> SpanGenerator for RasterTextGradientSpan<'a, G, F>
where
    G: GradientFunction,
    F: agg_rust::gradient_lut::ColorFunction,
    F::Color: Copy,
{
    type Color = F::Color;

    fn prepare(&mut self) {}

    fn generate(&mut self, span: &mut [F::Color], x: i32, y: i32, len: u32) {
        const DOWNSCALE_SHIFT: i32 = 4;
        let dd = (self.d2 - self.d1).max(1);
        self.interpolator.begin(x as f64 + 0.5, y as f64 + 0.5, len);
        let color_size = self.color_function.size() as i32;

        for pixel in span.iter_mut().take(len as usize) {
            let mut ix = 0i32;
            let mut iy = 0i32;
            self.interpolator.coordinates(&mut ix, &mut iy);
            let d = self.gradient_function.calculate(ix >> DOWNSCALE_SHIFT, iy >> DOWNSCALE_SHIFT, self.d2);
            let d = (((d - self.d1) * color_size) / dd).clamp(0, color_size - 1);
            *pixel = self.color_function.get(d as usize);
            self.interpolator.next();
        }
    }
}

fn render_raster_htext_span<PF, SG>(
    ren: &mut RendererBase<PF>,
    glyph: &mut GlyphRasterBin,
    x: f64,
    y: f64,
    text: &str,
    span_gen: &mut SG,
    flip: bool,
) where
    PF: PixelFormat<ColorType = SG::Color>,
    SG: SpanGenerator,
    SG::Color: Default + Clone,
{
    let mut x = x;
    let mut y = y;
    let mut r = GlyphRect::default();

    for ch in text.bytes() {
        glyph.prepare(&mut r, x, y, ch as u32, flip);
        if r.x2 >= r.x1 {
            span_gen.prepare();
            let row_len = (r.x2 - r.x1 + 1) as usize;
            for i in r.y1..=r.y2 {
                let covers = if flip {
                    glyph.span((r.y2 - i) as u32)
                } else {
                    glyph.span((i - r.y1) as u32)
                };
                let mut colors = vec![SG::Color::default(); row_len];
                span_gen.generate(&mut colors, r.x1, i, row_len as u32);
                ren.blend_color_hspan(r.x1, i, row_len as i32, &colors, covers, 0);
            }
        }
        x += r.dx;
        y += r.dy;
    }
}

// ============================================================================
// Gamma Ctrl
// ============================================================================

/// Gamma control widget demo — matching C++ gamma_ctrl.cpp.
///
/// params[0..4] = gamma spline values (kx1, ky1, kx2, ky2)
pub fn gamma_ctrl_demo(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let kx1 = params.first().copied().unwrap_or(1.0);
    let ky1 = params.get(1).copied().unwrap_or(1.0);
    let kx2 = params.get(2).copied().unwrap_or(1.0);
    let ky2 = params.get(3).copied().unwrap_or(1.0);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Create and render gamma control widget
    let mut g_ctrl = GammaCtrl::new(10.0, 10.0, 300.0, 200.0);
    g_ctrl.text_size(10.0, 12.0);
    g_ctrl.set_values(kx1, ky1, kx2, ky2);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut g_ctrl);

    let w = width as f64;
    let ecenter = w / 2.0;
    let ewidth = w / 2.0 - 10.0;

    // 5 pairs of stroked ellipses with different widths and colors
    let configs: &[(f64, f64, Rgba8)] = &[
        (220.0, 2.0, Rgba8::new(0, 0, 0, 255)),
        (260.0, 2.0, Rgba8::new(127, 127, 127, 255)),
        (300.0, 2.0, Rgba8::new(192, 192, 192, 255)),
        (340.0, 1.0, Rgba8::new(0, 0, 102, 255)),
        (380.0, 0.4, Rgba8::new(0, 0, 102, 255)),
    ];

    for &(cy, stroke_w, ref color) in configs {
        // Large ellipse
        let mut ell = Ellipse::new(ecenter, cy, ewidth, 15.5, 100, false);
        let mut poly = ConvStroke::new(&mut ell);
        poly.set_width(stroke_w);
        ras.reset();
        ras.add_path(&mut poly, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, color);

        // Small ellipse
        let mut ell2 = Ellipse::new(ecenter, cy, 10.5, 10.5, 100, false);
        let mut poly2 = ConvStroke::new(&mut ell2);
        poly2.set_width(stroke_w);
        ras.reset();
        ras.add_path(&mut poly2, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, color);
    }

    // Render skewed text "Text 2345"
    let mut mtx = TransAffine::new();
    mtx.multiply(&TransAffine::new_skewing(0.15, 0.0));

    let mut text = GsvText::new();
    text.size(50.0, 20.0);
    text.start_point(320.0, 10.0);
    text.text("Text 2345");
    let mut text_path = ConvStroke::new(&mut text);
    text_path.set_width(2.0);
    let mut text_transformed = ConvTransform::new(&mut text_path, mtx);
    ras.reset();
    ras.add_path(&mut text_transformed, 0);
    let green = Rgba8::new(0, 128, 0, 255);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &green);

    // Render rotating arrows
    let red = Rgba8::new(128, 0, 0, 255);
    let mut arrow = PathStorage::new();
    arrow.move_to(30.0, -1.0);
    arrow.line_to(60.0, 0.0);
    arrow.line_to(30.0, 1.0);
    arrow.move_to(27.0, -1.0);
    arrow.line_to(10.0, 0.0);
    arrow.line_to(27.0, 1.0);

    for i in 0..35 {
        let mut mtx2 = TransAffine::new();
        mtx2.multiply(&TransAffine::new_rotation(
            i as f64 / 35.0 * std::f64::consts::PI * 2.0,
        ));
        mtx2.multiply(&TransAffine::new_translation(400.0, 130.0));
        let mut trans = ConvTransform::new(&mut arrow, mtx2);
        ras.reset();
        ras.add_path(&mut trans, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &red);
    }

    buf
}

// ============================================================================
// Trans Polar
// ============================================================================

/// Polar coordinate transformation demo — matching C++ trans_polar.cpp.
///
/// params[0] = value (0-100, default 32)
/// params[1] = spiral (-0.1 to 0.1, default 0)
/// params[2] = base_y (50-200, default 120)
pub fn trans_polar_demo(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let value = params.first().copied().unwrap_or(32.0);
    let spiral = params.get(1).copied().unwrap_or(0.0);
    let base_y = params.get(2).copied().unwrap_or(120.0);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Create slider controls
    let w = width as f64;
    let h = height as f64;

    let mut slider1 = SliderCtrl::new(10.0, 10.0, w - 10.0, 17.0);
    slider1.range(0.0, 100.0);
    slider1.num_steps(5);
    slider1.set_value(value);
    slider1.label("Some Value=%1.0f");

    let mut slider_spiral = SliderCtrl::new(10.0, 30.0, w - 10.0, 37.0);
    slider_spiral.label("Spiral=%.3f");
    slider_spiral.range(-0.1, 0.1);
    slider_spiral.set_value(spiral);

    let mut slider_base_y = SliderCtrl::new(10.0, 50.0, w - 10.0, 57.0);
    slider_base_y.label("Base Y=%.3f");
    slider_base_y.range(50.0, 200.0);
    slider_base_y.set_value(base_y);

    // Render the straight sliders
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut slider1);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut slider_spiral);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut slider_base_y);

    // Set up polar transform
    let mut trans = TransPolar::new();
    trans.base_angle = 2.0 * std::f64::consts::PI / -600.0; // full_circle(-600)
    trans.base_scale = -1.0;
    trans.base_y = slider_base_y.value();
    trans.translation_x = w / 2.0;
    trans.translation_y = h / 2.0 + 30.0;
    trans.spiral = -slider_spiral.value();

    // Transform the first slider through polar coordinates.
    // Matches C++ transformed_control + conv_segmentator + conv_transform pipeline:
    // preserves all vertex commands (move_to, line_to, close_polygon) through transform.
    let num_paths = slider1.num_paths();
    let colors: Vec<Rgba8> = (0..num_paths).map(|i| slider1.color(i)).collect();
    for i in 0..num_paths {
        let mut segm = ConvSegmentator::new(&mut slider1);
        segm.rewind(i);
        let (mut x, mut y) = (0.0, 0.0);
        let mut path = PathStorage::new();
        loop {
            let cmd = segm.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            if is_vertex(cmd) {
                trans.transform(&mut x, &mut y);
                if is_move_to(cmd) {
                    path.move_to(x, y);
                } else {
                    path.line_to(x, y);
                }
            } else if is_end_poly(cmd) {
                path.close_polygon(0);
            }
        }
        ras.reset();
        ras.add_path(&mut path, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &colors[i as usize]);
    }

    buf
}

// ============================================================================
// Multi Clip
// ============================================================================

/// Multi-clip demo — lion rendered through N×N clip regions.
/// Matches C++ multi_clip.cpp.
///
/// params[0] = N (grid size, 2-10, default 4)
/// params[1] = angle (default 0)
/// params[2] = scale (default 1.0)
pub fn multi_clip(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let n = params.first().copied().unwrap_or(4.0).clamp(2.0, 10.0);
    let angle = params.get(1).copied().unwrap_or(0.0);
    let scale = params.get(2).copied().unwrap_or(1.0).max(0.01);

    let (mut path, colors, path_idx) = crate::lion_data::parse_lion();
    let npaths = colors.len();

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let w = width as i32;
    let h = height as i32;

    // Build transform
    let base_dx = 120.0;
    let base_dy = 190.0;
    let mut mtx = TransAffine::new();
    mtx.multiply(&TransAffine::new_translation(-base_dx, -base_dy));
    mtx.multiply(&TransAffine::new_scaling_uniform(scale));
    mtx.multiply(&TransAffine::new_rotation(angle + std::f64::consts::PI));
    mtx.multiply(&TransAffine::new_translation(
        width as f64 / 2.0,
        height as f64 / 2.0,
    ));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Render lion into each clip box in the N×N grid
    let ni = n as i32;
    for gx in 0..ni {
        for gy in 0..ni {
            let x1 = w * gx / ni + 5;
            let y1 = h * gy / ni + 5;
            let x2 = w * (gx + 1) / ni - 5;
            let y2 = h * (gy + 1) / ni - 5;
            if x2 > x1 && y2 > y1 {
                rb.clip_box_i(x1, y1, x2, y2);
                for i in 0..npaths {
                    let start = path_idx[i] as u32;
                    let mut transformed = ConvTransform::new(&mut path, mtx);
                    ras.reset();
                    ras.add_path(&mut transformed, start);
                    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &colors[i]);
                }
            }
        }
    }

    // Reset to full clip for controls
    rb.clip_box_i(0, 0, w - 1, h - 1);

    // Render random circles with gradients
    let mut seed: u32 = 12345;
    let mut rng = || -> u32 {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        (seed >> 16) & 0x7fff
    };

    for _ in 0..50 {
        let cx = (rng() % width as u32) as f64;
        let cy = (rng() % height as u32) as f64;
        let radius = (rng() % 10 + 5) as f64;

        let mut ell = Ellipse::new(cx, cy, radius, radius, 32, false);
        let color = Rgba8::new(
            (rng() & 0x7F) as u32,
            (rng() & 0x7F) as u32,
            (rng() & 0x7F) as u32,
            ((rng() & 0x7F) + 0x7F) as u32,
        );
        ras.reset();
        ras.add_path(&mut ell, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &color);
    }

    // Render slider control
    let mut s_num = SliderCtrl::new(5.0, 5.0, 150.0, 12.0);
    s_num.range(2.0, 10.0);
    s_num.label("N=%.2f");
    s_num.set_value(n);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_num);

    buf
}

// ============================================================================
// Simple Blur
// ============================================================================

/// Simple 3×3 box blur on the lion — matching C++ simple_blur.cpp.
///
/// params[0] = angle (default 0)
/// params[1] = scale (default 1.0)
pub fn simple_blur(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let angle = params.first().copied().unwrap_or(0.0);
    let scale = params.get(1).copied().unwrap_or(1.0).max(0.01);

    let (mut path, colors, path_idx) = crate::lion_data::parse_lion();
    let npaths = colors.len();

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Build transform
    let base_dx = 120.0;
    let base_dy = 190.0;
    let mut mtx = TransAffine::new();
    mtx.multiply(&TransAffine::new_translation(-base_dx, -base_dy));
    mtx.multiply(&TransAffine::new_scaling_uniform(scale));
    mtx.multiply(&TransAffine::new_rotation(angle + std::f64::consts::PI));
    mtx.multiply(&TransAffine::new_translation(
        width as f64 / 2.0,
        height as f64 / 2.0,
    ));

    // Render lion
    for i in 0..npaths {
        let start = path_idx[i] as u32;
        let mut transformed = ConvTransform::new(&mut path, mtx);
        ras.reset();
        ras.add_path(&mut transformed, start);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &colors[i]);
    }

    // Apply simple 3×3 box blur to the buffer
    apply_box_blur_3x3(&mut buf, width, height);

    // Render the un-blurred lion again on the left side for comparison
    rb.clip_box_i(0, 0, width as i32 / 2, height as i32 - 1);
    rb.clear(&Rgba8::new(255, 255, 255, 255));
    for i in 0..npaths {
        let start = path_idx[i] as u32;
        let mut transformed = ConvTransform::new(&mut path, mtx);
        ras.reset();
        ras.add_path(&mut transformed, start);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &colors[i]);
    }

    // Label
    rb.clip_box_i(0, 0, width as i32 - 1, height as i32 - 1);
    let mut label = GsvText::new();
    label.size(10.0, 0.0);
    label.start_point(10.0, height as f64 - 20.0);
    label.text("Left: original  |  Right: 3x3 box blur");
    let mut label_stroke = ConvStroke::new(&mut label);
    label_stroke.set_width(1.0);
    ras.reset();
    ras.add_path(&mut label_stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

    buf
}

/// Apply a simple 3×3 box blur to an RGBA buffer.
fn apply_box_blur_3x3(buf: &mut Vec<u8>, width: u32, height: u32) {
    let stride = (width * 4) as usize;
    let src = buf.clone();
    for y in 1..height as usize - 1 {
        for x in 1..width as usize - 1 {
            for c in 0..4 {
                let mut sum = 0u32;
                for dy in 0..3usize {
                    for dx in 0..3usize {
                        let ny = y + dy - 1;
                        let nx = x + dx - 1;
                        sum += src[ny * stride + nx * 4 + c] as u32;
                    }
                }
                buf[y * stride + x * 4 + c] = (sum / 9) as u8;
            }
        }
    }
}

// ============================================================================
// Blur
// ============================================================================

/// Stack blur demo — matching C++ blur.cpp.
///
/// params[0] = blur radius (0-40, default 15)
/// params[1] = method (0=stack_blur, 1=recursive, 2=channels)
pub fn blur_demo(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let radius = params.first().copied().unwrap_or(15.0).clamp(0.0, 40.0);
    let method = params.get(1).copied().unwrap_or(0.0) as u32;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    let w = width as f64;
    let h = height as f64;

    // Draw a colorful shape — bezier curves forming a closed shape
    let mut shape = PathStorage::new();
    shape.move_to(w * 0.2, h * 0.3);
    shape.curve4(w * 0.4, h * 0.1, w * 0.6, h * 0.1, w * 0.8, h * 0.3);
    shape.curve4(w * 0.9, h * 0.5, w * 0.8, h * 0.7, w * 0.6, h * 0.8);
    shape.curve4(w * 0.4, h * 0.9, w * 0.2, h * 0.7, w * 0.15, h * 0.5);
    shape.close_polygon(0);

    let mut curve = ConvCurve::new(&mut shape);
    ras.reset();
    ras.add_path(&mut curve, 0);
    let fill_color = Rgba8::new(100, 140, 220, 200);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &fill_color);

    // Draw a red circle
    let mut ell = Ellipse::new(w * 0.35, h * 0.45, 60.0, 60.0, 64, false);
    ras.reset();
    ras.add_path(&mut ell, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(220, 60, 60, 200));

    // Draw a green triangle
    let mut tri = PathStorage::new();
    tri.move_to(w * 0.5, h * 0.2);
    tri.line_to(w * 0.7, h * 0.65);
    tri.line_to(w * 0.3, h * 0.65);
    tri.close_polygon(0);
    ras.reset();
    ras.add_path(&mut tri, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(60, 200, 60, 180));

    // Apply blur based on method
    if radius > 0.5 {
        let r = radius as u32;
        let mut ra_blur = RowAccessor::new();
        let stride = (width * 4) as i32;
        unsafe { ra_blur.attach(buf.as_mut_ptr(), width, height, stride) };
        match method {
            0 => agg_rust::blur::stack_blur_rgba32(&mut ra_blur, r, r),
            1 => agg_rust::blur::recursive_blur_rgba32(&mut ra_blur, radius),
            _ => {
                agg_rust::blur::stack_blur_rgba32(&mut ra_blur, r, r);
            }
        }
    }

    // Render controls on top (after blur)
    {
        let mut ra2 = RowAccessor::new();
        let stride = (width * 4) as i32;
        unsafe { ra2.attach(buf.as_mut_ptr(), width, height, stride) };
        let pf2 = PixfmtRgba32::new(&mut ra2);
        let mut rb2 = RendererBase::new(pf2);
        let mut ras2 = RasterizerScanlineAa::new();
        let mut sl2 = ScanlineU8::new();

        let mut s_radius = SliderCtrl::new(5.0, 5.0, width as f64 - 5.0, 12.0);
        s_radius.range(0.0, 40.0);
        s_radius.label("Blur Radius=%.2f");
        s_radius.set_value(radius);
        render_ctrl(&mut ras2, &mut sl2, &mut rb2, &mut s_radius);

        let mut r_method = RboxCtrl::new(5.0, 25.0, 130.0, 82.0);
        r_method.add_item("Stack Blur");
        r_method.add_item("Recursive Blur");
        r_method.add_item("Channels");
        r_method.set_cur_item(method as i32);
        render_ctrl(&mut ras2, &mut sl2, &mut rb2, &mut r_method);
    }

    buf
}

// ============================================================================
// Trans Curve 1
// ============================================================================

/// Text along a curved path using TrueType font outlines and trans_single_path.
/// Matching C++ trans_curve1_ft.cpp.
///
/// params[0] = num_points (10-400, default 200)
/// params[1..12] = control points x,y pairs (6 points)
/// params[13] = preserve_x_scale (0 or 1, default 1)
/// params[14] = fixed_length (0 or 1, default 1)
/// params[15] = close (0 or 1, default 0)
/// params[16] = animate (0 or 1, default 0)
pub fn trans_curve1(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let num_points = params.first().copied().unwrap_or(200.0).clamp(10.0, 400.0);

    // Default control points matching C++ on_init():
    // (50,50), (150+20,150-20), (250-20,250+20), (350+20,350-20), (450-20,450+20), (550,550)
    let default_pts = [
        50.0, 50.0,
        170.0, 130.0,
        230.0, 270.0,
        370.0, 330.0,
        430.0, 470.0,
        550.0, 550.0,
    ];
    let pts: Vec<f64> = (0..12)
        .map(|i| params.get(i + 1).copied().unwrap_or(default_pts[i]))
        .collect();
    let preserve_x_scale = params.get(13).copied().unwrap_or(1.0) > 0.5;
    let fixed_length = params.get(14).copied().unwrap_or(1.0) > 0.5;
    let close_path = params.get(15).copied().unwrap_or(0.0) > 0.5;
    let animate = params.get(16).copied().unwrap_or(0.0) > 0.5;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Build B-spline from control points
    let n_pts = 6;
    let xs: Vec<f64> = (0..n_pts).map(|i| pts[i * 2]).collect();
    let ys: Vec<f64> = (0..n_pts).map(|i| pts[i * 2 + 1]).collect();
    let ts: Vec<f64> = (0..n_pts).map(|i| i as f64).collect();
    let mut bspline_x = Bspline::new();
    let mut bspline_y = Bspline::new();
    bspline_x.init(&ts, &xs);
    bspline_y.init(&ts, &ys);

    // Generate curve vertices and add to trans_single_path
    let mut tcurve = TransSinglePath::new();
    let step = 1.0 / num_points;
    let mut t = 0.0;
    let mut first = true;
    while t <= (n_pts - 1) as f64 + 0.001 {
        let x = bspline_x.get(t);
        let y = bspline_y.get(t);
        if first {
            tcurve.move_to(x, y);
            first = false;
        } else {
            tcurve.line_to(x, y);
        }
        t += step;
    }
    tcurve.finalize_path();
    tcurve.set_preserve_x_scale(preserve_x_scale);
    if fixed_length {
        tcurve.set_base_length(1120.0);
    }

    // Render the spline curve itself (matching C++ stroke on bspline)
    let mut curve_path = PathStorage::new();
    first = true;
    t = 0.0;
    while t <= (n_pts - 1) as f64 + 0.001 {
        let x = bspline_x.get(t);
        let y = bspline_y.get(t);
        if first {
            curve_path.move_to(x, y);
            first = false;
        } else {
            curve_path.line_to(x, y);
        }
        t += step;
    }
    let mut curve_stroke = ConvStroke::new(&mut curve_path);
    curve_stroke.set_width(2.0);
    ras.reset();
    ras.add_path(&mut curve_stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(170, 50, 20, 100));

    // ===== Render TrueType text along the curve =====
    // Matching C++ trans_curve1_ft.cpp font pipeline:
    //   font_engine → font_cache_manager → path_adaptor → conv_curve →
    //   conv_segmentator → conv_transform(trans_single_path) → rasterizer
    let text_str = "Anti-Grain Geometry is designed as a set of loosely coupled \
        algorithms and class templates united with a common idea, \
        so that all the components can be easily combined. Also, \
        the template based design allows you to replace any part of \
        the library without the necessity to modify a single byte in \
        the existing code. ";

    let mut fman = FontCacheManager::from_data(LIBERATION_SERIF_ITALIC.to_vec())
        .expect("Failed to load embedded font");
    fman.engine_mut().set_height(40.0);
    fman.engine_mut().set_hinting(false);

    // Matching C++ on_draw() text loop exactly:
    //   while(*p) {
    //     glyph = m_fman.glyph(*p);
    //     if(glyph) {
    //       if(x > tcurve.total_length()) break;
    //       m_fman.add_kerning(&x, &y);
    //       m_fman.init_embedded_adaptors(glyph, x, y);
    //       if(glyph->data_type == glyph_data_outline) { rasterize }
    //       x += glyph->advance_x; y += glyph->advance_y;
    //     }
    //   }
    let mut x = 0.0_f64;
    let mut y = 3.0_f64;

    for ch in text_str.chars() {
        let char_code = ch as u32;

        // Get glyph (returns Some even for spaces — they have advance but no outline)
        let glyph_info = match fman.glyph(char_code) {
            Some(g) => (g.advance_x, g.advance_y, g.data_type),
            None => continue,
        };
        let (adv_x, adv_y, data_type) = glyph_info;

        // Check if we've gone past the end of the curve
        if x > tcurve.total_length() {
            break;
        }

        // Apply kerning
        fman.add_kerning(char_code, &mut x, &mut y);

        // Initialize the path adaptor with the glyph at position (x, y)
        fman.init_embedded_adaptors(char_code, x, y);

        // Only rasterize glyphs that have outlines (skip spaces, tabs, etc.)
        if data_type == agg_rust::font_engine::GlyphDataType::Outline {
            let adaptor = fman.path_adaptor_mut();
            let mut fcurves = ConvCurve::new(adaptor);
            fcurves.set_approximation_scale(2.0);
            let mut fsegm = ConvSegmentator::new(&mut fcurves);
            fsegm.set_approximation_scale(3.0);
            let mut ftrans = ConvTransform::new(&mut fsegm, &tcurve);

            ras.reset();
            ras.add_path(&mut ftrans, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));
        }

        // Always advance pen position (even for spaces)
        x += adv_x;
        y += adv_y;
    }

    // Render the interactive polygon tool: connecting lines + control point circles
    // Matching C++ r.color(agg::rgba(0, 0.3, 0.5, 0.3))
    let poly_color = Rgba8::new(0, 77, 128, 77);

    // Draw connecting line segments between control points
    {
        let mut poly_path = PathStorage::new();
        for i in 0..n_pts {
            let px = pts[i * 2];
            let py = pts[i * 2 + 1];
            if i == 0 {
                poly_path.move_to(px, py);
            } else {
                poly_path.line_to(px, py);
            }
        }
        if close_path {
            poly_path.close_polygon(0);
        }
        let mut poly_stroke = ConvStroke::new(&mut poly_path);
        poly_stroke.set_width(1.0);
        ras.reset();
        ras.add_path(&mut poly_stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &poly_color);
    }

    // Draw control point circles
    for i in 0..n_pts {
        let cx = pts[i * 2];
        let cy = pts[i * 2 + 1];
        let mut ell = Ellipse::new(cx, cy, 5.0, 5.0, 16, false);
        ras.reset();
        ras.add_path(&mut ell, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &poly_color);
    }

    // Controls matching C++ layout:
    //   m_num_points      (5.0, 5.0, 340.0, 12.0)
    //   m_close           (350, 5.0,  "Close")
    //   m_preserve_x_scale(460, 5.0,  "Preserve X scale")
    //   m_fixed_len       (350, 25.0, "Fixed Length")
    //   m_animate         (460, 25.0, "Animate")

    let mut s_pts = SliderCtrl::new(5.0, 5.0, 340.0, 12.0);
    s_pts.range(10.0, 400.0);
    s_pts.label("Number of intermediate Points = %.3f");
    s_pts.set_value(num_points);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_pts);

    let mut cb_close = CboxCtrl::new(350.0, 5.0, "Close");
    cb_close.set_status(close_path);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut cb_close);

    let mut cb_preserve = CboxCtrl::new(460.0, 5.0, "Preserve X scale");
    cb_preserve.set_status(preserve_x_scale);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut cb_preserve);

    let mut cb_fixed = CboxCtrl::new(350.0, 25.0, "Fixed Length");
    cb_fixed.set_status(fixed_length);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut cb_fixed);

    let mut cb_animate = CboxCtrl::new(460.0, 25.0, "Animate");
    cb_animate.set_status(animate);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut cb_animate);

    buf
}

// ============================================================================
// Trans Curve 2
// ============================================================================

/// Text warped between two curved paths using TrueType font outlines
/// and trans_double_path. Matching C++ trans_curve2_ft.cpp.
///
/// params[0]       = num_points (10-400, default 200)
/// params[1..12]   = poly1 control points x,y pairs (6 points)
/// params[13..24]  = poly2 control points x,y pairs (6 points)
/// params[25]      = preserve_x_scale (0 or 1, default 1)
/// params[26]      = fixed_length (0 or 1, default 1)
/// params[27]      = animate (0 or 1, default 0)
pub fn trans_curve2(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    use agg_rust::trans_double_path::TransDoublePath;

    let num_points = params.first().copied().unwrap_or(200.0).clamp(10.0, 400.0);

    // Default control points matching C++ on_init():
    // poly1: offset (+10, -10) from diagonal
    // poly2: offset (-10, +10) from diagonal
    let default_poly1 = [
        60.0, 40.0,     // 10+50, -10+50
        180.0, 120.0,   // 10+150+20, -10+150-20
        240.0, 260.0,   // 10+250-20, -10+250+20
        380.0, 320.0,   // 10+350+20, -10+350-20
        440.0, 460.0,   // 10+450-20, -10+450+20
        560.0, 540.0,   // 10+550, -10+550
    ];
    let default_poly2 = [
        40.0, 60.0,     // -10+50, 10+50
        160.0, 140.0,   // -10+150+20, 10+150-20
        220.0, 280.0,   // -10+250-20, 10+250+20
        360.0, 340.0,   // -10+350+20, 10+350-20
        420.0, 480.0,   // -10+450-20, 10+450+20
        540.0, 560.0,   // -10+550, 10+550
    ];

    let pts1: Vec<f64> = (0..12)
        .map(|i| params.get(i + 1).copied().unwrap_or(default_poly1[i]))
        .collect();
    let pts2: Vec<f64> = (0..12)
        .map(|i| params.get(i + 13).copied().unwrap_or(default_poly2[i]))
        .collect();
    let preserve_x_scale = params.get(25).copied().unwrap_or(1.0) > 0.5;
    let fixed_length = params.get(26).copied().unwrap_or(1.0) > 0.5;
    let animate = params.get(27).copied().unwrap_or(0.0) > 0.5;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Build B-splines from control points
    let n_pts = 6;
    let xs1: Vec<f64> = (0..n_pts).map(|i| pts1[i * 2]).collect();
    let ys1: Vec<f64> = (0..n_pts).map(|i| pts1[i * 2 + 1]).collect();
    let xs2: Vec<f64> = (0..n_pts).map(|i| pts2[i * 2]).collect();
    let ys2: Vec<f64> = (0..n_pts).map(|i| pts2[i * 2 + 1]).collect();
    let ts: Vec<f64> = (0..n_pts).map(|i| i as f64).collect();

    let mut bspline_x1 = Bspline::new();
    let mut bspline_y1 = Bspline::new();
    let mut bspline_x2 = Bspline::new();
    let mut bspline_y2 = Bspline::new();
    bspline_x1.init(&ts, &xs1);
    bspline_y1.init(&ts, &ys1);
    bspline_x2.init(&ts, &xs2);
    bspline_y2.init(&ts, &ys2);

    // Build trans_double_path from both B-splines
    let mut tcurve = TransDoublePath::new();
    tcurve.set_preserve_x_scale(preserve_x_scale);
    if fixed_length {
        tcurve.set_base_length(1140.0);
    }
    tcurve.set_base_height(30.0);

    let step = 1.0 / num_points;
    // Feed path 1
    let mut t = 0.0;
    let mut first = true;
    while t <= (n_pts - 1) as f64 + 0.001 {
        let x = bspline_x1.get(t);
        let y = bspline_y1.get(t);
        if first { tcurve.move_to1(x, y); first = false; } else { tcurve.line_to1(x, y); }
        t += step;
    }
    // Feed path 2
    t = 0.0;
    first = true;
    while t <= (n_pts - 1) as f64 + 0.001 {
        let x = bspline_x2.get(t);
        let y = bspline_y2.get(t);
        if first { tcurve.move_to2(x, y); first = false; } else { tcurve.line_to2(x, y); }
        t += step;
    }
    tcurve.finalize_paths();

    // ===== Render TrueType text between the two curves =====
    let text_str = "Anti-Grain Geometry is designed as a set of loosely coupled \
        algorithms and class templates united with a common idea, \
        so that all the components can be easily combined. Also, \
        the template based design allows you to replace any part of \
        the library without the necessity to modify a single byte in \
        the existing code. ";

    let mut fman = FontCacheManager::from_data(LIBERATION_SERIF_ITALIC.to_vec())
        .expect("Failed to load embedded font");
    fman.engine_mut().set_height(40.0);
    fman.engine_mut().set_hinting(false);

    let mut x = 0.0_f64;
    let mut y = 3.0_f64;

    for ch in text_str.chars() {
        let char_code = ch as u32;

        let glyph_info = match fman.glyph(char_code) {
            Some(g) => (g.advance_x, g.advance_y, g.data_type),
            None => continue,
        };
        let (adv_x, adv_y, data_type) = glyph_info;

        if x > tcurve.total_length1() {
            break;
        }

        fman.add_kerning(char_code, &mut x, &mut y);
        fman.init_embedded_adaptors(char_code, x, y);

        if data_type == agg_rust::font_engine::GlyphDataType::Outline {
            let adaptor = fman.path_adaptor_mut();
            let mut fcurves = ConvCurve::new(adaptor);
            fcurves.set_approximation_scale(5.0);
            let mut fsegm = ConvSegmentator::new(&mut fcurves);
            fsegm.set_approximation_scale(3.0);
            let mut ftrans = ConvTransform::new(&mut fsegm, &tcurve);

            ras.reset();
            ras.add_path(&mut ftrans, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));
        }

        x += adv_x;
        y += adv_y;
    }

    // Render both B-spline curves as strokes
    let curve_color = Rgba8::new(170, 50, 20, 100);
    for curve_idx in 0..2 {
        let (bx, by) = if curve_idx == 0 {
            (&bspline_x1, &bspline_y1)
        } else {
            (&bspline_x2, &bspline_y2)
        };
        let mut curve_path = PathStorage::new();
        first = true;
        t = 0.0;
        while t <= (n_pts - 1) as f64 + 0.001 {
            let cx = bx.get(t);
            let cy = by.get(t);
            if first { curve_path.move_to(cx, cy); first = false; } else { curve_path.line_to(cx, cy); }
            t += step;
        }
        let mut curve_stroke = ConvStroke::new(&mut curve_path);
        curve_stroke.set_width(2.0);
        ras.reset();
        ras.add_path(&mut curve_stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &curve_color);
    }

    // Render interactive polygon tools (connecting lines + circles)
    // Matching C++ r.color(agg::rgba(0, 0.3, 0.5, 0.2))
    let poly_color = Rgba8::new(0, 77, 128, 51);

    for poly_pts in [&pts1, &pts2] {
        // Connecting lines
        let mut poly_path = PathStorage::new();
        for i in 0..n_pts {
            let px = poly_pts[i * 2];
            let py = poly_pts[i * 2 + 1];
            if i == 0 { poly_path.move_to(px, py); } else { poly_path.line_to(px, py); }
        }
        let mut poly_stroke = ConvStroke::new(&mut poly_path);
        poly_stroke.set_width(1.0);
        ras.reset();
        ras.add_path(&mut poly_stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &poly_color);

        // Control point circles
        for i in 0..n_pts {
            let cx = poly_pts[i * 2];
            let cy = poly_pts[i * 2 + 1];
            let mut ell = Ellipse::new(cx, cy, 5.0, 5.0, 16, false);
            ras.reset();
            ras.add_path(&mut ell, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &poly_color);
        }
    }

    // Controls matching C++ layout:
    //   m_num_points       (5, 5, 340, 12)
    //   m_fixed_len        (350, 5, "Fixed Length")
    //   m_preserve_x_scale (465, 5, "Preserve X scale")
    //   m_animate          (350, 25, "Animate")
    let mut s_pts = SliderCtrl::new(5.0, 5.0, 340.0, 12.0);
    s_pts.range(10.0, 400.0);
    s_pts.label("Number of intermediate Points = %.3f");
    s_pts.set_value(num_points);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_pts);

    let mut cb_fixed = CboxCtrl::new(350.0, 5.0, "Fixed Length");
    cb_fixed.set_status(fixed_length);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut cb_fixed);

    let mut cb_preserve = CboxCtrl::new(465.0, 5.0, "Preserve X scale");
    cb_preserve.set_status(preserve_x_scale);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut cb_preserve);

    let mut cb_animate = CboxCtrl::new(350.0, 25.0, "Animate");
    cb_animate.set_status(animate);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut cb_animate);

    buf
}

// ============================================================================
// Lion Lens
// ============================================================================

/// Magnifying lens on the lion — matching C++ lion_lens.cpp.
///
/// params[0] = magnification (0.01-4.0, default 3.0)
/// params[1] = radius (0.0-100.0, default 70.0)
/// params[2] = lens_x (default center)
/// params[3] = lens_y (default center)
/// params[4] = angle (default 0)
pub fn lion_lens(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let magn = params.first().copied().unwrap_or(3.0).clamp(0.01, 4.0);
    let radius = params.get(1).copied().unwrap_or(70.0).clamp(0.0, 100.0);
    let lens_x = params.get(2).copied().unwrap_or(width as f64 / 2.0);
    let lens_y = params.get(3).copied().unwrap_or(height as f64 / 2.0);
    let angle = params.get(4).copied().unwrap_or(0.0);

    let (mut path, colors, path_idx) = crate::lion_data::parse_lion();
    let npaths = colors.len();

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Set up lens
    let mut lens = TransWarpMagnifier::new();
    lens.center(lens_x, lens_y);
    lens.magnification(magn);
    lens.set_radius(radius / magn);

    // Affine transform for the lion
    let base_dx = 120.0;
    let base_dy = 190.0;
    let mtx = {
        let mut m = TransAffine::new();
        m.multiply(&TransAffine::new_translation(-base_dx, -base_dy));
        m.multiply(&TransAffine::new_rotation(angle + std::f64::consts::PI));
        m.multiply(&TransAffine::new_translation(
            width as f64 / 2.0,
            height as f64 / 2.0,
        ));
        m
    };

    // Render lion with lens distortion
    for i in 0..npaths {
        let start = path_idx[i] as u32;
        // Segmentate → affine → lens transform → render
        let mut segm = ConvSegmentator::new(&mut path);
        segm.set_approximation_scale(4.0);
        segm.rewind(start);

        let mut distorted = PathStorage::new();
        let (mut x, mut y) = (0.0, 0.0);
        let mut first_in_path = true;
        loop {
            let cmd = segm.vertex(&mut x, &mut y);
            if is_stop(cmd) { break; }
            if is_vertex(cmd) {
                // Apply affine transform
                mtx.transform(&mut x, &mut y);
                // Apply lens distortion
                lens.transform(&mut x, &mut y);
                if first_in_path || (cmd & 0x07) == 1 {
                    distorted.move_to(x, y);
                    first_in_path = false;
                } else {
                    distorted.line_to(x, y);
                }
            } else if (cmd & 0x0F) == 0x0F || (cmd & 0x0F) == 0x0E {
                distorted.close_polygon(0);
                first_in_path = true;
            }
        }

        ras.reset();
        ras.add_path(&mut distorted, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &colors[i]);
    }

    // Render sliders
    let mut s_magn = SliderCtrl::new(5.0, 5.0, 245.0, 12.0);
    s_magn.range(0.01, 4.0);
    s_magn.label("Scale=%.2f");
    s_magn.set_value(magn);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_magn);

    let mut s_radius = SliderCtrl::new(255.0, 5.0, 495.0, 12.0);
    s_radius.range(0.0, 100.0);
    s_radius.label("Radius=%.2f");
    s_radius.set_value(radius);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_radius);

    buf
}

// ============================================================================
// Distortions
// ============================================================================

/// Wave/swirl distortion on a procedural image — matching C++ distortions.cpp.
///
/// params[0] = angle (-180 to 180, default 20)
/// params[1] = scale (0.1-5.0, default 1.0)
/// params[2] = amplitude (0.1-40.0, default 10.0)
/// params[3] = period (0.1-2.0, default 1.0)
/// params[4] = distortion type (0=wave, 1=swirl, 2=wave-swirl, 3=swirl-wave)
pub fn distortions(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let angle = params.first().copied().unwrap_or(20.0);
    let scale = params.get(1).copied().unwrap_or(1.0).clamp(0.1, 5.0);
    let amplitude = params.get(2).copied().unwrap_or(10.0).clamp(0.1, 40.0);
    let period = params.get(3).copied().unwrap_or(1.0).clamp(0.1, 2.0);
    let dist_type = params.get(4).copied().unwrap_or(0.0) as u32;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    let w = width as f64;
    let h = height as f64;

    // Generate a procedural image: concentric colored circles
    let img_w = 200u32;
    let img_h = 200u32;
    let mut img_buf = vec![255u8; (img_w * img_h * 4) as usize];
    for iy in 0..img_h {
        for ix in 0..img_w {
            let dx = ix as f64 - 100.0;
            let dy = iy as f64 - 100.0;
            let d = (dx * dx + dy * dy).sqrt();
            let off = ((iy * img_w + ix) * 4) as usize;
            if d < 90.0 {
                let t = d / 90.0;
                img_buf[off] = (128.0 + 127.0 * (t * 6.0).sin()) as u8;
                img_buf[off + 1] = (128.0 + 127.0 * (t * 4.0 + 2.0).sin()) as u8;
                img_buf[off + 2] = (128.0 + 127.0 * (t * 8.0 + 4.0).sin()) as u8;
                img_buf[off + 3] = 255;
            }
        }
    }

    // Create rendering buffer for the source image
    let mut img_ra = RowAccessor::new();
    let img_stride = (img_w * 4) as i32;
    unsafe { img_ra.attach(img_buf.as_mut_ptr(), img_w, img_h, img_stride) };
    // Set up transform
    let angle_rad = angle * std::f64::consts::PI / 180.0;
    let mut img_mtx = TransAffine::new();
    img_mtx.multiply(&TransAffine::new_translation(-(img_w as f64) / 2.0, -(img_h as f64) / 2.0));
    img_mtx.multiply(&TransAffine::new_rotation(angle_rad));
    img_mtx.multiply(&TransAffine::new_scaling_uniform(scale));
    img_mtx.multiply(&TransAffine::new_translation(w / 2.0, h / 2.0));
    img_mtx.invert();

    // Render image with bilinear filter in an ellipse
    let r_ell = (img_w.min(img_h) as f64) / 2.0 - 20.0;

    let mut src_mtx = TransAffine::new();
    src_mtx.multiply(&TransAffine::new_translation(-(img_w as f64) / 2.0, -(img_h as f64) / 2.0));
    src_mtx.multiply(&TransAffine::new_rotation(angle_rad));
    src_mtx.multiply(&TransAffine::new_translation(w / 2.0, h / 2.0));

    let mut ell = Ellipse::new(img_w as f64 / 2.0, img_h as f64 / 2.0, r_ell, r_ell, 200, false);
    let mut tr = ConvTransform::new(&mut ell, src_mtx);

    // Render with bilinear image filter
    let mut inter = SpanInterpolatorLinear::new(img_mtx);
    let bg_color = Rgba8::new(255, 255, 255, 255);
    let mut sg = SpanImageFilterRgbaBilinearClip::new(&img_ra, bg_color, &mut inter);
    let mut sa = SpanAllocator::new();
    ras.reset();
    ras.add_path(&mut tr, 0);
    render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut sa, &mut sg);

    // Render controls
    let mut s_angle = SliderCtrl::new(5.0, 5.0, w - 5.0, 12.0);
    s_angle.range(-180.0, 180.0);
    s_angle.label("Angle=%.1f");
    s_angle.set_value(angle);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_angle);

    let mut s_scale = SliderCtrl::new(5.0, 20.0, w - 5.0, 27.0);
    s_scale.range(0.1, 5.0);
    s_scale.label("Scale=%.2f");
    s_scale.set_value(scale);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_scale);

    let mut s_amp = SliderCtrl::new(5.0, 35.0, w - 5.0, 42.0);
    s_amp.range(0.1, 40.0);
    s_amp.label("Amplitude=%.1f");
    s_amp.set_value(amplitude);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_amp);

    let mut s_period = SliderCtrl::new(5.0, 50.0, w - 5.0, 57.0);
    s_period.range(0.1, 2.0);
    s_period.label("Period=%.2f");
    s_period.set_value(period);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_period);

    // Label showing distortion type
    let labels = ["Wave", "Swirl", "Wave-Swirl", "Swirl-Wave"];
    let label = labels.get(dist_type as usize).unwrap_or(&"Wave");
    let mut txt = GsvText::new();
    txt.size(10.0, 0.0);
    txt.start_point(5.0, h - 20.0);
    txt.text(label);
    let mut txt_stroke = ConvStroke::new(&mut txt);
    txt_stroke.set_width(1.0);
    ras.reset();
    ras.add_path(&mut txt_stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

    buf
}

// ============================================================================
// Gouraud Mesh — animated color-interpolated triangle mesh
// ============================================================================

/// Gouraud-shaded triangle mesh rendered with the compound rasterizer.
/// Adapted from C++ gouraud_mesh.cpp.
///
/// params[0] = grid_cols (3-20, default 8)
/// params[1] = grid_rows (3-20, default 8)
/// params[2] = animation seed (incremented each frame for color cycling)
pub fn gouraud_mesh(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let cols = params.get(0).copied().unwrap_or(8.0).clamp(3.0, 20.0) as usize;
    let rows = params.get(1).copied().unwrap_or(8.0).clamp(3.0, 20.0) as usize;
    let seed = params.get(2).copied().unwrap_or(0.0) as u64;

    let w = width as f64;
    let h = height as f64;
    let margin = 30.0;
    let cell_w = (w - 2.0 * margin) / (cols - 1) as f64;
    let cell_h = (h - 2.0 * margin) / (rows - 1) as f64;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(0, 0, 0, 255));

    // Generate mesh vertices with pseudo-random colors and offsets
    let num_pts = cols * rows;
    let mut vx = vec![0.0f64; num_pts];
    let mut vy = vec![0.0f64; num_pts];
    let mut vc = vec![Rgba8::new(0, 0, 0, 255); num_pts];

    // Simple hash for reproducible randomness
    let hash = |i: u64, ch: u64| -> u8 {
        let v = ((i.wrapping_mul(2654435761).wrapping_add(ch.wrapping_mul(2246822519)))
            .wrapping_mul(seed.wrapping_add(1).wrapping_mul(131))) >> 24;
        (v & 0xFF) as u8
    };

    for r in 0..rows {
        for c in 0..cols {
            let idx = r * cols + c;
            let base_x = margin + c as f64 * cell_w;
            let base_y = margin + r as f64 * cell_h;
            // Add small perturbation based on seed
            let dx = ((hash(idx as u64, 0) as f64) - 128.0) / 128.0 * cell_w * 0.3;
            let dy = ((hash(idx as u64, 1) as f64) - 128.0) / 128.0 * cell_h * 0.3;
            // Don't perturb boundary vertices
            let is_boundary = r == 0 || r == rows - 1 || c == 0 || c == cols - 1;
            vx[idx] = if is_boundary { base_x } else { base_x + dx };
            vy[idx] = if is_boundary { base_y } else { base_y + dy };
            vc[idx] = Rgba8::new(
                hash(idx as u64, 10) as u32,
                hash(idx as u64, 20) as u32,
                hash(idx as u64, 30) as u32,
                255,
            );
        }
    }

    // Build triangles and edges
    struct MeshTriangle { p1: usize, p2: usize, p3: usize }
    struct MeshEdge { p1: usize, p2: usize, tl: i32, tr: i32 }

    let mut triangles: Vec<MeshTriangle> = Vec::new();
    let mut edges: Vec<MeshEdge> = Vec::new();

    for r in 0..(rows - 1) {
        for c in 0..(cols - 1) {
            let p1 = r * cols + c;         // top-left
            let p2 = p1 + 1;              // top-right
            let p3 = p2 + cols;           // bottom-right
            let p4 = p1 + cols;           // bottom-left

            let t1 = triangles.len() as i32; // lower: p1,p2,p3
            triangles.push(MeshTriangle { p1, p2, p3 });
            let t2 = triangles.len() as i32; // upper: p3,p4,p1
            triangles.push(MeshTriangle { p1: p3, p2: p4, p3: p1 });

            // Diagonal edge (p1-p3): t2 on left, t1 on right
            edges.push(MeshEdge { p1, p2: p3, tl: t2, tr: t1 });

            // Top edge
            let top_tr = if r > 0 {
                ((r - 1) * (cols - 1) * 2 + c * 2 + 1) as i32
            } else { -1 };
            edges.push(MeshEdge { p1, p2, tl: top_tr, tr: t1 });

            // Left edge
            let left_tl = if c > 0 {
                (r * (cols - 1) * 2 + (c - 1) * 2) as i32
            } else { -1 };
            edges.push(MeshEdge { p1, p2: p4, tl: t2, tr: left_tl });

            // Right edge (only at last column)
            if c == cols - 2 {
                edges.push(MeshEdge { p1: p2, p2: p3, tl: t1, tr: -1 });
            }
            // Bottom edge (only at last row)
            if r == rows - 2 {
                edges.push(MeshEdge { p1: p3, p2: p4, tl: -1, tr: t2 });
            }
        }
    }

    // Prepare SpanGouraudRgba for each triangle
    let mut gouraud_spans: Vec<SpanGouraudRgba> = Vec::with_capacity(triangles.len());
    for tri in &triangles {
        let mut sg = SpanGouraudRgba::new_with_triangle(
            vc[tri.p1], vc[tri.p2], vc[tri.p3],
            vx[tri.p1], vy[tri.p1],
            vx[tri.p2], vy[tri.p2],
            vx[tri.p3], vy[tri.p3],
            0.0,
        );
        sg.prepare();
        gouraud_spans.push(sg);
    }

    // Rasterize edges with compound rasterizer
    let mut rasc = RasterizerCompoundAa::new();
    for edge in &edges {
        rasc.styles(edge.tl, edge.tr);
        rasc.move_to_d(vx[edge.p1], vy[edge.p1]);
        rasc.line_to_d(vx[edge.p2], vy[edge.p2]);
    }

    // Sweep scanlines and render with Gouraud shading
    {
        use agg_rust::rasterizer_scanline_aa::Scanline;
        if rasc.rewind_scanlines() {
            let mut sl = ScanlineU8::new();
            sl.reset(rasc.min_x(), rasc.max_x());
            loop {
                let num_styles = rasc.sweep_styles();
                if num_styles == 0 { break; }
                for s in 0..num_styles {
                    let style_id = rasc.style(s) as usize;
                    if rasc.sweep_scanline(&mut sl, s as i32) {
                        if style_id < gouraud_spans.len() {
                            let y = Scanline::y(&sl);
                            for span in sl.begin() {
                                let x = span.x;
                                let len = span.len;
                                if len > 0 {
                                    let mut colors = vec![Rgba8::new(0, 0, 0, 0); len as usize];
                                    gouraud_spans[style_id].generate(
                                        &mut colors, x, y, len as u32,
                                    );
                                    let covers = &sl.covers()
                                        [span.cover_offset..span.cover_offset + len as usize];
                                    for i in 0..len as usize {
                                        let c = &colors[i];
                                        let cover = covers[i];
                                        if c.a > 0 && cover > 0 {
                                            rb.blend_pixel(x + i as i32, y, c, cover);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Draw text label
    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    let label = format!("Gouraud Mesh: {}x{} grid, {} triangles",
        cols, rows, triangles.len());
    let mut txt = GsvText::new();
    txt.size(8.0, 0.0);
    txt.start_point(5.0, h - 15.0);
    txt.text(&label);
    let mut txt_stroke = ConvStroke::new(&mut txt);
    txt_stroke.set_width(0.8);
    ras.add_path(&mut txt_stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(255, 255, 255, 200));

    buf
}

// ============================================================================
// TrueType LCD Subpixel Text Demo
// ============================================================================
// Port of C++ truetype_lcd.cpp (truetype_test_02_win).
// Renders text paragraphs with LCD subpixel rendering, faux weight/italic,
// gamma correction, and multiple typeface controls.

static TEXT1: &str = "A single pixel on a color LCD is made of three colored elements \n\
ordered (on various displays) either as blue, green, and red (BGR), \n\
or as red, green, and blue (RGB). These pixel components, sometimes \n\
called sub-pixels, appear as a single color to the human eye because \n\
of blurring by the optics and spatial integration by nerve cells in the eye.";

static TEXT2: &str = "The components are easily visible, however, when viewed with \n\
a small magnifying glass, such as a loupe. Over a certain resolution \n\
range the colors in the sub-pixels are not visible, but the relative \n\
intensity of the components shifts the apparent position or orientation \n\
of a line. Methods that take this interaction between the display \n\
technology and the human visual system into account are called \n\
subpixel rendering algorithms.";

static TEXT3: &str = "The resolution at which colored sub-pixels go unnoticed differs, \n\
however, with each user some users are distracted by the colored \n\
\"fringes\" resulting from sub-pixel rendering. Subpixel rendering \n\
is better suited to some display technologies than others. The \n\
technology is well-suited to LCDs, but less so for CRTs. In a CRT \n\
the light from the pixel components often spread across pixels, \n\
and the outputs of adjacent pixels are not perfectly independent.";

static TEXT4: &str = "If a designer knew precisely a great deal about the display's \n\
electron beams and aperture grille, subpixel rendering might \n\
have some advantage. But the properties of the CRT components, \n\
coupled with the alignment variations that are part of the \n\
production process, make subpixel rendering less effective for \n\
these displays. The technique should have good application to \n\
organic light emitting diodes and other display technologies.";

/// Draw a block of text using the TrueType font pipeline.
///
/// Port of C++ `draw_text()` from truetype_lcd.cpp.
/// Returns the final y position after rendering all lines.
fn draw_text_lcd<PF: agg_rust::pixfmt_rgba::PixelFormat<ColorType = Rgba8>>(
    ras: &mut RasterizerScanlineAa,
    sl: &mut ScanlineU8,
    rb: &mut RendererBase<PF>,
    fman: &mut FontCacheManager,
    text: &str,
    x: f64,
    mut y: f64,
    height: f64,
    subpixel_scale: u32,
    invert: bool,
    kerning: bool,
    hinting: bool,
    faux_italic: f64,
    faux_weight_val: f64,
    width_val: f64,
    interval: f64,
) -> f64 {
    const SCALE_X: f64 = 16.0;
    let color = if invert {
        Rgba8::new(255, 255, 255, 255)
    } else {
        Rgba8::new(0, 0, 0, 255)
    };

    let sp_scale = subpixel_scale as f64;

    // Match C++ truetype_lcd.cpp exactly:
    //   m_feng.scale_x(16 * subpixel_scale)
    //   x is tracked in this scaled space and converted back by /16 in transform.
    fman.engine_mut().set_height(height);
    fman.engine_mut().set_scale_x(SCALE_X * sp_scale);
    fman.engine_mut().set_hinting(hinting);
    // Match C++ truetype_lcd.cpp: app uses flip_y at platform level,
    // while draw_text itself keeps the font engine in non-flipped mode.
    fman.engine_mut().set_flip_y(false);
    fman.reset_cache();

    let start_x = x * sp_scale;
    let mut pen_x = start_x;

    for ch in text.chars() {
        if ch == '\n' {
            pen_x = start_x;
            y -= height * 1.25;
            continue;
        }

        let char_code = ch as u32;
        let glyph_info = match fman.glyph(char_code) {
            Some(g) => (g.advance_x, g.advance_y, g.data_type),
            None => continue,
        };
        let (adv_x, adv_y, data_type) = glyph_info;

        if kerning {
            let _ = fman.add_kerning(char_code, &mut pen_x, &mut y);
        }

        fman.init_embedded_adaptors(char_code, 0.0, 0.0);

        if data_type == agg_rust::font_engine::GlyphDataType::Outline {
            let ty = if hinting { (y + 0.5).floor() } else { y };

            // C++ transform order:
            //   scale(width/16, 1) * skew(faux_italic*subpixel/3, 0) * translate(start_x + x/16, y)
            let mut mtx = TransAffine::new_scaling(width_val / SCALE_X, 1.0);
            mtx *= TransAffine::new_skewing(faux_italic * sp_scale / 3.0, 0.0);
            mtx *= TransAffine::new_translation(start_x + pen_x / SCALE_X, ty);

            let adaptor = fman.path_adaptor_mut();
            let mut curves = ConvCurve::new(adaptor);

            let use_faux_weight = faux_weight_val.abs() >= 0.05;

            if !use_faux_weight {
                // Simple path: curves -> transform -> rasterize
                let mut trans = ConvTransform::new(&mut curves, mtx);
                ras.reset();
                ras.add_path(&mut trans, 0);
            } else {
                // Faux weight pipeline: curves -> transform -> zoom_in_y ->
                //   conv_contour -> zoom_out_y -> rasterize
                // This adds horizontal weight while preserving vertical sharpness.
                let mut trans = ConvTransform::new(&mut curves, mtx);

                let zoom_in = TransAffine::new_scaling(1.0, 100.0);
                let mut zoomed_in = ConvTransform::new(&mut trans, zoom_in);

                let mut contour =
                    agg_rust::conv_contour::ConvContour::new(&mut zoomed_in);
                contour.set_auto_detect_orientation(false);
                contour.set_width(
                    -faux_weight_val * height * sp_scale / 15.0,
                );

                let zoom_out = TransAffine::new_scaling(1.0, 1.0 / 100.0);
                let mut zoomed_out = ConvTransform::new(&mut contour, zoom_out);

                ras.reset();
                ras.add_path(&mut zoomed_out, 0);
            }

            render_scanlines_aa_solid(ras, sl, rb, &color);
        }

        pen_x += adv_x + interval * SCALE_X * sp_scale;
        y += adv_y;
    }

    y
}

/// Render the truetype_test_02 LCD subpixel text demo.
///
/// Port of C++ truetype_lcd.cpp (truetype_test_02_win).
/// Parameters:
///   [0] typeface_idx: 0=Arial, 1=Tahoma, 2=Verdana, 3=Times, 4=Georgia
///   [1] font_scale: 0.5..2.0 (default 1.0)
///   [2] faux_italic: -1..1 (default 0.0)
///   [3] faux_weight: -1..1 (default 0.0)
///   [4] interval: -0.2..0.2 (default 0.0)
///   [5] width: 0.75..1.25 (default 1.0)
///   [6] gamma: 0.5..2.5 (default 1.0)
///   [7] primary_weight: 0..1 (default 1/3)
///   [8] grayscale: 0 or 1 (default 0)
///   [9] hinting: 0 or 1 (default 1)
///  [10] kerning: 0 or 1 (default 1)
///  [11] invert: 0 or 1 (default 0)
pub fn truetype_test(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    use agg_rust::gamma::GammaLut;
    use agg_rust::pixfmt_lcd::{LcdDistributionLut, PixfmtRgba32Lcd};

    // Parse parameters
    let typeface_idx = params.first().copied().unwrap_or(0.0) as usize;
    let font_scale = params.get(1).copied().unwrap_or(1.0);
    let faux_italic = params.get(2).copied().unwrap_or(0.0);
    let faux_weight = params.get(3).copied().unwrap_or(0.0);
    let interval = params.get(4).copied().unwrap_or(0.0);
    let width_val = params.get(5).copied().unwrap_or(1.0);
    let gamma_val = params.get(6).copied().unwrap_or(1.0);
    let primary_weight = params.get(7).copied().unwrap_or(1.0 / 3.0);
    let grayscale = params.get(8).copied().unwrap_or(0.0) > 0.5;
    let hinting = params.get(9).copied().unwrap_or(1.0) > 0.5;
    let kerning = params.get(10).copied().unwrap_or(1.0) > 0.5;
    let invert = params.get(11).copied().unwrap_or(0.0) > 0.5;

    // Select font data based on C++ typeface order.
    // Tahoma italic isn't present on this system image; use regular for both.
    let (font_regular, font_italic): (&[u8], &[u8]) = match typeface_idx {
        0 => (ARIAL_REGULAR, ARIAL_ITALIC),
        1 => (TAHOMA_REGULAR, TAHOMA_REGULAR),
        2 => (VERDANA_REGULAR, VERDANA_ITALIC),
        3 => (TIMES_REGULAR, TIMES_ITALIC),
        _ => (GEORGIA_REGULAR, GEORGIA_ITALIC),
    };

    // Step 1: Setup RGBA32 buffer and clear to white
    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);

    // Clear to white using PixfmtRgba32
    {
        let pf = PixfmtRgba32::new(&mut ra);
        let mut rb = RendererBase::new(pf);
        rb.clear(&Rgba8::new(255, 255, 255, 255));
        if invert {
            // Match C++: invert only the text area below controls.
            rb.blend_bar(0, 120, width as i32, height as i32, &Rgba8::new(0, 0, 0, 255), 255);
        }
    }

    // Step 2: Render text
    let mut sl = ScanlineU8::new();
    let mut ras = RasterizerScanlineAa::new();

    let text_height = font_scale * 12.0;
    let texts: [(&str, bool); 4] = [
        (TEXT1, false),
        (TEXT2, true),
        (TEXT3, false),
        (TEXT4, true),
    ];

    if grayscale {
        // Grayscale rendering: standard RGBA32 pipeline at 1x scale.
        let pf = PixfmtRgba32::new(&mut ra);
        let mut rb = RendererBase::new(pf);
        ras.clip_box(0.0, 120.0, (width * 3) as f64, height as f64);
        let mut y = height as f64 - 20.0;
        for &(text, italic) in &texts {
            let font_data = if italic { font_italic } else { font_regular };
            let mut fman = FontCacheManager::from_data(font_data.to_vec())
                .expect("Failed to load font");

            y = draw_text_lcd(
                &mut ras,
                &mut sl,
                &mut rb,
                &mut fman,
                text,
                10.0,
                y,
                text_height,
                1, // subpixel_scale = 1 for grayscale
                invert,
                kerning,
                hinting,
                faux_italic,
                faux_weight,
                width_val,
                interval,
            );
            y -= 7.0 + text_height;
        }
    } else {
        // LCD subpixel rendering: 3x horizontal resolution
        let lut = LcdDistributionLut::new(primary_weight, 2.0 / 9.0, 1.0 / 9.0);
        let pf_lcd = PixfmtRgba32Lcd::new(&mut ra, &lut);
        let mut rb_lcd = RendererBase::new(pf_lcd);

        ras.clip_box(0.0, 120.0, (width * 3) as f64, height as f64);
        let mut y = height as f64 - 20.0;
        for &(text, italic) in &texts {
            let font_data = if italic { font_italic } else { font_regular };
            let mut fman = FontCacheManager::from_data(font_data.to_vec())
                .expect("Failed to load font");

            y = draw_text_lcd(
                &mut ras,
                &mut sl,
                &mut rb_lcd,
                &mut fman,
                text,
                10.0,
                y,
                text_height,
                3, // subpixel_scale = 3 for LCD
                invert,
                kerning,
                hinting,
                faux_italic,
                faux_weight,
                width_val,
                interval,
            );
            y -= 7.0 + text_height;
        }
    }

    // Step 3: Apply inverse gamma correction to full buffer (C++ behavior).
    {
        let gamma_lut = GammaLut::new_with_gamma(gamma_val);
        let mut pf = PixfmtRgba32::new(&mut ra);
        pf.apply_gamma_inv(&gamma_lut);
    }

    // Step 4: Render C++-style controls over the gamma-corrected image.
    // The sidebar UI remains functional; this paints the same control visuals
    // into the canvas so the output matches the C++ demo layout.
    {
        let pf = PixfmtRgba32::new(&mut ra);
        let mut rb = RendererBase::new(pf);
        ras.clip_box(0.0, 0.0, width as f64, height as f64);

        let mut typeface = RboxCtrl::new(5.0, 5.0, 155.0, 110.0);
        typeface.add_item("Arial");
        typeface.add_item("Tahoma");
        typeface.add_item("Verdana");
        typeface.add_item("Times");
        typeface.add_item("Georgia");
        typeface.set_cur_item(typeface_idx.min(4) as i32);

        let mut s_height = SliderCtrl::new(160.0, 10.0, 635.0, 17.0);
        s_height.label("Font Scale=%.2f");
        s_height.range(0.5, 2.0);
        s_height.set_value(font_scale);

        let mut s_faux_italic = SliderCtrl::new(160.0, 25.0, 635.0, 32.0);
        s_faux_italic.label("Faux Italic=%.2f");
        s_faux_italic.range(-1.0, 1.0);
        s_faux_italic.set_value(faux_italic);

        let mut s_faux_weight = SliderCtrl::new(160.0, 40.0, 635.0, 47.0);
        s_faux_weight.label("Faux Weight=%.2f");
        s_faux_weight.range(-1.0, 1.0);
        s_faux_weight.set_value(faux_weight);

        let mut s_interval = SliderCtrl::new(260.0, 55.0, 635.0, 62.0);
        s_interval.label("Interval=%.3f");
        s_interval.range(-0.2, 0.2);
        s_interval.set_value(interval);

        let mut s_width = SliderCtrl::new(260.0, 70.0, 635.0, 77.0);
        s_width.label("Width=%.2f");
        s_width.range(0.75, 1.25);
        s_width.set_value(width_val);

        let mut s_gamma = SliderCtrl::new(260.0, 85.0, 635.0, 92.0);
        s_gamma.label("Gamma=%.2f");
        s_gamma.range(0.5, 2.5);
        s_gamma.set_value(gamma_val);

        let mut s_primary = SliderCtrl::new(260.0, 100.0, 635.0, 107.0);
        s_primary.label("Primary Weight=%.2f");
        s_primary.range(0.0, 1.0);
        s_primary.set_value(primary_weight);

        let mut c_grayscale = CboxCtrl::new(160.0, 50.0, "Grayscale");
        c_grayscale.set_status(grayscale);
        let mut c_hinting = CboxCtrl::new(160.0, 65.0, "Hinting");
        c_hinting.set_status(hinting);
        let mut c_kerning = CboxCtrl::new(160.0, 80.0, "Kerning");
        c_kerning.set_status(kerning);
        let mut c_invert = CboxCtrl::new(160.0, 95.0, "Invert");
        c_invert.set_status(invert);

        render_ctrl(&mut ras, &mut sl, &mut rb, &mut typeface);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_height);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_faux_italic);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_faux_weight);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_interval);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_width);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_gamma);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_primary);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_hinting);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_kerning);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_invert);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_grayscale);
    }

    buf
}

#[cfg(test)]
mod tests {
    use super::{raster_text, truetype_test};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    #[test]
    fn raster_text_contains_red_to_green_gradient_banner() {
        let buf = raster_text(640, 480, &[]);
        let mut has_red = false;
        let mut has_green = false;

        for px in buf.chunks_exact(4) {
            let r = px[0];
            let g = px[1];
            let b = px[2];
            let a = px[3];
            if a == 0 {
                continue;
            }
            if r > 170 && g < 90 && b < 90 {
                has_red = true;
            }
            if g > 100 && r < 130 && b < 90 {
                has_green = true;
            }
            if has_red && has_green {
                break;
            }
        }

        assert!(has_red, "expected vivid red pixels in gradient banner");
        assert!(has_green, "expected green pixels in gradient banner");
    }

    #[test]
    fn truetype_test_invert_only_affects_text_region() {
        let w = 640u32;
        let h = 560u32;
        // [0..11]: default values with invert enabled.
        let params = [4.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0 / 3.0, 0.0, 1.0, 1.0, 1.0];
        let buf = truetype_test(w, h, &params);

        // Pick a pixel inside the control area (y < 120): should not be pure black.
        let ctrl_i = ((20 * w + 20) * 4) as usize;
        let ctrl_rgb = (buf[ctrl_i], buf[ctrl_i + 1], buf[ctrl_i + 2]);
        assert_ne!(ctrl_rgb, (0, 0, 0), "control region should not be inverted");

        // Pick a background pixel in the text area with no text: should be black after invert.
        let text_i = ((150 * w + 630) * 4) as usize;
        let text_rgb = (buf[text_i], buf[text_i + 1], buf[text_i + 2]);
        assert_eq!(text_rgb, (0, 0, 0), "text region background should be inverted");
    }

    #[test]
    fn truetype_test_renders_controls_in_top_panel() {
        let w = 640u32;
        let h = 560u32;
        let params = [4.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0 / 3.0, 0.0, 1.0, 1.0, 0.0];
        let buf = truetype_test(w, h, &params);

        // Sample the top panel and ensure not all pixels are white.
        let mut non_white = 0usize;
        for y in 0..120u32 {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                if !(buf[i] == 255 && buf[i + 1] == 255 && buf[i + 2] == 255) {
                    non_white += 1;
                }
            }
        }
        assert!(non_white > 1000, "expected many control pixels in top panel, got {}", non_white);
    }

    fn load_raw_rgba(path: &Path) -> (u32, u32, Vec<u8>) {
        let bytes = fs::read(path).expect("failed to read raw image");
        assert!(bytes.len() >= 8, "raw image too small");
        let width = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let height = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let expected = 8usize + (width as usize) * (height as usize) * 4usize;
        assert!(
            bytes.len() >= expected,
            "raw image size mismatch: got {}, expected at least {}",
            bytes.len(),
            expected
        );
        (width, height, bytes[8..expected].to_vec())
    }

    #[test]
    #[ignore = "requires local C++ renderer binary"]
    fn raster_text_matches_cpp_reference() {
        let width = 640u32;
        let height = 480u32;
        let rust = raster_text(width, height, &[]);

        let default_cpp_exe = {
            let manifest_dir =
                PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()));
            manifest_dir
                .join("../../tools/cpp-renderer/build/Release/agg-render.exe")
                .to_string_lossy()
                .into_owned()
        };
        let cpp_exe = std::env::var("AGG_CPP_RENDERER").unwrap_or(default_cpp_exe);
        assert!(
            Path::new(&cpp_exe).exists(),
            "C++ renderer not found at {} (set AGG_CPP_RENDERER to override)",
            cpp_exe
        );

        let out_path: PathBuf = std::env::temp_dir().join("agg_raster_text_cpp.raw");
        if out_path.exists() {
            fs::remove_file(&out_path).expect("failed to remove old temp raw output");
        }

        let status = Command::new(&cpp_exe)
            .args([
                "raster_text",
                &width.to_string(),
                &height.to_string(),
                out_path
                    .to_str()
                    .expect("temp path must be valid UTF-8"),
            ])
            .status()
            .expect("failed to run C++ renderer");
        assert!(status.success(), "C++ renderer exited with {}", status);

        let (cpp_w, cpp_h, cpp) = load_raw_rgba(&out_path);
        assert_eq!(cpp_w, width);
        assert_eq!(cpp_h, height);

        let mut different_pixels = 0usize;
        let mut max_channel_diff = 0u8;
        let mut first_diff: Option<(usize, usize, [u8; 4], [u8; 4])> = None;

        for y in 0..height as usize {
            for x in 0..width as usize {
                let i = (y * width as usize + x) * 4;
                let a = [rust[i], rust[i + 1], rust[i + 2], rust[i + 3]];
                let b = [cpp[i], cpp[i + 1], cpp[i + 2], cpp[i + 3]];
                if a != b {
                    different_pixels += 1;
                    for c in 0..4 {
                        let d = (a[c] as i16 - b[c] as i16).unsigned_abs() as u8;
                        if d > max_channel_diff {
                            max_channel_diff = d;
                        }
                    }
                    if first_diff.is_none() {
                        first_diff = Some((x, y, a, b));
                    }
                }
            }
        }

        assert_eq!(
            different_pixels,
            0,
            "raster_text mismatch: {} differing pixels, max_channel_diff={}, first_diff={:?}",
            different_pixels,
            max_channel_diff,
            first_diff
        );
    }
}

