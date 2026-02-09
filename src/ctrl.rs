//! Interactive UI controls rendered via AGG's rendering pipeline.
//!
//! Port of `ctrl/agg_slider_ctrl.h`, `ctrl/agg_cbox_ctrl.h`, `ctrl/agg_rbox_ctrl.h`.
//!
//! Controls are vertex sources with multiple colored paths. Use `render_ctrl()`
//! to render them into a renderer_base, which iterates paths and colors.
//!
//! In the original C++ library, controls handle mouse interaction directly.
//! In our WASM demos, the JS sidebar handles interaction and passes values
//! as params; these controls render the visual representation on the canvas.

use crate::basics::{
    is_stop, VertexSource, PATH_CMD_LINE_TO, PATH_CMD_MOVE_TO, PATH_CMD_STOP,
};
use crate::color::Rgba8;
use crate::conv_stroke::ConvStroke;
use crate::ellipse::Ellipse;
use crate::gsv_text::GsvText;
use crate::math_stroke::{LineCap, LineJoin};
use crate::pixfmt_rgba::PixfmtRgba32;
use crate::rasterizer_scanline_aa::RasterizerScanlineAa;
use crate::renderer_base::RendererBase;
use crate::renderer_scanline::render_scanlines_aa_solid;
use crate::scanline_u::ScanlineU8;

// ============================================================================
// render_ctrl — render any control with its multi-path color scheme
// ============================================================================

/// Render a control by iterating its paths and rendering each with its color.
///
/// Port of C++ `render_ctrl()` template function.
pub fn render_ctrl(
    ras: &mut RasterizerScanlineAa,
    sl: &mut ScanlineU8,
    ren: &mut RendererBase<PixfmtRgba32>,
    ctrl: &mut dyn Ctrl,
) {
    for i in 0..ctrl.num_paths() {
        ras.reset();
        ras.add_path(ctrl, i);
        render_scanlines_aa_solid(ras, sl, ren, &ctrl.color(i));
    }
}

/// Trait for AGG controls that can be rendered as multi-path vertex sources.
pub trait Ctrl: VertexSource {
    fn num_paths(&self) -> u32;
    fn color(&self, path_id: u32) -> Rgba8;
}

// ============================================================================
// SliderCtrl — horizontal value slider
// ============================================================================

/// Horizontal slider control rendered via AGG.
///
/// Renders 6 paths: background, triangle indicator, label text,
/// pointer preview, active pointer, and step markers.
///
/// Port of C++ `slider_ctrl_impl`.
pub struct SliderCtrl {
    // Bounds
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    // Inner slider bounds
    xs1: f64,
    ys1: f64,
    xs2: f64,
    ys2: f64,
    // Value (normalized 0..1)
    value: f64,
    preview_value: f64,
    min: f64,
    max: f64,
    // Text
    label: String,
    // Visual
    border_width: f64,
    border_extra: f64,
    text_thickness: f64,
    num_steps: u32,
    descending: bool,
    // Colors (6 paths)
    colors: [Rgba8; 6],
    // Rendering state
    vertices: Vec<(f64, f64, u32)>,
    vertex_idx: usize,
}

