// What this test guards
// ---------------------
// `hit()` has two vertical arms: the in-line arm covers
// [line_top, line_top + line_height), and the below-last-line arm used
// to fire on `y > run.line_y` — the BASELINE (line_top + centering +
// max_ascent), not the line-box bottom. With a font whose
// ascent − descent exceeds the line height (font_size 32 on
// line_height 10 — perfectly legal metrics), the baseline sits below
// the line box: clicks with y between the box bottom and the baseline
// matched neither arm and were silently dropped.
//
// The below arm now fires on y >= line_top + line_height, the exact
// complement of the in-line arm: every y maps to a cursor.

use kalamos::{fontdb, Attrs, Buffer, FontSystem, Metrics, Shaping, Wrap};

fn font_system() -> FontSystem {
    let mut font_system =
        FontSystem::new_with_locale_and_db("en-US".into(), fontdb::Database::new());
    font_system
        .db_mut()
        .load_font_data(std::fs::read("fonts/Inter-Regular.ttf").unwrap());
    font_system
}

fn degenerate_buffer(font_system: &mut FontSystem) -> Buffer {
    // Big font, small line height: the baseline lands below the line box.
    let mut buffer = Buffer::new(font_system, Metrics::new(32.0, 10.0));
    buffer.set_wrap(Wrap::None);
    buffer.set_text("hello", &Attrs::new(), Shaping::Advanced, None);
    buffer.shape_until_scroll(font_system, false);
    buffer
}

#[test]
fn click_between_box_bottom_and_baseline_hits_the_line_end() {
    let mut font_system = font_system();
    let buffer = degenerate_buffer(&mut font_system);

    let run = buffer.layout_runs().next().expect("one run");
    let box_bottom = run.line_top + run.line_height;
    assert!(
        run.line_y > box_bottom,
        "precondition: the baseline ({}) sits below the line box ({box_bottom})",
        run.line_y
    );
    let dead_window_y = (box_bottom + run.line_y) / 2.0;

    let cursor = buffer.hit(0.0, dead_window_y);
    assert_eq!(
        cursor.map(|c| (c.line, c.index)),
        Some((0, 5)),
        "a click below the line box lands at the end of the last line, not nowhere"
    );
}

#[test]
fn click_inside_the_box_still_hits_glyphs() {
    let mut font_system = font_system();
    let buffer = degenerate_buffer(&mut font_system);

    let cursor = buffer.hit(0.5, 5.0).expect("in-box click resolves");
    assert_eq!(cursor.line, 0);
    assert_eq!(
        cursor.index, 0,
        "a click at the left edge is the line start"
    );
}

#[test]
fn click_far_below_still_hits_the_line_end() {
    let mut font_system = font_system();
    let buffer = degenerate_buffer(&mut font_system);

    let cursor = buffer.hit(0.0, 500.0).expect("far-below click resolves");
    assert_eq!((cursor.line, cursor.index), (0, 5));
}
