// Copyright 2025-2026. Native rasterizers2 renderer for pixel comparison.
//
// This must produce identical output to the WASM version in
// demo/wasm/src/render/compositing.rs::rasterizers2().

use agg_rust::basics::{is_stop, is_vertex, VertexSource, PATH_CMD_LINE_TO, PATH_CMD_MOVE_TO, PATH_CMD_STOP};
use agg_rust::color::Rgba8;
use agg_rust::conv_stroke::ConvStroke;
use agg_rust::ctrl::{render_ctrl, CboxCtrl, SliderCtrl};
use agg_rust::gsv_text::GsvText;
use agg_rust::math_stroke::LineCap;
use agg_rust::path_storage::PathStorage;
use agg_rust::pattern_filters_rgba::PatternFilterBilinearRgba;
use agg_rust::pixfmt_rgba::PixfmtRgba32;
use agg_rust::rasterizer_outline::RasterizerOutline;
use agg_rust::rasterizer_outline_aa::{OutlineAaJoin, RasterizerOutlineAa};
use agg_rust::rasterizer_scanline_aa::RasterizerScanlineAa;
use agg_rust::renderer_base::RendererBase;
use agg_rust::renderer_outline_aa::{LineProfileAa, RendererOutlineAa};
use agg_rust::renderer_outline_image::{
    LineImagePatternPow2, LineImageScale, RendererOutlineImage,
};
use agg_rust::renderer_primitives::RendererPrimitives;
use agg_rust::renderer_scanline::render_scanlines_aa_solid;
use agg_rust::rendering_buffer::RowAccessor;
use agg_rust::scanline_u::ScanlineU8;

// ============================================================================
// Spiral vertex source — matching C++ rasterizers2.cpp spiral class
// ============================================================================

struct Spiral {
    cx: f64,
    cy: f64,
    r1: f64,
    r2: f64,
    da: f64,
    dr: f64,
    start_angle: f64,
    angle: f64,
    curr_r: f64,
    start: bool,
}

impl Spiral {
    fn new(cx: f64, cy: f64, r1: f64, r2: f64, step: f64, start_angle: f64) -> Self {
        let da = (8.0_f64).to_radians();
        Self {
            cx, cy, r1, r2, da,
            dr: step / 45.0,
            start_angle,
            angle: start_angle,
            curr_r: r1,
            start: true,
        }
    }
}

impl VertexSource for Spiral {
    fn rewind(&mut self, _path_id: u32) {
        self.angle = self.start_angle;
        self.curr_r = self.r1;
        self.start = true;
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        if self.curr_r > self.r2 {
            return PATH_CMD_STOP;
        }
        *x = self.cx + self.angle.cos() * self.curr_r;
        *y = self.cy + self.angle.sin() * self.curr_r;
        self.curr_r += self.dr;
        self.angle += self.da;
        if self.start {
            self.start = false;
            PATH_CMD_MOVE_TO
        } else {
            PATH_CMD_LINE_TO
        }
    }
}

// ============================================================================
// Pixmap chain pattern — exact copy from C++ rasterizers2.cpp
// ============================================================================

static PIXMAP_CHAIN: [u32; 114] = [
    16, 7,
    0x00ffffff, 0x00ffffff, 0x00ffffff, 0x00ffffff, 0xb4c29999, 0xff9a5757, 0xff9a5757, 0xff9a5757, 0xff9a5757, 0xff9a5757, 0xff9a5757, 0xb4c29999, 0x00ffffff, 0x00ffffff, 0x00ffffff, 0x00ffffff,
    0x00ffffff, 0x00ffffff, 0x0cfbf9f9, 0xff9a5757, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xb4c29999, 0x00ffffff, 0x00ffffff, 0x00ffffff,
    0x00ffffff, 0x5ae0cccc, 0xffa46767, 0xff660000, 0xff975252, 0x7ed4b8b8, 0x5ae0cccc, 0x5ae0cccc, 0x5ae0cccc, 0x5ae0cccc, 0xa8c6a0a0, 0xff7f2929, 0xff670202, 0x9ecaa6a6, 0x5ae0cccc, 0x00ffffff,
    0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xa4c7a2a2, 0x3affff00, 0x3affff00, 0xff975151, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xff660000,
    0x00ffffff, 0x5ae0cccc, 0xffa46767, 0xff660000, 0xff954f4f, 0x7ed4b8b8, 0x5ae0cccc, 0x5ae0cccc, 0x5ae0cccc, 0x5ae0cccc, 0xa8c6a0a0, 0xff7f2929, 0xff670202, 0x9ecaa6a6, 0x5ae0cccc, 0x00ffffff,
    0x00ffffff, 0x00ffffff, 0x0cfbf9f9, 0xff9a5757, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xb4c29999, 0x00ffffff, 0x00ffffff, 0x00ffffff,
    0x00ffffff, 0x00ffffff, 0x00ffffff, 0x00ffffff, 0xb4c29999, 0xff9a5757, 0xff9a5757, 0xff9a5757, 0xff9a5757, 0xff9a5757, 0xff9a5757, 0xb4c29999, 0x00ffffff, 0x00ffffff, 0x00ffffff, 0x00ffffff,
];

