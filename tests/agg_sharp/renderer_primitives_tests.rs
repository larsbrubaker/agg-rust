//! Port of agg-sharp `RendererPrimitivesTests.cs`: renderer_primitives' shapes, traced by hand
//! through agg_renderer_primitives.h.

use crate::common::Image;
use agg_rust::color::Rgba8;
use agg_rust::pixfmt_rgba::PixfmtRgba32;
use agg_rust::renderer_base::RendererBase;
use agg_rust::renderer_primitives::RendererPrimitives;

const LINE: [u8; 4] = [255, 0, 0, 255];
const FILL: [u8; 4] = [0, 0, 255, 255];

fn draw(image: &mut Image, f: impl FnOnce(&mut RendererPrimitives<PixfmtRgba32>)) {
    let mut ren = RendererBase::new(PixfmtRgba32::new(&mut image.ra));
    let mut primitives = RendererPrimitives::new(&mut ren);
    primitives.set_line_color(Rgba8::new(255, 0, 0, 255));
    primitives.set_fill_color(Rgba8::new(0, 0, 255, 255));
    f(&mut primitives);
}

#[test]
fn outlined_rectangle_draws_its_border_in_the_line_color_and_its_inside_in_the_fill_color() {
    let mut image = Image::new(6, 6);
    draw(&mut image, |p| p.outlined_rectangle(1, 1, 4, 4));

    assert_eq!(image.pixel(1, 1), LINE);
    assert_eq!(image.pixel(4, 1), LINE);
    assert_eq!(image.pixel(4, 4), LINE);
    assert_eq!(image.pixel(1, 4), LINE);
    assert_eq!(image.pixel(2, 2), FILL);
    assert_eq!(image.pixel(3, 3), FILL);
    assert_eq!(image.pixel(0, 0)[3], 0);
    assert_eq!(image.pixel(5, 5)[3], 0);
}

/// A bar reaching past the image is cut to it, as renderer_base::blend_bar clips.
#[test]
fn solid_rectangle_is_clipped_to_the_image() {
    let mut image = Image::new(4, 4);
    draw(&mut image, |p| p.solid_rectangle(2, 2, -3, 9));

    assert_eq!(image.pixel(0, 3), FILL);
    assert_eq!(image.pixel(2, 2), FILL);
    assert_eq!(image.pixel(3, 2)[3], 0);
    assert_eq!(image.pixel(0, 1)[3], 0);
}

#[test]
fn solid_ellipse_fills_its_whole_radius() {
    let mut image = Image::new(11, 11);
    draw(&mut image, |p| p.solid_ellipse(5, 5, 3, 3));

    assert_eq!(image.pixel(2, 5), FILL);
    assert_eq!(image.pixel(8, 5), FILL);
    assert_eq!(image.pixel(5, 2), FILL);
    assert_eq!(image.pixel(5, 8), FILL);
    assert_eq!(image.pixel(1, 5)[3], 0);
    assert_eq!(image.pixel(5, 9)[3], 0);
    assert_eq!(image.pixel(2, 2)[3], 0);
}

#[test]
fn outlined_ellipse_rings_its_fill_in_the_line_color() {
    let mut image = Image::new(11, 11);
    draw(&mut image, |p| p.outlined_ellipse(5, 5, 3, 3));

    assert_eq!(image.pixel(2, 5), LINE);
    assert_eq!(image.pixel(8, 5), LINE);
    assert_eq!(image.pixel(5, 2), LINE);
    assert_eq!(image.pixel(5, 8), LINE);
    assert_eq!(image.pixel(5, 5), FILL);
    assert_eq!(image.pixel(1, 5)[3], 0);
}