impl SliderCtrl {
    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        let border_extra = (y2 - y1) / 2.0;
        let mut s = Self {
            x1,
            y1,
            x2,
            y2,
            xs1: 0.0,
            ys1: 0.0,
            xs2: 0.0,
            ys2: 0.0,
            value: 0.5,
            preview_value: 0.5,
            min: 0.0,
            max: 1.0,
            label: String::new(),
            border_width: 1.0,
            border_extra,
            text_thickness: 1.0,
            num_steps: 0,
            descending: false,
            colors: [
                Rgba8::new(255, 230, 204, 255), // 0: background (1.0, 0.9, 0.8)
                Rgba8::new(179, 153, 153, 255), // 1: triangle (0.7, 0.6, 0.6)
                Rgba8::new(0, 0, 0, 255),       // 2: text (black)
                Rgba8::new(153, 102, 102, 102),  // 3: preview pointer (0.6,0.4,0.4, 0.4)
                Rgba8::new(204, 0, 0, 153),      // 4: pointer (0.8, 0, 0, 0.6)
                Rgba8::new(0, 0, 0, 255),       // 5: step markers (black)
            ],
            vertices: Vec::new(),
            vertex_idx: 0,
        };
        s.calc_box();
        s
    }

    fn calc_box(&mut self) {
        self.xs1 = self.x1 + self.border_width;
        self.ys1 = self.y1 + self.border_width;
        self.xs2 = self.x2 - self.border_width;
        self.ys2 = self.y2 - self.border_width;
    }

    /// Set the label format string. Use `%3.2f` as placeholder for the value.
    pub fn label(&mut self, fmt: &str) {
        self.label = fmt.to_string();
    }

    /// Set the value range.
    pub fn range(&mut self, min: f64, max: f64) {
        self.min = min;
        self.max = max;
    }

    /// Get the current value (in user range).
    pub fn value(&self) -> f64 {
        self.value * (self.max - self.min) + self.min
    }

    /// Set the current value (in user range).
    pub fn set_value(&mut self, v: f64) {
        self.preview_value = ((v - self.min) / (self.max - self.min)).clamp(0.0, 1.0);
        self.normalize_value(true);
    }

    /// Set the number of discrete steps (0 = continuous).
    pub fn num_steps(&mut self, n: u32) {
        self.num_steps = n;
    }

    pub fn set_descending(&mut self, d: bool) {
        self.descending = d;
    }

    pub fn border_width(&mut self, t: f64, extra: f64) {
        self.border_width = t;
        self.border_extra = extra;
        self.calc_box();
    }

    pub fn text_thickness(&mut self, t: f64) {
        self.text_thickness = t;
    }

    fn normalize_value(&mut self, preview_value_flag: bool) {
        if self.num_steps > 0 {
            let step = (self.preview_value * self.num_steps as f64 + 0.5) as i32;
            self.value = step as f64 / self.num_steps as f64;
        } else {
            self.value = self.preview_value;
        }
        if preview_value_flag {
            self.preview_value = self.value;
        }
    }

    /// Build vertices for the background rectangle (path 0).
    fn calc_background(&mut self) {
        let (x1, y1, x2, y2) = (self.x1, self.y1, self.x2, self.y2);
        let be = self.border_extra;
        self.vertices.clear();
        self.vertices.push((x1 - be, y1 - be, PATH_CMD_MOVE_TO));
        self.vertices.push((x2 + be, y1 - be, PATH_CMD_LINE_TO));
        self.vertices.push((x2 + be, y2 + be, PATH_CMD_LINE_TO));
        self.vertices.push((x1 - be, y2 + be, PATH_CMD_LINE_TO));
    }

    /// Build vertices for the triangle indicator (path 1).
    fn calc_triangle(&mut self) {
        self.vertices.clear();
        if self.descending {
            self.vertices.push((self.x1, self.y1, PATH_CMD_MOVE_TO));
            self.vertices.push((self.x2, self.y1, PATH_CMD_LINE_TO));
            self.vertices.push((self.x1, self.y2, PATH_CMD_LINE_TO));
            self.vertices.push((self.x1, self.y1, PATH_CMD_LINE_TO));
        } else {
            self.vertices.push((self.x1, self.y1, PATH_CMD_MOVE_TO));
            self.vertices.push((self.x2, self.y1, PATH_CMD_LINE_TO));
            self.vertices.push((self.x2, self.y2, PATH_CMD_LINE_TO));
            self.vertices.push((self.x1, self.y1, PATH_CMD_LINE_TO));
        }
    }

    /// Build vertices for the label text (path 2).
    fn calc_text(&mut self) {
        self.vertices.clear();

        let text = if self.label.contains('%') {
            // Format the label with the value (matching C++ sprintf behavior)
            self.label.replace("%3.2f", &format!("{:.2}", self.value()))
                      .replace("%.3f", &format!("{:.3}", self.value()))
                      .replace("%5.3f", &format!("{:.3}", self.value()))
                      .replace("%5.4f", &format!("{:.4}", self.value()))
                      .replace("%.2f", &format!("{:.2}", self.value()))
                      .replace("%d", &format!("{}", self.value() as i32))
        } else if self.label.is_empty() {
            return;
        } else {
            self.label.clone()
        };

        let text_height = self.y2 - self.y1;
        let mut txt = GsvText::new();
        txt.start_point(self.x1, self.y1);
        txt.size(text_height * 1.2, text_height);
        txt.text(&text);

        let mut stroke = ConvStroke::new(txt);
        stroke.set_width(self.text_thickness);
        stroke.set_line_join(LineJoin::Round);
        stroke.set_line_cap(LineCap::Round);

        stroke.rewind(0);
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = stroke.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            self.vertices.push((x, y, cmd));
        }
    }

    /// Build vertices for the pointer preview circle (path 3).
    fn calc_pointer_preview(&mut self) {
        self.vertices.clear();
        let cx = self.xs1 + (self.xs2 - self.xs1) * self.preview_value;
        let cy = (self.ys1 + self.ys2) / 2.0;
        let r = self.y2 - self.y1;

        let mut ell = Ellipse::new(cx, cy, r, r, 32, false);
        ell.rewind(0);
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = ell.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            self.vertices.push((x, y, cmd));
        }
    }

    /// Build vertices for the active pointer circle (path 4).
    fn calc_pointer(&mut self) {
        self.normalize_value(false);
        self.vertices.clear();
        let cx = self.xs1 + (self.xs2 - self.xs1) * self.value;
        let cy = (self.ys1 + self.ys2) / 2.0;
        let r = self.y2 - self.y1;

        let mut ell = Ellipse::new(cx, cy, r, r, 32, false);
        ell.rewind(0);
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = ell.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            self.vertices.push((x, y, cmd));
        }
    }

    /// Build vertices for step markers (path 5).
    fn calc_steps(&mut self) {
        self.vertices.clear();
        if self.num_steps == 0 {
            return;
        }
        let mut d = (self.xs2 - self.xs1) / self.num_steps as f64;
        if d > 0.004 {
            d = 0.004;
        }
        for i in 0..=self.num_steps {
            let x = self.xs1 + (self.xs2 - self.xs1) * i as f64 / self.num_steps as f64;
            self.vertices.push((x, self.y1, PATH_CMD_MOVE_TO));
            self.vertices
                .push((x - d * (self.x2 - self.x1), self.y1 - self.border_extra, PATH_CMD_LINE_TO));
            self.vertices
                .push((x + d * (self.x2 - self.x1), self.y1 - self.border_extra, PATH_CMD_LINE_TO));
        }
    }
}

impl Ctrl for SliderCtrl {
    fn num_paths(&self) -> u32 {
        6
    }

    fn color(&self, path_id: u32) -> Rgba8 {
        self.colors[path_id.min(5) as usize]
    }
}

impl VertexSource for SliderCtrl {
    fn rewind(&mut self, path_id: u32) {
        self.vertex_idx = 0;
        match path_id {
            0 => self.calc_background(),
            1 => self.calc_triangle(),
            2 => self.calc_text(),
            3 => self.calc_pointer_preview(),
            4 => self.calc_pointer(),
            5 => self.calc_steps(),
            _ => {
                self.vertices.clear();
            }
        }
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        if self.vertex_idx < self.vertices.len() {
            let (vx, vy, cmd) = self.vertices[self.vertex_idx];
            *x = vx;
            *y = vy;
            self.vertex_idx += 1;
            cmd
        } else {
            PATH_CMD_STOP
        }
    }
}

// ============================================================================
// CboxCtrl — checkbox
// ============================================================================

/// Checkbox control rendered via AGG.
///
/// Renders 3 paths: box border (hollow), label text, checkmark (when active).
///
/// Port of C++ `cbox_ctrl_impl`.
pub struct CboxCtrl {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    text_thickness: f64,
    text_height: f64,
    text_width: f64,
    label: String,
    status: bool,
    colors: [Rgba8; 3],
    vertices: Vec<(f64, f64, u32)>,
    vertex_idx: usize,
}

impl CboxCtrl {
    pub fn new(x: f64, y: f64, label: &str) -> Self {
        let text_height = 9.0;
        Self {
            x1: x,
            y1: y,
            x2: x + text_height * 1.5,
            y2: y + text_height * 1.5,
            text_thickness: 1.5,
            text_height,
            text_width: 0.0,
            label: label.to_string(),
            status: false,
            colors: [
                Rgba8::new(0, 0, 0, 255),     // 0: border (black)
                Rgba8::new(0, 0, 0, 255),     // 1: text (black)
                Rgba8::new(102, 0, 0, 255),   // 2: checkmark (dark red, 0.4,0,0)
            ],
            vertices: Vec::new(),
            vertex_idx: 0,
        }
    }

