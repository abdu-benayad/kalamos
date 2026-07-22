// What this test guards
// ---------------------
// `layout_cursor`'s exact pass and `Motion::LayoutCursor` are the two
// halves of one roundtrip: vertical motion encodes the cursor to a
// (row, slot) pair, adjusts the row, and decodes back. The decoder is
// level-blind — slot g means `(glyphs[g].start, After)`, one past the
// end means `(last.end, Before)` — but the encoder used to swap the
// slot sides for RTL-level glyphs (a visual-left/right frame). On RTL
// text encode(decode) advanced one slot per cycle: the caret crept one
// cluster toward the logical line end on every fresh vertical-motion
// sequence (fresh = after any horizontal move reset cursor_x_opt).
//
// The encoder now speaks the decoder's frame for every glyph. The pins
// are roundtrips: Down preserves the byte position on identical Arabic
// lines (it used to land one cluster off), a Down-Up-Up excursion
// returns to its origin, and the LTR fence keeps the frame that was
// already coherent.

use kalamos::{
    fontdb, Affinity, Attrs, Buffer, Cursor, FontSystem, Metrics, Motion, Shaping, Wrap,
};

fn font_system() -> FontSystem {
    let mut font_system =
        FontSystem::new_with_locale_and_db("en-US".into(), fontdb::Database::new());
    font_system
        .db_mut()
        .load_font_data(std::fs::read("fonts/NotoSansArabic.ttf").unwrap());
    font_system
        .db_mut()
        .load_font_data(std::fs::read("fonts/Inter-Regular.ttf").unwrap());
    font_system
}

fn arabic_buffer(font_system: &mut FontSystem) -> Buffer {
    let mut buffer = Buffer::new(font_system, Metrics::new(14.0, 20.0));
    buffer.set_wrap(Wrap::None);
    // Three identical lines: any vertical motion should preserve the
    // byte position exactly.
    buffer.set_text("سلام\nسلام\nسلام", &Attrs::new(), Shaping::Advanced, None);
    buffer
}

#[test]
fn down_preserves_the_byte_position() {
    let mut font_system = font_system();
    let mut buffer = arabic_buffer(&mut font_system);

    // Byte 2: after the seen, before the lam-alef cluster.
    let start = Cursor::new_with_affinity(0, 2, Affinity::After);
    let (cursor, _) = buffer
        .cursor_motion(&mut font_system, start, None, Motion::Down)
        .expect("down resolves");

    assert_eq!(cursor.line, 1);
    assert_eq!(
        cursor.index, 2,
        "identical lines: down keeps the byte position (it used to land at 6)"
    );
}

#[test]
fn down_up_up_returns_to_the_origin_row_position() {
    let mut font_system = font_system();
    let mut buffer = arabic_buffer(&mut font_system);

    let mut cursor = Cursor::new_with_affinity(1, 2, Affinity::After);
    let mut cursor_x_opt = None;
    for motion in [Motion::Down, Motion::Up, Motion::Up] {
        (cursor, cursor_x_opt) = buffer
            .cursor_motion(&mut font_system, cursor, cursor_x_opt, motion)
            .expect("motion resolves");
    }

    assert_eq!(
        (cursor.line, cursor.index),
        (0, 2),
        "no drift across the excursion"
    );
}

#[test]
fn ltr_down_preserves_the_byte_position() {
    let mut font_system = font_system();
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 20.0));
    buffer.set_wrap(Wrap::None);
    buffer.set_text("hello\nhello", &Attrs::new(), Shaping::Advanced, None);

    let start = Cursor::new_with_affinity(0, 2, Affinity::After);
    let (cursor, _) = buffer
        .cursor_motion(&mut font_system, start, None, Motion::Down)
        .expect("down resolves");

    assert_eq!((cursor.line, cursor.index), (1, 2));
}
