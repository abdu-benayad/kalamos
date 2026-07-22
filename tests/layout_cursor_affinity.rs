// What this test guards
// ---------------------
// `Buffer::layout_cursor` used to recognize only the two exact cursors
// that name a glyph edge — `(start, After)` and `(end, Before)` — and
// fell back to `LayoutCursor::new(line, 0, 0)` for everything else. But
// the motions manufacture cursors outside that set: `Motion::Previous`
// across a line boundary, `PreviousWord`, and `ParagraphEnd` all produce
// `(len, After)`, which no glyph edge spells. The fallback teleported the
// caret to the line start, seeded `cursor_x_opt` with column 0, and on a
// wrapped line claimed layout row 0 — so Up from the end of a wrapped
// line skipped the line's own rows.
//
// Resolution is now total by byte position, in the one frame that is
// coherent: the `Motion::LayoutCursor` decoder, where slot g on a row
// means `(glyphs[g].start, After)` and the row's one-past-the-end slot
// means `(last.end, Before)`. `LayoutLine::glyphs` is stored in visual
// traversal order along the line's BASE direction (x descending for an
// RTL line), not logical order — so byte-position matching, not visual
// geometry, is what stays correct across direction mixes. A cursor the
// exact pass cannot spell snaps to the slot whose decoded byte is
// nearest; `(line, 0, 0)` remains only for glyph-less lines.
//
// `line_start_before_stays_at_line_start` and
// `rtl_line_start_before_resolves_to_slot_zero` pass on the old code too
// (there the old fallback happened to coincide); they are fences for the
// semantics, not proofs of the fix.

use kalamos::{
    fontdb, Affinity, Attrs, Buffer, Cursor, FontSystem, Metrics, Motion, Shaping, Wrap,
};

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

fn single_line_buffer(font_system: &mut FontSystem, text: &str) -> Buffer {
    let mut buffer = Buffer::new(font_system, Metrics::new(14.0, 20.0));
    buffer.set_wrap(Wrap::None);
    buffer.set_text(text, &Attrs::new(), Shaping::Advanced, None);
    buffer
}

/// A buffer whose single logical line wraps into exactly two layout rows.
fn wrapped_buffer(font_system: &mut FontSystem) -> Buffer {
    let mut buffer = Buffer::new(font_system, Metrics::new(14.0, 20.0));
    buffer.set_wrap(Wrap::Word);
    buffer.set_size(Some(50.0), None);
    buffer.set_text("aaaa bbbb", &Attrs::new(), Shaping::Advanced, None);
    let rows = buffer.line_layout(font_system, 0).unwrap().len();
    assert_eq!(rows, 2, "precondition: 'aaaa bbbb' wraps into two rows");
    buffer
}

#[test]
fn eol_after_resolves_to_line_end() {
    let mut font_system = font_system();
    let mut buffer = single_line_buffer(&mut font_system, "hello");

    let cursor = Cursor::new_with_affinity(0, 5, Affinity::After);
    let layout_cursor = buffer.layout_cursor(&mut font_system, cursor).unwrap();

    assert_eq!(layout_cursor.layout, 0);
    assert_eq!(
        layout_cursor.glyph, 5,
        "(len, After) is the end of the line, not column 0"
    );
}

#[test]
fn eol_after_on_wrapped_line_lands_in_last_row() {
    let mut font_system = font_system();
    let mut buffer = wrapped_buffer(&mut font_system);
    let last_row_glyphs = buffer.line_layout(&mut font_system, 0).unwrap()[1]
        .glyphs
        .len();

    let cursor = Cursor::new_with_affinity(0, 9, Affinity::After);
    let layout_cursor = buffer.layout_cursor(&mut font_system, cursor).unwrap();

    assert_eq!(
        layout_cursor.layout, 1,
        "(len, After) lives in the last layout row, not row 0"
    );
    assert_eq!(layout_cursor.glyph, last_row_glyphs);
}

#[test]
fn line_start_before_stays_at_line_start() {
    let mut font_system = font_system();
    let mut buffer = single_line_buffer(&mut font_system, "hello");

    let cursor = Cursor::new_with_affinity(0, 0, Affinity::Before);
    let layout_cursor = buffer.layout_cursor(&mut font_system, cursor).unwrap();

    assert_eq!(layout_cursor.layout, 0);
    assert_eq!(layout_cursor.glyph, 0);
}

