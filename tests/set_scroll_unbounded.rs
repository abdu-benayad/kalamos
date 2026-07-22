// What this test guards
// ---------------------
// With no window height (`height_opt = None`), shape_until_scroll's
// "scroll up to stay inside of buffer" branch compared accumulated
// height against `vertical + INFINITY`, fired for any scroll.line > 0,
// subtracted infinity, and the negative-vertical walk-back dragged the
// scroll to line 0 — silently reverting any set_scroll on an unbounded
// buffer (and flagging a spurious redraw). The branch exists to keep a
// BOUNDED window full when scrolled past the end of the content; with
// no height there is no window to fill, and it now requires one. The
// fence pins the bounded behavior it was written for.

use kalamos::{fontdb, Attrs, Buffer, FontSystem, Metrics, Scroll, Shaping, Wrap};

fn font_system() -> FontSystem {
    let mut font_system =
        FontSystem::new_with_locale_and_db("en-US".into(), fontdb::Database::new());
    font_system
        .db_mut()
        .load_font_data(std::fs::read("fonts/Inter-Regular.ttf").unwrap());
    font_system
}

fn forty_line_buffer(font_system: &mut FontSystem) -> Buffer {
    let mut buffer = Buffer::new(font_system, Metrics::new(14.0, 20.0));
    buffer.set_wrap(Wrap::None);
    let text = (0..40).map(|i| format!("line {i}")).collect::<Vec<_>>();
    buffer.set_text(&text.join("\n"), &Attrs::new(), Shaping::Advanced, None);
    buffer
}

#[test]
fn set_scroll_persists_without_a_window_height() {
    let mut font_system = font_system();
    let mut buffer = forty_line_buffer(&mut font_system);

    buffer.set_scroll(Scroll::new(10, 0.0, 0.0));
    buffer.shape_until_scroll(&mut font_system, false);

    assert_eq!(
        buffer.scroll().line,
        10,
        "an unbounded buffer honors the scroll it was given"
    );
    let first_visible = buffer.layout_runs().next().map(|run| run.line_i);
    assert_eq!(first_visible, Some(10));
}

#[test]
fn bounded_buffer_scrolled_past_the_end_still_walks_back() {
    let mut font_system = font_system();
    let mut buffer = forty_line_buffer(&mut font_system);
    // Window of two lines (line_height 20), scrolled to the last line:
    // the window would show one line and empty space, so the walk-back
    // must pull the scroll up to keep it full.
    buffer.set_size(None, Some(40.0));

    buffer.set_scroll(Scroll::new(39, 0.0, 0.0));
    buffer.shape_until_scroll(&mut font_system, false);

    assert!(
        buffer.scroll().line < 39,
        "a bounded window scrolled past the end pulls back, got line {}",
        buffer.scroll().line
    );
}
