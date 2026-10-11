//! Port of agg-sharp `StackBlurTests.cs`: stack blur keeps a flat field exactly. The C++ tables'
//! multiply and shift fall just short of dividing by the (radius + 1)^2 weight, but never far
//! enough to drop a flat field by a level.

use crate::common::Image;
use agg_rust::blur::stack_blur_rgba32;

#[test]
fn flat_field_stays_exact() {
    for (radius, value) in [
        (1u32, 255u32),
        (4, 255),
        (15, 255),
        (40, 255),
        (254, 255),
        (15, 1),
        (15, 128),
        (40, 77),
        (200, 200),
        (254, 128),
    ] {
        let color = [value as u8, (255 - value) as u8, (value / 2) as u8, 255];
        let mut image = Image::new(60, 40);
        for px in image.buf.chunks_exact_mut(4) {
            px.copy_from_slice(&color);
        }

        stack_blur_rgba32(&mut image.ra, radius, radius);

        for y in 0..image.height {
            for x in 0..image.width {
                assert_eq!(
                    image.pixel(x, y),
                    color,
                    "radius {radius}, value {value} at {x},{y}"
                );
            }
        }
    }
}