    pub fn set_status(&mut self, s: bool) {
        self.status = s;
    }

    pub fn status(&self) -> bool {
        self.status
    }

    pub fn text_size(&mut self, h: f64, w: f64) {
        self.text_height = h;
        self.text_width = w;
    }

    /// Build border vertices (path 0): outer rect + inner rect (hollow).
    fn calc_border(&mut self) {
        self.vertices.clear();
        let t = self.text_thickness;
        // Outer rectangle
        self.vertices.push((self.x1, self.y1, PATH_CMD_MOVE_TO));
        self.vertices.push((self.x2, self.y1, PATH_CMD_LINE_TO));
        self.vertices.push((self.x2, self.y2, PATH_CMD_LINE_TO));
        self.vertices.push((self.x1, self.y2, PATH_CMD_LINE_TO));
        // Inner rectangle (winding creates hollow)
        self.vertices.push((self.x1 + t, self.y1 + t, PATH_CMD_MOVE_TO));
        self.vertices.push((self.x1 + t, self.y2 - t, PATH_CMD_LINE_TO));
        self.vertices.push((self.x2 - t, self.y2 - t, PATH_CMD_LINE_TO));
        self.vertices.push((self.x2 - t, self.y1 + t, PATH_CMD_LINE_TO));
    }

    /// Build text vertices (path 1).
    fn calc_text(&mut self) {
        self.vertices.clear();
        let mut txt = GsvText::new();
        txt.start_point(
            self.x1 + self.text_height * 2.0,
            self.y1 + self.text_height / 5.0,
        );
        txt.size(self.text_height, self.text_width);
        txt.text(&self.label);

        let mut stroke = ConvStroke::new(txt);
        stroke.set_width(self.text_thickness);
        stroke.set_line_join(LineJoin::Round);
        stroke.set_line_cap(LineCap::Round);

        stroke.rewind(0);
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = stroke.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            self.vertices.push((x, y, cmd));
        }
    }

    /// Build checkmark vertices (path 2): octagon star, only when active.
    fn calc_checkmark(&mut self) {
        self.vertices.clear();
        if !self.status {
            return;
        }
        let d2 = (self.y2 - self.y1) / 2.0;
        let t = self.text_thickness * 1.5;
        // 8-vertex star pattern matching C++ cbox_ctrl exactly
        self.vertices.push((self.x1 + self.text_thickness, self.y1 + self.text_thickness, PATH_CMD_MOVE_TO));
        self.vertices.push((self.x1 + d2, self.y1 + d2 - t, PATH_CMD_LINE_TO));
        self.vertices.push((self.x2 - self.text_thickness, self.y1 + self.text_thickness, PATH_CMD_LINE_TO));
        self.vertices.push((self.x1 + d2 + t, self.y1 + d2, PATH_CMD_LINE_TO));
        self.vertices.push((self.x2 - self.text_thickness, self.y2 - self.text_thickness, PATH_CMD_LINE_TO));
        self.vertices.push((self.x1 + d2, self.y1 + d2 + t, PATH_CMD_LINE_TO));
        self.vertices.push((self.x1 + self.text_thickness, self.y2 - self.text_thickness, PATH_CMD_LINE_TO));
        self.vertices.push((self.x1 + d2 - t, self.y1 + d2, PATH_CMD_LINE_TO));
    }
}

impl Ctrl for CboxCtrl {
    fn num_paths(&self) -> u32 {
        3
    }

    fn color(&self, path_id: u32) -> Rgba8 {
        self.colors[path_id.min(2) as usize]
    }
}

impl VertexSource for CboxCtrl {
    fn rewind(&mut self, path_id: u32) {
        self.vertex_idx = 0;
        match path_id {
            0 => self.calc_border(),
            1 => self.calc_text(),
            2 => self.calc_checkmark(),
            _ => {
                self.vertices.clear();
            }
        }
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        if self.vertex_idx < self.vertices.len() {
            let (vx, vy, cmd) = self.vertices[self.vertex_idx];
            *x = vx;
            *y = vy;
            self.vertex_idx += 1;
            cmd
        } else {
            PATH_CMD_STOP
        }
    }
}

// ============================================================================
// RboxCtrl — radio button box
// ============================================================================

/// Radio button box control rendered via AGG.
///
/// Renders 5 paths: background, border (hollow), item text labels,
/// inactive radio circles (stroked), active radio circle (filled).
///
/// Port of C++ `rbox_ctrl_impl`.
pub struct RboxCtrl {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    xs1: f64,
    ys1: f64,
    xs2: f64,
    ys2: f64,
    border_width: f64,
    border_extra: f64,
    text_thickness: f64,
    text_height: f64,
    text_width: f64,
    items: Vec<String>,
    cur_item: i32,
    dy: f64,
    colors: [Rgba8; 5],
    vertices: Vec<(f64, f64, u32)>,
    vertex_idx: usize,
}

