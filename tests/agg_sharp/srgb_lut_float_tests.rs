//! agg-sharp's `SrgbLutFloatTests` (`Tests/Agg.Tests/Agg/SrgbLutFloatTests.cs`): C++
//! `rgba32(srgba8)` and `srgba8(rgba32)` through `sRGB_lut<float>`, pinned bit for bit to a
//! C++ trace.

use agg_rust::color::Rgba8;
use agg_rust::color_rgba32::Rgba32;

/// C++ `rgba32(srgba8(r, g, b, a))` - the compositing demo's colors among them - pinned bit for
/// bit to a trace built against the reference renderer's headers.
#[test]
fn rgba32_from_srgba8_matches_cpp() {
    let cases: [(u32, u32, u32, u32, u32, u32, u32, u32); 7] = [
        (
            0xFD, 0xF0, 0x6F, 255, 0x3F7B74C6, 0x3F5F11EA, 0x3E22C6A1, 0x3F800000,
        ),
        (
            0xFE, 0x9F, 0x34, 127, 0x3F7DB8DE, 0x3EB18332, 0x3D0CA7E6, 0x3EFEFEFF,
        ),
        (
            0x7F, 0xC1, 0xFF, 191, 0x3E595305, 0x3F0884CD, 0x3F800000, 0x3F3FBFC0,
        ),
        (
            0x05, 0x00, 0x5F, 191, 0x3AC6EB61, 0x00000000, 0x3DEA5D18, 0x3F3FBFC0,
        ),
        (
            0xDF, 0xDF, 0xDF, 255, 0x3F3CE7B2, 0x3F3CE7B2, 0x3F3CE7B2, 0x3F800000,
        ),
        (
            0x00, 0x00, 0x00, 0, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
        ),
        (
            0x01, 0x0A, 0x0B, 1, 0x399F22B4, 0x3B46EB61, 0x3B5B518E, 0x3B808081,
        ),
    ];
    for (r, g, b, a, er, eg, eb, ea) in cases {
        let c = Rgba32::from_srgba8(&Rgba8::new(r, g, b, a));

        assert_eq!(c.r.to_bits(), er, "red of {r:#x}");
        assert_eq!(c.g.to_bits(), eg, "green of {g:#x}");
        assert_eq!(c.b.to_bits(), eb, "blue of {b:#x}");
        assert_eq!(c.a.to_bits(), ea, "alpha of {a}");
    }
}

/// C++ `srgba8(rgba32)` - how the compositing demo's float frame becomes 8-bit pixels - pinned to
/// the same trace: channels by the binary search over the float inverse table (clamped at both
/// ends), alpha `int8u(0.5 + a * 255)`.
#[test]
fn srgba8_from_rgba32_matches_cpp() {
    let cases: [(u32, u32, u32, u32, u8, u8, u8, u8); 6] = [
        (0x00000000, 0x00000000, 0x00000000, 0x00000000, 0, 0, 0, 0),
        (
            0x3F800000, 0x3F800000, 0x3F800000, 0x3F800000, 255, 255, 255, 255,
        ),
        (
            0x3F000000, 0x3E800000, 0x3F400000, 0x3F000000, 188, 137, 225, 128,
        ),
        (
            0x3A83126F, 0x3C23D70A, 0x3E4CCCCD, 0x3E99999A, 3, 25, 124, 77,
        ),
        (
            0x3F666666, 0x3F4CCCCD, 0x3B4D2E1C, 0x3F7F7CEE, 243, 231, 10, 254,
        ),
        (
            0xBDCCCCCD, 0x3F99999A, 0x3E5B2D97, 0x3F333333, 0, 255, 127, 179,
        ),
    ];
    for (r, g, b, a, er, eg, eb, ea) in cases {
        let c = Rgba32::new(
            f32::from_bits(r),
            f32::from_bits(g),
            f32::from_bits(b),
            f32::from_bits(a),
        );

        let s = c.to_srgba8();

        assert_eq!(s.r, er);
        assert_eq!(s.g, eg);
        assert_eq!(s.b, eb);
        assert_eq!(s.a, ea);
    }
}
