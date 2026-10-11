//! Port of agg-sharp `RendererMarkersTests.cs`: renderer_markers culls a marker only when its
//! own box misses the clip box (C++'s `visible` builds the right edge as `x+y`; the divergence
//! is documented at `RendererMarkers::visible`).

use crate::common::Image;
use agg_rust::color::Rgba8;
use agg_rust::pixfmt_rgba::PixfmtRgba32;
use agg_rust::renderer_base::RendererBase;
use agg_rust::renderer_markers::{MarkerType, RendererMarkers};

fn black() -> Rgba8 {
    Rgba8::new(0, 0, 0, 255)
}

/// A marker centered just left of the image still draws the part that reaches into it. C++
/// renderer_markers::visible builds the box's right edge as x+y, not x+r, so at y 0 it culled
/// this cross.
#[test]
fn marker_left_of_the_clip_box_draws_its_visible_part() {
    let mut image = Image::new(10, 10);
    {
        let mut ren = RendererBase::new(PixfmtRgba32::new(&mut image.ra));
        let mut markers = RendererMarkers::new(&mut ren);
        markers.set_line_color(black());
        markers.set_fill_color(black());

        markers.marker(-2, 0, 4, MarkerType::Cross);
    }

    assert_eq!(image.pixel(0, 0)[3], 255);
    assert_eq!(image.pixel(2, 0)[3], 255);
    assert_eq!(image.pixel(3, 0)[3], 0);
}

/// Divergence regression: `visible` is the marker's own box against the clip box, whatever `y`
/// is. With C++'s `x+y` the first marker (right edge -2 + 0 instead of -2 + 4) was culled.
#[test]
fn rust_only_visible_uses_the_marker_box_not_x_plus_y() {
    let mut image = Image::new(10, 10);
    let mut ren = RendererBase::new(PixfmtRgba32::new(&mut image.ra));
    let markers = RendererMarkers::new(&mut ren);

    assert!(markers.visible(-2, 0, 4));
    assert!(markers.visible(13, 5, 4));
    assert!(!markers.visible(14, 5, 4));
    assert!(!markers.visible(-5, 9, 4));
    assert!(!markers.visible(5, -5, 4));
}
