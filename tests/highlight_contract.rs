// What this test guards
// ---------------------
// `LayoutRun::highlight` documents: "Returns an empty iterator if the
// cursor range does not intersect this run." The per-grapheme test
// inside only disambiguates *within* the boundary lines — for a run
// whose line differs from both cursor lines, both arms of the check
// were vacuously true and the whole line reported as selected. Lines
// *between* the cursors are exactly that case and must stay fully
// highlighted; lines *outside* the selection hit the same logic and
// painted phantom full-line highlights. The editor never noticed
// because it pre-filters runs to the selection's line range; any
// external caller following the doc got the false contract.
//
// The guard now returns empty for runs outside
// [cursor_start.line, cursor_end.line]. The two fences pin that the
// guard did not eat the legitimate cases.

use kalamos::{fontdb, Attrs, Buffer, Cursor, FontSystem, Metrics, Shaping, Wrap};

fn shaped_buffer(font_system: &mut FontSystem) -> Buffer {
    let mut buffer = Buffer::new(font_system, Metrics::new(14.0, 20.0));
    buffer.set_wrap(Wrap::None);
    buffer.set_text(
        "hello\nworld\nagain",
        &Attrs::new(),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(font_system, false);
    buffer
}

fn font_system() -> FontSystem {
    let mut font_system =
        FontSystem::new_with_locale_and_db("en-US".into(), fontdb::Database::new());
    font_system
        .db_mut()
        .load_font_data(std::fs::read("fonts/Inter-Regular.ttf").unwrap());
    font_system
}

fn highlights_for_line(
    buffer: &Buffer,
    line_i: usize,
    start: Cursor,
    end: Cursor,
) -> Vec<(f32, f32)> {
    buffer
        .layout_runs()
        .filter(|run| run.line_i == line_i)
        .flat_map(|run| run.highlight(start, end))
        .collect()
}

#[test]
fn line_outside_the_selection_yields_nothing() {
    let mut font_system = font_system();
    let buffer = shaped_buffer(&mut font_system);
    // Selection entirely on line 1.
    let start = Cursor::new(1, 0);
    let end = Cursor::new(1, 5);

    assert_eq!(
        highlights_for_line(&buffer, 0, start, end),
        vec![],
        "line 0 is before the selection"
    );
    assert_eq!(
        highlights_for_line(&buffer, 2, start, end),
        vec![],
        "line 2 is after the selection"
    );
}

#[test]
fn line_between_the_boundaries_is_fully_highlighted() {
    let mut font_system = font_system();
    let buffer = shaped_buffer(&mut font_system);
    let start = Cursor::new(0, 1);
    let end = Cursor::new(2, 1);

    let spans = highlights_for_line(&buffer, 1, start, end);
    assert_eq!(spans.len(), 1, "one contiguous span for a pure-LTR line");
    let (x, w) = spans[0];
    assert_eq!(x, 0.0, "the span starts at the line's left edge");
    assert!(w > 0.0);
}

#[test]
fn intra_line_selection_highlights_a_partial_span() {
    let mut font_system = font_system();
    let buffer = shaped_buffer(&mut font_system);
    let start = Cursor::new(0, 1);
    let end = Cursor::new(0, 3);

    let spans = highlights_for_line(&buffer, 0, start, end);
    assert_eq!(spans.len(), 1);
    let (x, w) = spans[0];
    assert!(x > 0.0, "the span starts after the first glyph");
    assert!(w > 0.0);

    let full = highlights_for_line(&buffer, 0, Cursor::new(0, 0), Cursor::new(0, 5));
    assert!(
        w < full[0].1,
        "two graphemes highlight narrower than the whole line"
    );
}
