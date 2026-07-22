// What this test guards
// ---------------------
// `ShapeRunKey` must include the bidi direction the run was shaped with
// (`span_rtl`). The cache unit is a word-level run keyed by its text and
// attrs; a direction-neutral word such as "(!)" legitimately occurs at
// level 1 inside an Arabic sentence and at level 0 inside a Latin one.
// Direction changes the shaped output — mirroring of paired brackets,
// joining forms, glyph order — so a key that omits it serves glyphs shaped
// for the wrong direction to whichever context shapes second.
//
// The test is differential: shape a line in a FontSystem whose cache was
// deliberately pre-poisoned by the same neutral word in the opposite
// direction, and compare the glyph stream against a fresh FontSystem that
// never saw the other direction. Both orders are exercised. With the
// direction missing from the key, the poisoned stream differs and this
// file goes red; it is meaningless without `shape-run-cache`, hence the
// crate-level cfg.
#![cfg(feature = "shape-run-cache")]

use kalamos::{fontdb, Attrs, Buffer, FontSystem, Metrics, Shaping, Wrap};

fn font_system() -> FontSystem {
    let mut font_system =
        FontSystem::new_with_locale_and_db("en-US".into(), fontdb::Database::new());
    font_system
        .db_mut()
        .load_font_data(std::fs::read("fonts/Inter-Regular.ttf").unwrap());
    font_system
        .db_mut()
        .load_font_data(std::fs::read("fonts/NotoSansArabic.ttf").unwrap());
    font_system
}

/// The glyph stream of every layout run: glyph id plus the logical cluster
/// range it covers, in visual emission order. Direction bugs surface here as
/// reversed order or substituted (mirrored) glyph ids.
fn glyph_stream(font_system: &mut FontSystem, text: &str) -> Vec<(u16, usize, usize)> {
    let mut buffer = Buffer::new(font_system, Metrics::new(14.0, 20.0));
    buffer.set_wrap(Wrap::None);
    buffer.set_text(text, &Attrs::new(), Shaping::Advanced, None);
    buffer.shape_until_scroll(font_system, false);
    buffer
        .layout_runs()
        .flat_map(|run| run.glyphs.iter().map(|g| (g.glyph_id, g.start, g.end)))
        .collect::<Vec<_>>()
}

/// Shared neutral word: every character is bidi-neutral, so its level — and
/// therefore its shaped form — comes entirely from the surrounding paragraph.
const NEUTRAL: &str = "(!)";

fn arabic_line() -> String {
    format!("ابجد {NEUTRAL} ابجد")
}

fn latin_line() -> String {
    format!("abcd {NEUTRAL} abcd")
}

#[test]
fn ltr_line_survives_rtl_poisoned_cache() {
    // Poison: shape the neutral word inside an RTL paragraph first.
    let mut poisoned = font_system();
    let _ = glyph_stream(&mut poisoned, &arabic_line());
    let via_poisoned = glyph_stream(&mut poisoned, &latin_line());

    let mut fresh = font_system();
    let expected = glyph_stream(&mut fresh, &latin_line());

    assert_eq!(
        via_poisoned, expected,
        "LTR glyph stream changed after the shape-run cache saw the same word RTL"
    );
}

#[test]
fn rtl_line_survives_ltr_poisoned_cache() {
    // Poison in the opposite order: LTR first, then the RTL paragraph.
    let mut poisoned = font_system();
    let _ = glyph_stream(&mut poisoned, &latin_line());
    let via_poisoned = glyph_stream(&mut poisoned, &arabic_line());

    let mut fresh = font_system();
    let expected = glyph_stream(&mut fresh, &arabic_line());

    assert_eq!(
        via_poisoned, expected,
        "RTL glyph stream changed after the shape-run cache saw the same word LTR"
    );
}
