// What this test guards
// ---------------------
// Justification expansion is (line_width - visual_line.w) / spaces, clamped
// at zero. Unclamped, a visual line wider than line_width would COMPRESS
// its blanks — negative glyph widths and backward x — silently corrupting
// caret geometry. Probing found no public-API construction that reaches the
// negative branch today (overflowing lines shed their blanks during wrap:
// a long word overflows alone, and a blank run gets its own fitting line),
// so these are fences, green on both sides of the clamp: they pin that
// justified layout never produces negative widths or backward x, across
// the three constructions the probe tried.

use kalamos::{fontdb, Align, Attrs, Buffer, FontSystem, Metrics, Shaping, Wrap};

fn font_system() -> FontSystem {
    let mut font_system =
        FontSystem::new_with_locale_and_db("en-US".into(), fontdb::Database::new());
    font_system
        .db_mut()
        .load_font_data(std::fs::read("fonts/Inter-Regular.ttf").unwrap());
    font_system
}

fn assert_sane_geometry(text: &str, width: f32) {
    let mut font_system = font_system();
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(16.0, 20.0));
    buffer.set_size(Some(width), None);
    buffer.set_wrap(Wrap::Word);
    buffer.set_text(text, &Attrs::new(), Shaping::Advanced, None);
    for line in &mut buffer.lines {
        line.set_align(Some(Align::Justified));
    }
    buffer.shape_until_scroll(&mut font_system, false);

    let mut runs = 0;
    for run in buffer.layout_runs() {
        runs += 1;
        let mut previous_end = f32::NEG_INFINITY;
        for glyph in run.glyphs.iter() {
            assert!(
                glyph.w >= 0.0,
                "justified glyph width must not be negative: {:?} w {}",
                &run.text[glyph.start..glyph.end],
                glyph.w
            );
            assert!(
                glyph.x >= previous_end - 0.01,
                "justified glyphs must not move backward on an LTR line: \
                 {:?} at x {} after edge {previous_end}",
                &run.text[glyph.start..glyph.end],
                glyph.x
            );
            previous_end = glyph.x + glyph.w;
        }
    }
    assert!(runs > 0, "the layout must produce runs");
}

#[test]
fn overflowing_word_between_fitting_lines() {
    assert_sane_geometry("aa bb cccccccccccccccccccc dd ee", 60.0);
}

#[test]
fn long_blank_run_after_an_overflowing_word() {
    assert_sane_geometry("cccccccccccccccccccc          dd ee", 60.0);
}

#[test]
fn every_word_overflows_the_width() {
    assert_sane_geometry("aaa aaa aaa", 20.0);
}
