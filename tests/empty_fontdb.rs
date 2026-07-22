// What this test guards
// ---------------------
// Shaping used to reach `font_iter.next().expect("no default font
// found")` — but an empty font database is an environmental condition,
// not an invariant: `FontSystem::new_with_locale_and_db` with a fresh
// `fontdb::Database` constructs one in two lines, and a misconfigured
// system (no fonts installed, broken fontconfig) does it in zero. Even
// `Buffer::new` panicked, because shaping the buffer's initial empty
// line already selects a font.
//
// Shaping now degrades: a run that no font can shape produces zero
// glyphs and a warning, the buffer stays alive and measurable, and the
// process gets to report its own misconfiguration.

use kalamos::{fontdb, Attrs, Buffer, FontSystem, Metrics, Shaping, Wrap};

#[test]
fn empty_database_shapes_to_nothing_without_panicking() {
    let mut font_system =
        FontSystem::new_with_locale_and_db("en-US".into(), fontdb::Database::new());

    // Old code died right here, in Buffer::new.
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 20.0));
    buffer.set_wrap(Wrap::None);
    buffer.set_text("مرحبا hello", &Attrs::new(), Shaping::Advanced, None);
    buffer.shape_until_scroll(&mut font_system, false);

    let glyphs: usize = buffer.layout_runs().map(|run| run.glyphs.len()).sum();
    assert_eq!(glyphs, 0, "no font, no glyphs - but also no panic");
}

#[test]
fn empty_database_basic_shaping_survives_too() {
    let mut font_system =
        FontSystem::new_with_locale_and_db("en-US".into(), fontdb::Database::new());

    let mut buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 20.0));
    buffer.set_text("plain ascii", &Attrs::new(), Shaping::Basic, None);
    buffer.shape_until_scroll(&mut font_system, false);

    let glyphs: usize = buffer.layout_runs().map(|run| run.glyphs.len()).sum();
    assert_eq!(glyphs, 0);
}
