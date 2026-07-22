// What this test guards
// ---------------------
// SubpixelBin::new splits a float position into an integer pixel and a
// subpixel bin. `pos as i32` saturates for positions beyond i32's range,
// which leaves `fract` enormous — so the bin logic falls into the branches
// that adjust the pixel by one, and `trunc - 1` / `trunc + 1` overflowed at
// the saturation boundary (debug panic, release wrap to the far end of the
// axis). Both extremes are reachable through the public API: CacheKey::new
// takes a caller-supplied (f32, f32) position.
//
// The pinned contract: the pixel adjustment saturates, matching the cast's
// own saturation — a position past the end of the i32 axis stays clamped to
// that end. Watched red (overflow panic in both directions) before the fix.

use kalamos::SubpixelBin;

#[test]
fn a_hugely_negative_position_clamps_to_the_axis_start() {
    let (x, _bin) = SubpixelBin::new(f32::MIN);
    assert_eq!(x, i32::MIN);
}

#[test]
fn a_hugely_positive_position_clamps_to_the_axis_end() {
    let (x, _bin) = SubpixelBin::new(f32::MAX);
    assert_eq!(x, i32::MAX);
}

#[test]
fn a_nan_position_stays_finite() {
    let (x, bin) = SubpixelBin::new(f32::NAN);
    // NaN casts to 0; whatever bin it lands in, the pixel must stay near 0
    // rather than panicking or wrapping.
    assert!(
        (-1..=1).contains(&x),
        "NaN mapped to pixel {x}, bin {bin:?}"
    );
}