impl RboxCtrl {
    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        let mut r = Self {
            x1,
            y1,
            x2,
            y2,
            xs1: 0.0,
            ys1: 0.0,
            xs2: 0.0,
            ys2: 0.0,
            border_width: 1.0,
            border_extra: 0.0,
            text_thickness: 1.5,
            text_height: 9.0,
            text_width: 0.0,
            items: Vec::new(),
            cur_item: -1,
            dy: 18.0,
            colors: [
                Rgba8::new(255, 255, 230, 255), // 0: background (1.0, 1.0, 0.9)
                Rgba8::new(0, 0, 0, 255),       // 1: border (black)
                Rgba8::new(0, 0, 0, 255),       // 2: text (black)
                Rgba8::new(0, 0, 0, 255),       // 3: inactive circles (black)
                Rgba8::new(102, 0, 0, 255),     // 4: active circle (dark red, 0.4,0,0)
            ],
            vertices: Vec::new(),
            vertex_idx: 0,
        };
        r.calc_rbox();
        r
    }

    fn calc_rbox(&mut self) {
        self.xs1 = self.x1 + self.border_width;
        self.ys1 = self.y1 + self.border_width;
        self.xs2 = self.x2 - self.border_width;
        self.ys2 = self.y2 - self.border_width;
    }

    pub fn add_item(&mut self, text: &str) {
        self.items.push(text.to_string());
    }

    pub fn cur_item(&self) -> i32 {
        self.cur_item
    }

    pub fn set_cur_item(&mut self, i: i32) {
        self.cur_item = i;
    }

    pub fn text_size(&mut self, h: f64, w: f64) {
        self.text_height = h;
        self.text_width = w;
    }

    pub fn border_width(&mut self, t: f64, extra: f64) {
        self.border_width = t;
        self.border_extra = extra;
        self.calc_rbox();
    }

    pub fn text_thickness(&mut self, t: f64) {
        self.text_thickness = t;
    }

    pub fn background_color(&mut self, c: Rgba8) {
        self.colors[0] = c;
    }

    /// Build background rectangle (path 0).
    fn calc_background(&mut self) {
        self.vertices.clear();
        let be = self.border_extra;
        self.vertices.push((self.x1 - be, self.y1 - be, PATH_CMD_MOVE_TO));
        self.vertices.push((self.x2 + be, self.y1 - be, PATH_CMD_LINE_TO));
        self.vertices.push((self.x2 + be, self.y2 + be, PATH_CMD_LINE_TO));
        self.vertices.push((self.x1 - be, self.y2 + be, PATH_CMD_LINE_TO));
    }

    /// Build border (path 1): outer rect + inner rect (hollow).
    fn calc_border(&mut self) {
        self.vertices.clear();
        let bw = self.border_width;
        // Outer
        self.vertices.push((self.x1, self.y1, PATH_CMD_MOVE_TO));
        self.vertices.push((self.x2, self.y1, PATH_CMD_LINE_TO));
        self.vertices.push((self.x2, self.y2, PATH_CMD_LINE_TO));
        self.vertices.push((self.x1, self.y2, PATH_CMD_LINE_TO));
        // Inner
        self.vertices.push((self.x1 + bw, self.y1 + bw, PATH_CMD_MOVE_TO));
        self.vertices.push((self.x1 + bw, self.y2 - bw, PATH_CMD_LINE_TO));
        self.vertices.push((self.x2 - bw, self.y2 - bw, PATH_CMD_LINE_TO));
        self.vertices.push((self.x2 - bw, self.y1 + bw, PATH_CMD_LINE_TO));
    }

    /// Build all item text labels (path 2).
    fn calc_text(&mut self) {
        self.vertices.clear();
        let dy = self.text_height * 2.0;

        for (i, item) in self.items.iter().enumerate() {
            let mut txt = GsvText::new();
            txt.start_point(self.xs1 + dy * 1.5, self.ys1 + dy * (i as f64 + 1.0) - dy / 2.0);
            txt.size(self.text_height, self.text_width);
            txt.text(item);

            let mut stroke = ConvStroke::new(txt);
            stroke.set_width(self.text_thickness);
            stroke.set_line_join(LineJoin::Round);
            stroke.set_line_cap(LineCap::Round);

            stroke.rewind(0);
            loop {
                let (mut x, mut y) = (0.0, 0.0);
                let cmd = stroke.vertex(&mut x, &mut y);
                if is_stop(cmd) {
                    break;
                }
                self.vertices.push((x, y, cmd));
            }
        }
    }

    /// Build inactive radio circles (path 3): stroked circle outlines.
    fn calc_inactive_circles(&mut self) {
        self.vertices.clear();
        let dy = self.text_height * 2.0;
        let r = self.text_height / 1.5;

        for i in 0..self.items.len() {
            let cx = self.xs1 + dy / 1.3;
            let cy = self.ys1 + dy * i as f64 + dy / 1.3;

            let mut ell = Ellipse::new(cx, cy, r, r, 32, false);
            let mut stroke = ConvStroke::new(&mut ell);
            stroke.set_width(self.text_thickness);

            stroke.rewind(0);
            loop {
                let (mut x, mut y) = (0.0, 0.0);
                let cmd = stroke.vertex(&mut x, &mut y);
                if is_stop(cmd) {
                    break;
                }
                self.vertices.push((x, y, cmd));
            }
        }
    }

    /// Build the active radio circle (path 4): filled smaller circle.
    fn calc_active_circle(&mut self) {
        self.vertices.clear();
        if self.cur_item < 0 {
            return;
        }
        let dy = self.text_height * 2.0;
        let cx = self.xs1 + dy / 1.3;
        let cy = self.ys1 + dy * self.cur_item as f64 + dy / 1.3;
        let r = self.text_height / 2.0;

        let mut ell = Ellipse::new(cx, cy, r, r, 32, false);
        ell.rewind(0);
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = ell.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            self.vertices.push((x, y, cmd));
        }
    }
}

impl Ctrl for RboxCtrl {
    fn num_paths(&self) -> u32 {
        5
    }

    fn color(&self, path_id: u32) -> Rgba8 {
        self.colors[path_id.min(4) as usize]
    }
}

impl VertexSource for RboxCtrl {
    fn rewind(&mut self, path_id: u32) {
        self.vertex_idx = 0;
        self.dy = self.text_height * 2.0;
        match path_id {
            0 => self.calc_background(),
            1 => self.calc_border(),
            2 => self.calc_text(),
            3 => self.calc_inactive_circles(),
            4 => self.calc_active_circle(),
            _ => {
                self.vertices.clear();
            }
        }
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        if self.vertex_idx < self.vertices.len() {
            let (vx, vy, cmd) = self.vertices[self.vertex_idx];
            *x = vx;
            *y = vy;
            self.vertex_idx += 1;
            cmd
        } else {
            PATH_CMD_STOP
        }
    }
}

// ============================================================================
// GammaCtrl — interactive gamma spline control
// ============================================================================

/// Interactive gamma correction curve control.
///
/// Renders 7 paths: background, border, gamma curve, grid/crosshairs,
/// inactive point, active point, and text display.
///
/// Port of C++ `gamma_ctrl_impl` + `gamma_ctrl<ColorT>`.
pub struct GammaCtrl {
    // Widget bounds
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    // Gamma spline
    gamma_spline: crate::gamma::GammaSpline,
    // Visual parameters
    border_width: f64,
    border_extra: f64,
    curve_width: f64,
    grid_width: f64,
    text_thickness: f64,
    point_size: f64,
    text_height: f64,
    text_width: f64,
    // Chart area
    xc1: f64,
    yc1: f64,
    xc2: f64,
    yc2: f64,
    // Spline drawing area
    xs1: f64,
    ys1: f64,
    xs2: f64,
    ys2: f64,
    // Text area
    xt1: f64,
    yt1: f64,
    #[allow(dead_code)]
    xt2: f64,
    #[allow(dead_code)]
    yt2: f64,
    // Control point positions
    xp1: f64,
    yp1: f64,
    xp2: f64,
    yp2: f64,
    // Interaction state
    p1_active: bool,
    mouse_point: u32,
    pdx: f64,
    pdy: f64,
    // Colors (7 paths)
    colors: [Rgba8; 7],
    // Rendering state
    vertices: Vec<(f64, f64, u32)>,
    vertex_idx: usize,
}