struct PatternPixmapArgb32 {
    pixmap: &'static [u32],
}

impl PatternPixmapArgb32 {
    fn new(pixmap: &'static [u32]) -> Self {
        Self { pixmap }
    }
    fn pw(&self) -> u32 { self.pixmap[0] }
    fn ph(&self) -> u32 { self.pixmap[1] }
}

impl agg_rust::renderer_outline_image::ImagePatternSource for PatternPixmapArgb32 {
    fn width(&self) -> f64 { self.pw() as f64 }
    fn height(&self) -> f64 { self.ph() as f64 }
    fn pixel(&self, x: i32, y: i32) -> Rgba8 {
        let p = self.pixmap[(y as u32 * self.pw() + x as u32 + 2) as usize];
        let r = (p >> 16) & 0xFF;
        let g = (p >> 8) & 0xFF;
        let b = p & 0xFF;
        let a = p >> 24;
        Rgba8::new(r, g, b, a)
    }
}

// ============================================================================
// Render function
// ============================================================================

/// Render the rasterizers2 demo.
///
/// params[0] = step (unused in static render)
/// params[1] = line width
/// params[2] = accurate_joins (0 or 1)
/// params[3] = start_angle (degrees)
/// params[4] = scale_pattern (0 or 1, default 1)
pub fn render(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let line_width = params.get(1).copied().unwrap_or(3.0).max(0.1);
    let accurate_joins = params.get(2).copied().unwrap_or(0.0) > 0.5;
    let start_angle = params.get(3).copied().unwrap_or(0.0).to_radians();
    let scale_pattern = params.get(4).copied().unwrap_or(1.0) > 0.5;

    let w = width as f64;
    let h = height as f64;

    let stride = (width * 4) as i32;
    let mut buf = vec![255u8; (width * height * 4) as usize];
    let mut ra = RowAccessor::new();
    unsafe { ra.attach(buf.as_mut_ptr(), width, height, stride) };
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 242, 255));

    let color = Rgba8::new(102, 77, 26, 255);

    // 1. Aliased pixel accuracy — Bresenham with rounded coords
    {
        let mut prim = RendererPrimitives::new(&mut rb);
        prim.set_line_color(Rgba8::new(102, 77, 26, 255));
        let mut ras_al = RasterizerOutline::new(&mut prim);
        let mut s1 = Spiral::new(w / 5.0, h / 4.0 + 50.0, 5.0, 70.0, 8.0, start_angle);
        let mut px = PathStorage::new();
        s1.rewind(0);
        let (mut vx, mut vy) = (0.0, 0.0);
        loop {
            let cmd = s1.vertex(&mut vx, &mut vy);
            if is_stop(cmd) { break; }
            if is_vertex(cmd) {
                let rx = vx.floor();
                let ry = vy.floor();
                if cmd == PATH_CMD_MOVE_TO {
                    px.move_to(rx, ry);
                } else {
                    px.line_to(rx, ry);
                }
            }
        }
        ras_al.add_path(&mut px, 0);
    }

    // 2. Aliased subpixel accuracy — Bresenham direct
    {
        let mut prim = RendererPrimitives::new(&mut rb);
        prim.set_line_color(Rgba8::new(102, 77, 26, 255));
        let mut ras_al = RasterizerOutline::new(&mut prim);
        let mut s2 = Spiral::new(w / 2.0, h / 4.0 + 50.0, 5.0, 70.0, 8.0, start_angle);
        ras_al.add_path(&mut s2, 0);
    }

    // 3. Anti-aliased outline
    {
        let profile = LineProfileAa::with_width(line_width);
        let mut ren_oaa = RendererOutlineAa::new(&mut rb, &profile);
        ren_oaa.set_color(color);
        let mut ras_oaa = RasterizerOutlineAa::new();
        ras_oaa.set_round_cap(true);
        ras_oaa.set_line_join(if accurate_joins {
            OutlineAaJoin::MiterAccurate
        } else {
            OutlineAaJoin::Round
        });
        let mut s3 = Spiral::new(w / 5.0, h - h / 4.0 + 20.0, 5.0, 70.0, 8.0, start_angle);
        ras_oaa.add_path(&mut s3, 0, &mut ren_oaa);
    }

    // 4. Scanline rasterizer
    {
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();
        let mut s4 = Spiral::new(w / 2.0, h - h / 4.0 + 20.0, 5.0, 70.0, 8.0, start_angle);
        let mut stroke = ConvStroke::new(&mut s4);
        stroke.set_width(line_width);
        stroke.set_line_cap(LineCap::Round);
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &color);
    }

    // 5. Anti-aliased outline with image pattern
    {
        let src = PatternPixmapArgb32::new(&PIXMAP_CHAIN);
        let src_scaled = LineImageScale::new(&src, line_width);

        let pattern = if scale_pattern {
            LineImagePatternPow2::<PatternFilterBilinearRgba>::with_source(&src_scaled)
        } else {
            LineImagePatternPow2::<PatternFilterBilinearRgba>::with_source(&src)
        };

        let mut ren_img = RendererOutlineImage::new(&mut rb, &pattern);
        if scale_pattern {
            ren_img.set_scale_x(line_width / src.ph() as f64);
        }

        let mut ras_img = RasterizerOutlineAa::new();
        let mut s5 = Spiral::new(
            w - w / 5.0, h - h / 4.0 + 20.0, 5.0, 70.0, 8.0, start_angle,
        );
        ras_img.add_path(&mut s5, 0, &mut ren_img);
    }

    // Labels
    {
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();
        let labels = [
            (50.0, 80.0, "Bresenham lines,\n\nregular accuracy"),
            (w / 2.0 - 50.0, 80.0, "Bresenham lines,\n\nsubpixel accuracy"),
            (50.0, h / 2.0 + 50.0, "Anti-aliased lines"),
            (w / 2.0 - 50.0, h / 2.0 + 50.0, "Scanline rasterizer"),
            (w - w / 5.0 - 50.0, h / 2.0 + 50.0, "Arbitrary Image Pattern"),
        ];
        for (lx, ly, txt) in labels {
            let mut t = GsvText::new();
            t.size(8.0, 0.0);
            t.text(txt);
            t.start_point(lx, ly);
            let mut ts = ConvStroke::new(&mut t);
            ts.set_width(0.7);
            ras.reset();
            ras.add_path(&mut ts, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));
        }
    }

    // Controls — match C++ layout
    {
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();

        let mut s_step = SliderCtrl::new(10.0, 14.0, 150.0, 22.0);
        s_step.range(0.0, 2.0);
        s_step.set_value(params.get(0).copied().unwrap_or(0.1));
        s_step.label("Step=%1.2f");
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_step);

        let mut s_width = SliderCtrl::new(150.0 + 10.0, 14.0, 400.0 - 10.0, 22.0);
        s_width.range(0.0, 14.0);
        s_width.set_value(line_width);
        s_width.label("Width=%1.2f");
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_width);

        let mut cbox_test = CboxCtrl::new(10.0, 30.0, "Test Performance");
        cbox_test.text_size(9.0, 7.0);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut cbox_test);

        let mut cbox_rotate = CboxCtrl::new(130.0 + 10.0, 30.0, "Rotate");
        cbox_rotate.text_size(9.0, 7.0);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut cbox_rotate);

        let mut cbox_aj = CboxCtrl::new(200.0 + 10.0, 30.0, "Accurate Joins");
        cbox_aj.text_size(9.0, 7.0);
        cbox_aj.set_status(accurate_joins);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut cbox_aj);

        let mut cbox_sp = CboxCtrl::new(310.0 + 10.0, 30.0, "Scale Pattern");
        cbox_sp.text_size(9.0, 7.0);
        cbox_sp.set_status(scale_pattern);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut cbox_sp);
    }

    buf
}