#[test]
fn rtl_line_start_before_resolves_to_slot_zero() {
    let mut font_system = font_system();
    let mut buffer = single_line_buffer(&mut font_system, "سلام");

    // On a pure RTL line the glyph vec runs x-descending, so vec glyph 0
    // is the logically first cluster and slot 0 decodes to (0, After) —
    // the same byte position. The old fallback gave the same answer by
    // accident; this fence keeps it.
    let cursor = Cursor::new_with_affinity(0, 0, Affinity::Before);
    let layout_cursor = buffer.layout_cursor(&mut font_system, cursor).unwrap();

    assert_eq!(layout_cursor.layout, 0);
    assert_eq!(layout_cursor.glyph, 0);
}

#[test]
fn rtl_eol_after_resolves_to_line_end() {
    let mut font_system = font_system();
    let mut buffer = single_line_buffer(&mut font_system, "سلام");
    let row_glyphs = buffer.line_layout(&mut font_system, 0).unwrap()[0]
        .glyphs
        .len();

    // The one-past-the-end slot decodes to (last.end, Before) = (len,
    // Before): the logical end, visually the line's left edge. The old
    // fallback teleported this to slot 0 — byte 0, the OTHER end of the
    // line.
    let cursor = Cursor::new_with_affinity(0, 8, Affinity::After);
    let layout_cursor = buffer.layout_cursor(&mut font_system, cursor).unwrap();

    assert_eq!(layout_cursor.layout, 0);
    assert_eq!(
        layout_cursor.glyph, row_glyphs,
        "(len, After) on an RTL line is the logical end slot, not byte 0"
    );
}

#[test]
fn cluster_interior_snaps_to_cluster_start() {
    let mut font_system = font_system();
    let mut buffer = single_line_buffer(&mut font_system, "سلام");

    // Byte 4 is the lam–alef boundary. Noto Sans Arabic ligates lam+alef
    // into one cluster spanning [2, 6), so no glyph edge spells index 4:
    // the old code teleported to slot 0 (byte 0). Byte 4 is equidistant
    // from the decodable bytes 2 (slot 1) and 6 (slot 2); the earlier
    // slot wins, snapping to the cluster's logical start. This pins the
    // ligated shape — if the ligature ever stops forming, the exact pass
    // resolves index 4 instead and this test must be revisited.
    let cursor = Cursor::new_with_affinity(0, 4, Affinity::After);
    let layout_cursor = buffer.layout_cursor(&mut font_system, cursor).unwrap();

    assert_eq!(layout_cursor.layout, 0);
    assert_eq!(layout_cursor.glyph, 1);
}

#[test]
fn up_from_end_of_wrapped_line_stays_in_line() {
    let mut font_system = font_system();
    let mut buffer = wrapped_buffer(&mut font_system);

    let end = Cursor::new_with_affinity(0, 9, Affinity::After);
    let (cursor, _) = buffer
        .cursor_motion(&mut font_system, end, None, Motion::Up)
        .expect("up from end of wrapped line resolves");

    assert_eq!(cursor.line, 0);
    assert_eq!(
        cursor.index, 4,
        "up from the end of row 1 keeps the column in row 0"
    );
}

#[test]
fn previous_then_up_preserves_column() {
    let mut font_system = font_system();
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 20.0));
    buffer.set_wrap(Wrap::None);
    buffer.set_text(
        "hello world\nhi\nabc",
        &Attrs::new(),
        Shaping::Advanced,
        None,
    );

    // Left at the start of line 2 lands at the end of line 1 as
    // (len, After) — exactly the cursor the exact pass cannot spell.
    let (cursor, cursor_x_opt) = buffer
        .cursor_motion(&mut font_system, Cursor::new(2, 0), None, Motion::Previous)
        .expect("previous resolves");
    assert_eq!((cursor.line, cursor.index), (1, 2));
    assert_eq!(cursor.affinity, Affinity::After);

    let (cursor, _) = buffer
        .cursor_motion(&mut font_system, cursor, cursor_x_opt, Motion::Up)
        .expect("up resolves");

    assert_eq!(cursor.line, 0);
    assert_eq!(
        cursor.index, 2,
        "up from the end of 'hi' preserves the column, not column 0"
    );
}