impl GammaCtrl {
    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        let text_height = 9.0;
        let border_width = 2.0;
        let yc2 = y2 - text_height * 2.0;

        let mut gc = Self {
            x1,
            y1,
            x2,
            y2,
            gamma_spline: crate::gamma::GammaSpline::new(),
            border_width,
            border_extra: 0.0,
            curve_width: 2.0,
            grid_width: 0.2,
            text_thickness: 1.5,
            point_size: 5.0,
            text_height,
            text_width: 0.0,
            xc1: x1,
            yc1: y1,
            xc2: x2,
            yc2,
            xs1: 0.0,
            ys1: 0.0,
            xs2: 0.0,
            ys2: 0.0,
            xt1: x1,
            yt1: yc2,
            xt2: x2,
            yt2: y2,
            xp1: 0.0,
            yp1: 0.0,
            xp2: 0.0,
            yp2: 0.0,
            p1_active: true,
            mouse_point: 0,
            pdx: 0.0,
            pdy: 0.0,
            colors: [
                Rgba8::new(255, 255, 230, 255), // 0: background (1.0, 1.0, 0.9)
                Rgba8::new(0, 0, 0, 255),       // 1: border (black)
                Rgba8::new(0, 0, 0, 255),       // 2: curve (black)
                Rgba8::new(51, 51, 0, 255),     // 3: grid (0.2, 0.2, 0.0)
                Rgba8::new(0, 0, 0, 255),       // 4: inactive point (black)
                Rgba8::new(255, 0, 0, 255),     // 5: active point (red)
                Rgba8::new(0, 0, 0, 255),       // 6: text (black)
            ],
            vertices: Vec::new(),
            vertex_idx: 0,
        };
        gc.calc_spline_box();
        gc
    }

    fn calc_spline_box(&mut self) {
        self.xs1 = self.xc1 + self.border_width;
        self.ys1 = self.yc1 + self.border_width;
        self.xs2 = self.xc2 - self.border_width;
        self.ys2 = self.yc2 - self.border_width * 0.5;
    }

    fn calc_points(&mut self) {
        let (kx1, ky1, kx2, ky2) = self.gamma_spline.get_values();
        self.xp1 = self.xs1 + (self.xs2 - self.xs1) * kx1 * 0.25;
        self.yp1 = self.ys1 + (self.ys2 - self.ys1) * ky1 * 0.25;
        self.xp2 = self.xs2 - (self.xs2 - self.xs1) * kx2 * 0.25;
        self.yp2 = self.ys2 - (self.ys2 - self.ys1) * ky2 * 0.25;
    }

    fn calc_values(&mut self) {
        let kx1 = (self.xp1 - self.xs1) * 4.0 / (self.xs2 - self.xs1);
        let ky1 = (self.yp1 - self.ys1) * 4.0 / (self.ys2 - self.ys1);
        let kx2 = (self.xs2 - self.xp2) * 4.0 / (self.xs2 - self.xs1);
        let ky2 = (self.ys2 - self.yp2) * 4.0 / (self.ys2 - self.ys1);
        self.gamma_spline.set_values(kx1, ky1, kx2, ky2);
    }

    // --- Configuration ---

    pub fn border_width(&mut self, t: f64, extra: f64) {
        self.border_width = t;
        self.border_extra = extra;
        self.calc_spline_box();
    }

    pub fn curve_width(&mut self, t: f64) {
        self.curve_width = t;
    }

    pub fn grid_width(&mut self, t: f64) {
        self.grid_width = t;
    }

    pub fn text_thickness(&mut self, t: f64) {
        self.text_thickness = t;
    }

    pub fn text_size(&mut self, h: f64, w: f64) {
        self.text_width = w;
        self.text_height = h;
        self.yc2 = self.y2 - self.text_height * 2.0;
        self.yt1 = self.y2 - self.text_height * 2.0;
        self.calc_spline_box();
    }

    pub fn point_size(&mut self, s: f64) {
        self.point_size = s;
    }

    // --- Spline value access ---

    pub fn set_values(&mut self, kx1: f64, ky1: f64, kx2: f64, ky2: f64) {
        self.gamma_spline.set_values(kx1, ky1, kx2, ky2);
    }

    pub fn get_values(&self) -> (f64, f64, f64, f64) {
        self.gamma_spline.get_values()
    }

    pub fn gamma(&self) -> &[u8; 256] {
        self.gamma_spline.gamma()
    }

    pub fn y(&self, x: f64) -> f64 {
        self.gamma_spline.y(x)
    }

    pub fn get_gamma_spline(&self) -> &crate::gamma::GammaSpline {
        &self.gamma_spline
    }

    pub fn change_active_point(&mut self) {
        self.p1_active = !self.p1_active;
    }

    // --- Color setters ---

    pub fn background_color(&mut self, c: Rgba8) {
        self.colors[0] = c;
    }
    pub fn border_color(&mut self, c: Rgba8) {
        self.colors[1] = c;
    }
    pub fn curve_color(&mut self, c: Rgba8) {
        self.colors[2] = c;
    }
    pub fn grid_color(&mut self, c: Rgba8) {
        self.colors[3] = c;
    }
    pub fn inactive_pnt_color(&mut self, c: Rgba8) {
        self.colors[4] = c;
    }
    pub fn active_pnt_color(&mut self, c: Rgba8) {
        self.colors[5] = c;
    }
    pub fn text_color(&mut self, c: Rgba8) {
        self.colors[6] = c;
    }

    // --- Mouse interaction ---

    pub fn in_rect(&self, x: f64, y: f64) -> bool {
        x >= self.x1 && x <= self.x2 && y >= self.y1 && y <= self.y2
    }

    pub fn on_mouse_button_down(&mut self, x: f64, y: f64) -> bool {
        self.calc_points();
        let dist1 = ((x - self.xp1).powi(2) + (y - self.yp1).powi(2)).sqrt();
        if dist1 <= self.point_size + 1.0 {
            self.mouse_point = 1;
            self.pdx = self.xp1 - x;
            self.pdy = self.yp1 - y;
            self.p1_active = true;
            return true;
        }
        let dist2 = ((x - self.xp2).powi(2) + (y - self.yp2).powi(2)).sqrt();
        if dist2 <= self.point_size + 1.0 {
            self.mouse_point = 2;
            self.pdx = self.xp2 - x;
            self.pdy = self.yp2 - y;
            self.p1_active = false;
            return true;
        }
        false
    }

    pub fn on_mouse_button_up(&mut self, _x: f64, _y: f64) -> bool {
        if self.mouse_point != 0 {
            self.mouse_point = 0;
            return true;
        }
        false
    }

    pub fn on_mouse_move(&mut self, x: f64, y: f64, button_flag: bool) -> bool {
        if !button_flag {
            return self.on_mouse_button_up(x, y);
        }
        if self.mouse_point == 1 {
            self.xp1 = x + self.pdx;
            self.yp1 = y + self.pdy;
            self.calc_values();
            return true;
        }
        if self.mouse_point == 2 {
            self.xp2 = x + self.pdx;
            self.yp2 = y + self.pdy;
            self.calc_values();
            return true;
        }
        false
    }

    pub fn on_arrow_keys(&mut self, left: bool, right: bool, down: bool, up: bool) -> bool {
        let (mut kx1, mut ky1, mut kx2, mut ky2) = self.gamma_spline.get_values();
        let mut ret = false;
        if self.p1_active {
            if left {
                kx1 -= 0.005;
                ret = true;
            }
            if right {
                kx1 += 0.005;
                ret = true;
            }
            if down {
                ky1 -= 0.005;
                ret = true;
            }
            if up {
                ky1 += 0.005;
                ret = true;
            }
        } else {
            if left {
                kx2 += 0.005;
                ret = true;
            }
            if right {
                kx2 -= 0.005;
                ret = true;
            }
            if down {
                ky2 += 0.005;
                ret = true;
            }
            if up {
                ky2 -= 0.005;
                ret = true;
            }
        }
        if ret {
            self.gamma_spline.set_values(kx1, ky1, kx2, ky2);
        }
        ret
    }

    // --- Path generation helpers ---

    /// Path 0: Background rectangle.
    fn calc_background(&mut self) {
        self.vertices.clear();
        let be = self.border_extra;
        self.vertices
            .push((self.x1 - be, self.y1 - be, PATH_CMD_MOVE_TO));
        self.vertices
            .push((self.x2 + be, self.y1 - be, PATH_CMD_LINE_TO));
        self.vertices
            .push((self.x2 + be, self.y2 + be, PATH_CMD_LINE_TO));
        self.vertices
            .push((self.x1 - be, self.y2 + be, PATH_CMD_LINE_TO));
    }

    /// Path 1: Border (3 contours: outer frame, inner hollow, separator line).
    fn calc_border(&mut self) {
        self.vertices.clear();
        let bw = self.border_width;
        // Outer rectangle
        self.vertices
            .push((self.x1, self.y1, PATH_CMD_MOVE_TO));
        self.vertices
            .push((self.x2, self.y1, PATH_CMD_LINE_TO));
        self.vertices
            .push((self.x2, self.y2, PATH_CMD_LINE_TO));
        self.vertices
            .push((self.x1, self.y2, PATH_CMD_LINE_TO));
        // Inner hollow
        self.vertices
            .push((self.x1 + bw, self.y1 + bw, PATH_CMD_MOVE_TO));
        self.vertices
            .push((self.x1 + bw, self.y2 - bw, PATH_CMD_LINE_TO));
        self.vertices
            .push((self.x2 - bw, self.y2 - bw, PATH_CMD_LINE_TO));
        self.vertices
            .push((self.x2 - bw, self.y1 + bw, PATH_CMD_LINE_TO));
        // Separator line between chart and text
        self.vertices
            .push((self.xc1 + bw, self.yc2 - bw * 0.5, PATH_CMD_MOVE_TO));
        self.vertices
            .push((self.xc2 - bw, self.yc2 - bw * 0.5, PATH_CMD_LINE_TO));
        self.vertices
            .push((self.xc2 - bw, self.yc2 + bw * 0.5, PATH_CMD_LINE_TO));
        self.vertices
            .push((self.xc1 + bw, self.yc2 + bw * 0.5, PATH_CMD_LINE_TO));
    }

    /// Path 2: Gamma curve (stroked spline).
    fn calc_curve(&mut self) {
        self.vertices.clear();
        self.gamma_spline
            .set_box(self.xs1, self.ys1, self.xs2, self.ys2);

        let mut spline_copy = crate::gamma::GammaSpline::new();
        // Re-use the same values
        let (kx1, ky1, kx2, ky2) = self.gamma_spline.get_values();
        spline_copy.set_values(kx1, ky1, kx2, ky2);
        spline_copy.set_box(self.xs1, self.ys1, self.xs2, self.ys2);

        // Generate spline vertices, then stroke
        let mut path_verts = Vec::new();
        spline_copy.rewind(0);
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = spline_copy.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            path_verts.push((x, y, cmd));
        }

        // Build a custom vertex source from the captured vertices
        let mut path = crate::path_storage::PathStorage::new();
        for (i, &(x, y, cmd)) in path_verts.iter().enumerate() {
            if i == 0 {
                path.move_to(x, y);
            } else {
                path.line_to(x, y);
            }
            let _ = cmd;
        }

        let mut stroke = ConvStroke::new(&mut path);
        stroke.set_width(self.curve_width);
        stroke.rewind(0);
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = stroke.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            self.vertices.push((x, y, cmd));
        }
    }

    /// Path 3: Grid (center lines + crosshairs at control points).
    fn calc_grid(&mut self) {
        self.vertices.clear();
        self.calc_points();

        let gw = self.grid_width;
        let (xs1, ys1, xs2, ys2) = (self.xs1, self.ys1, self.xs2, self.ys2);
        let ymid = (ys1 + ys2) * 0.5;
        let xmid = (xs1 + xs2) * 0.5;

        // Horizontal center line
        self.vertices.push((xs1, ymid - gw * 0.5, PATH_CMD_MOVE_TO));
        self.vertices.push((xs2, ymid - gw * 0.5, PATH_CMD_LINE_TO));
        self.vertices.push((xs2, ymid + gw * 0.5, PATH_CMD_LINE_TO));
        self.vertices.push((xs1, ymid + gw * 0.5, PATH_CMD_LINE_TO));

        // Vertical center line
        self.vertices.push((xmid - gw * 0.5, ys1, PATH_CMD_MOVE_TO));
        self.vertices
            .push((xmid - gw * 0.5, ys2, PATH_CMD_LINE_TO));
        self.vertices
            .push((xmid + gw * 0.5, ys2, PATH_CMD_LINE_TO));
        self.vertices
            .push((xmid + gw * 0.5, ys1, PATH_CMD_LINE_TO));

        // Crosshair at point 1
        let (xp1, yp1) = (self.xp1, self.yp1);
        self.vertices
            .push((xs1, yp1 - gw * 0.5, PATH_CMD_MOVE_TO));
        self.vertices
            .push((xp1 - gw * 0.5, yp1 - gw * 0.5, PATH_CMD_LINE_TO));
        self.vertices
            .push((xp1 - gw * 0.5, ys1, PATH_CMD_LINE_TO));
        self.vertices
            .push((xp1 + gw * 0.5, ys1, PATH_CMD_LINE_TO));
        self.vertices
            .push((xp1 + gw * 0.5, yp1 + gw * 0.5, PATH_CMD_LINE_TO));
        self.vertices
            .push((xs1, yp1 + gw * 0.5, PATH_CMD_LINE_TO));

        // Crosshair at point 2
        let (xp2, yp2) = (self.xp2, self.yp2);
        self.vertices
            .push((xs2, yp2 + gw * 0.5, PATH_CMD_MOVE_TO));
        self.vertices
            .push((xp2 + gw * 0.5, yp2 + gw * 0.5, PATH_CMD_LINE_TO));
        self.vertices
            .push((xp2 + gw * 0.5, ys2, PATH_CMD_LINE_TO));
        self.vertices
            .push((xp2 - gw * 0.5, ys2, PATH_CMD_LINE_TO));
        self.vertices
            .push((xp2 - gw * 0.5, yp2 - gw * 0.5, PATH_CMD_LINE_TO));
        self.vertices
            .push((xs2, yp2 - gw * 0.5, PATH_CMD_LINE_TO));
    }

    /// Path 4: Inactive control point (ellipse).
    fn calc_inactive_point(&mut self) {
        self.vertices.clear();
        self.calc_points();
        let (cx, cy) = if self.p1_active {
            (self.xp2, self.yp2)
        } else {
            (self.xp1, self.yp1)
        };
        let mut ell = Ellipse::new(cx, cy, self.point_size, self.point_size, 32, false);
        ell.rewind(0);
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = ell.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            self.vertices.push((x, y, cmd));
        }
    }

    /// Path 5: Active control point (ellipse).
    fn calc_active_point(&mut self) {
        self.vertices.clear();
        self.calc_points();
        let (cx, cy) = if self.p1_active {
            (self.xp1, self.yp1)
        } else {
            (self.xp2, self.yp2)
        };
        let mut ell = Ellipse::new(cx, cy, self.point_size, self.point_size, 32, false);
        ell.rewind(0);
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = ell.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            self.vertices.push((x, y, cmd));
        }
    }

    /// Path 6: Text display showing current kx1, ky1, kx2, ky2 values.
    fn calc_text_display(&mut self) {
        self.vertices.clear();
        let (kx1, ky1, kx2, ky2) = self.gamma_spline.get_values();
        let text = format!("{:.3} {:.3} {:.3} {:.3}", kx1, ky1, kx2, ky2);

        let mut txt = GsvText::new();
        txt.text(&text);
        txt.size(self.text_height, self.text_width);
        txt.start_point(
            self.xt1 + self.border_width * 2.0,
            (self.yt1 + self.y2) * 0.5 - self.text_height * 0.5,
        );

        let mut stroke = ConvStroke::new(txt);
        stroke.set_width(self.text_thickness);
        stroke.set_line_join(LineJoin::Round);
        stroke.set_line_cap(LineCap::Round);

        stroke.rewind(0);
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = stroke.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            self.vertices.push((x, y, cmd));
        }
    }
}

