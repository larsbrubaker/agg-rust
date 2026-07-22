// Byte-for-byte parity test between the native and WASM `rasterizers2` renderers.
//
// The native renderer (`pixel_compare::render::render_demo`) is proven
// byte-identical to the C++ AGG reference by `rasterizers2_reference.rs`. This
// parity test therefore transitively proves the WASM render path
// (`agg_wasm::render_demo`, the real production WASM entry point, which compiles
// and runs natively) also matches the C++ reference.
//
// It guards the three fixes that were ported into the WASM version:
//   1. The premultiplied main scene via `PixfmtRgba32Pre`.
//   2. The sRGB-decoded + premultiplied image pattern source.
//   3. The full-precision `ImagePatternSource::pixel_rgba` scaling path.

#[test]
fn rasterizers2_wasm_matches_native_500x450() {
    let width: u32 = 500;
    let height: u32 = 450;

    let native = pixel_compare::render::render_demo("rasterizers2", width, height, &[])
        .expect("native rasterizers2 renderer missing");
    let wasm = agg_wasm::render_demo("rasterizers2", width, height, &[]);

    assert_eq!(
        native.data.len(),
        wasm.len(),
        "rasterizers2: native/WASM byte length mismatch ({} vs {})",
        native.data.len(),
        wasm.len(),
    );

    if native.data != wasm {
        let diff_count = native
            .data
            .iter()
            .zip(wasm.iter())
            .filter(|(a, b)| a != b)
            .count();
        let first = native
            .data
            .iter()
            .zip(wasm.iter())
            .position(|(a, b)| a != b)
            .unwrap();
        let px = first / 4;
        panic!(
            "rasterizers2: WASM path diverged from native reference: {diff_count} differing \
             bytes; first at byte {first} (pixel {}, {}): native={:?} wasm={:?}",
            px % width as usize,
            px / width as usize,
            &native.data[px * 4..px * 4 + 4],
            &wasm[px * 4..px * 4 + 4],
        );
    }
}