impl Ctrl for GammaCtrl {
    fn num_paths(&self) -> u32 {
        7
    }

    fn color(&self, path_id: u32) -> Rgba8 {
        self.colors[path_id.min(6) as usize]
    }
}

impl VertexSource for GammaCtrl {
    fn rewind(&mut self, path_id: u32) {
        self.vertex_idx = 0;
        match path_id {
            0 => self.calc_background(),
            1 => self.calc_border(),
            2 => self.calc_curve(),
            3 => self.calc_grid(),
            4 => self.calc_inactive_point(),
            5 => self.calc_active_point(),
            6 => self.calc_text_display(),
            _ => {
                self.vertices.clear();
            }
        }
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        if self.vertex_idx < self.vertices.len() {
            let (vx, vy, cmd) = self.vertices[self.vertex_idx];
            *x = vx;
            *y = vy;
            self.vertex_idx += 1;
            cmd
        } else {
            PATH_CMD_STOP
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slider_ctrl_default_value() {
        let s = SliderCtrl::new(5.0, 5.0, 300.0, 12.0);
        // Default normalized value is 0.5, range 0..1
        assert!((s.value() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_slider_ctrl_set_value() {
        let mut s = SliderCtrl::new(5.0, 5.0, 300.0, 12.0);
        s.range(-180.0, 180.0);
        s.set_value(90.0);
        assert!((s.value() - 90.0).abs() < 1e-6);
    }

    #[test]
    fn test_slider_ctrl_vertex_source() {
        let mut s = SliderCtrl::new(5.0, 5.0, 300.0, 12.0);
        // Path 0: background rectangle — should have 4 vertices
        s.rewind(0);
        let mut count = 0;
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = s.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            count += 1;
        }
        assert_eq!(count, 4);
    }

    #[test]
    fn test_slider_ctrl_pointer_circle() {
        let mut s = SliderCtrl::new(5.0, 5.0, 300.0, 12.0);
        // Path 4: pointer circle — should have ~33 vertices (32 segments + close)
        s.rewind(4);
        let mut count = 0;
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = s.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            count += 1;
        }
        assert!(count >= 32);
    }

    #[test]
    fn test_slider_ctrl_text() {
        let mut s = SliderCtrl::new(5.0, 5.0, 300.0, 12.0);
        s.label("Angle=%3.2f");
        s.set_value(45.0);
        // Path 2: text — should produce some vertices
        s.rewind(2);
        let mut count = 0;
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = s.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            count += 1;
        }
        assert!(count > 0);
    }

    #[test]
    fn test_slider_ctrl_num_paths() {
        let s = SliderCtrl::new(5.0, 5.0, 300.0, 12.0);
        assert_eq!(s.num_paths(), 6);
    }

    #[test]
    fn test_cbox_ctrl_basic() {
        let mut c = CboxCtrl::new(10.0, 10.0, "Outline");
        assert!(!c.status());
        c.set_status(true);
        assert!(c.status());
        assert_eq!(c.num_paths(), 3);
    }

    #[test]
    fn test_cbox_ctrl_checkmark_only_when_active() {
        let mut c = CboxCtrl::new(10.0, 10.0, "Test");
        // Path 2 (checkmark) should have 0 vertices when inactive
        c.set_status(false);
        c.rewind(2);
        let (mut x, mut y) = (0.0, 0.0);
        let cmd = c.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_STOP);

        // Should have vertices when active
        c.set_status(true);
        c.rewind(2);
        let cmd = c.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_MOVE_TO);
    }

    #[test]
    fn test_rbox_ctrl_basic() {
        let mut r = RboxCtrl::new(10.0, 10.0, 150.0, 100.0);
        r.add_item("Option A");
        r.add_item("Option B");
        r.add_item("Option C");
        r.set_cur_item(1);
        assert_eq!(r.cur_item(), 1);
        assert_eq!(r.num_paths(), 5);
    }

    #[test]
    fn test_rbox_ctrl_inactive_circles() {
        let mut r = RboxCtrl::new(10.0, 10.0, 150.0, 100.0);
        r.add_item("A");
        r.add_item("B");
        // Path 3: inactive circles — should produce vertices for 2 items
        r.rewind(3);
        let mut count = 0;
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = r.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            count += 1;
        }
        assert!(count > 32); // At least 2 circles worth
    }

    #[test]
    fn test_rbox_ctrl_no_active_when_negative() {
        let mut r = RboxCtrl::new(10.0, 10.0, 150.0, 100.0);
        r.add_item("X");
        // cur_item = -1 means no selection
        r.rewind(4);
        let (mut x, mut y) = (0.0, 0.0);
        let cmd = r.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_STOP);
    }

    // ====================================================================
    // GammaCtrl tests
    // ====================================================================

    #[test]
    fn test_gamma_ctrl_basic() {
        let gc = GammaCtrl::new(10.0, 10.0, 200.0, 200.0);
        assert_eq!(gc.num_paths(), 7);
    }

    #[test]
    fn test_gamma_ctrl_default_values() {
        let gc = GammaCtrl::new(10.0, 10.0, 200.0, 200.0);
        let (kx1, ky1, kx2, ky2) = gc.get_values();
        assert!((kx1 - 1.0).abs() < 0.01);
        assert!((ky1 - 1.0).abs() < 0.01);
        assert!((kx2 - 1.0).abs() < 0.01);
        assert!((ky2 - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_gamma_ctrl_set_values() {
        let mut gc = GammaCtrl::new(10.0, 10.0, 200.0, 200.0);
        gc.set_values(0.5, 1.5, 0.8, 1.2);
        let (kx1, ky1, kx2, ky2) = gc.get_values();
        assert!((kx1 - 0.5).abs() < 0.001);
        assert!((ky1 - 1.5).abs() < 0.001);
        assert!((kx2 - 0.8).abs() < 0.001);
        assert!((ky2 - 1.2).abs() < 0.001);
    }

    #[test]
    fn test_gamma_ctrl_gamma_table() {
        let gc = GammaCtrl::new(10.0, 10.0, 200.0, 200.0);
        let gamma = gc.gamma();
        assert_eq!(gamma[0], 0);
        assert_eq!(gamma[255], 255);
    }

    #[test]
    fn test_gamma_ctrl_background_path() {
        let mut gc = GammaCtrl::new(10.0, 10.0, 200.0, 200.0);
        gc.rewind(0);
        let mut count = 0;
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = gc.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            count += 1;
        }
        assert_eq!(count, 4); // Rectangle
    }

    #[test]
    fn test_gamma_ctrl_border_path() {
        let mut gc = GammaCtrl::new(10.0, 10.0, 200.0, 200.0);
        gc.rewind(1);
        let mut count = 0;
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = gc.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            count += 1;
        }
        assert_eq!(count, 12); // 3 contours * 4 vertices
    }

    #[test]
    fn test_gamma_ctrl_curve_path() {
        let mut gc = GammaCtrl::new(10.0, 10.0, 200.0, 200.0);
        gc.rewind(2);
        let mut count = 0;
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = gc.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            count += 1;
        }
        assert!(count > 10, "Curve path should have many vertices, got {count}");
    }

    #[test]
    fn test_gamma_ctrl_grid_path() {
        let mut gc = GammaCtrl::new(10.0, 10.0, 200.0, 200.0);
        gc.rewind(3);
        let mut count = 0;
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = gc.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            count += 1;
        }
        assert_eq!(count, 20); // 4 contours: 4+4+6+6
    }

    #[test]
    fn test_gamma_ctrl_points_path() {
        let mut gc = GammaCtrl::new(10.0, 10.0, 200.0, 200.0);
        // Path 4: inactive point (ellipse with 32 segments)
        gc.rewind(4);
        let mut count = 0;
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = gc.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            count += 1;
        }
        assert!(count >= 32);

        // Path 5: active point
        gc.rewind(5);
        let mut count = 0;
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = gc.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            count += 1;
        }
        assert!(count >= 32);
    }

    #[test]
    fn test_gamma_ctrl_text_path() {
        let mut gc = GammaCtrl::new(10.0, 10.0, 200.0, 200.0);
        gc.rewind(6);
        let mut count = 0;
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = gc.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            count += 1;
        }
        assert!(count > 0, "Text path should have vertices");
    }

    #[test]
    fn test_gamma_ctrl_mouse_interaction() {
        let mut gc = GammaCtrl::new(0.0, 0.0, 200.0, 200.0);
        // in_rect
        assert!(gc.in_rect(100.0, 100.0));
        assert!(!gc.in_rect(300.0, 300.0));

        // Arrow key adjustment
        let changed = gc.on_arrow_keys(false, true, false, false);
        assert!(changed);
        let (kx1, _, _, _) = gc.get_values();
        assert!((kx1 - 1.005).abs() < 0.001);
    }
}
