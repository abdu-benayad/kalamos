// Pins the fork's caret-motion convention: `Motion::Left`/`Right` map to logical
// `Next`/`Previous` by the LINE's base direction only — the GTK/Firefox model —
// so on a mixed-direction line the caret advances in logical (byte) order and
// visually reverses inside an embedded opposite-direction run. That reversal is
// the convention, not a bug.
//
// This is a decision, not an inheritance. A visual-order caret (the macOS/parley
// model, `3a6cc2b6` + `518a813f`) was implemented and then deliberately removed:
// the first consumer (abdu-egui-ui's field editor) documents and pins the logical
// contract, and one stack ships one convention. This test is the kalamos-level
// record of that decision — it was watched failing against the visual-order
// implementation before the removal turned it green. If it reddens, caret order
// changed; that is a design conversation, not a test to update.

use kalamos::{
    fontdb, Attrs, Buffer, Cursor, Direction, FontSystem, Metrics, Motion, Shaping, Wrap,
};

fn font_system() -> FontSystem {
    let mut font_system = FontSystem::new_with_locale_and_db("ar".into(), fontdb::Database::new());
    font_system
        .db_mut()
        .load_font_data(std::fs::read("fonts/Inter-Regular.ttf").expect("Inter present"));
    font_system
        .db_mut()
        .load_font_data(std::fs::read("fonts/NotoSansArabic.ttf").expect("Noto Arabic present"));
    font_system
}

fn buffer_of(font_system: &mut FontSystem, text: &str, direction: Direction) -> Buffer {
    let mut buffer = Buffer::new(font_system, Metrics::new(14.0, 20.0));
    buffer.set_wrap(Wrap::None);
    buffer.set_direction(direction);
    buffer.set_text(text, &Attrs::new(), Shaping::Advanced, None);
    buffer.shape_until_scroll(font_system, false);
    buffer
}

/// Walk `motion` from `start`, `steps` times, collecting the byte index after
/// each step. Stops early if the buffer reports no further motion.
fn walk(
    font_system: &mut FontSystem,
    buffer: &mut Buffer,
    start: Cursor,
    motion: Motion,
    steps: usize,
) -> Vec<usize> {
    let mut cursor = start;
    let mut cursor_x_opt = None;
    let mut indices = Vec::with_capacity(steps);
    for _ in 0..steps {
        let Some((next, next_x)) = buffer.cursor_motion(font_system, cursor, cursor_x_opt, motion)
        else {
            break;
        };
        cursor = next;
        cursor_x_opt = next_x;
        indices.push(cursor.index);
    }
    indices
}

// "المبلغ 250": Arabic 0..12 (six 2-byte chars), space 12..13, LTR digits 13..16.
const KIOSK_RTL: &str = "المبلغ 250";
const KIOSK_RTL_BOUNDARIES: [usize; 10] = [2, 4, 6, 8, 10, 12, 13, 14, 15, 16];

#[test]
fn left_on_an_rtl_base_line_advances_in_logical_order() {
    let mut font_system = font_system();
    let mut buffer = buffer_of(&mut font_system, KIOSK_RTL, Direction::RightToLeft);
    let walked = walk(
        &mut font_system,
        &mut buffer,
        Cursor::new(0, 0),
        Motion::Left,
        KIOSK_RTL_BOUNDARIES.len(),
    );
    assert_eq!(
        walked, KIOSK_RTL_BOUNDARIES,
        "Left on an RTL-base line is logical-forward, straight through the embedded digits"
    );
}

#[test]
fn right_on_an_rtl_base_line_retreats_in_logical_order() {
    let mut font_system = font_system();
    let mut buffer = buffer_of(&mut font_system, KIOSK_RTL, Direction::RightToLeft);
    let walked_back = walk(
        &mut font_system,
        &mut buffer,
        Cursor::new(0, KIOSK_RTL.len()),
        Motion::Right,
        KIOSK_RTL_BOUNDARIES.len(),
    );
    let expected: Vec<usize> = core::iter::once(0)
        .chain(
            KIOSK_RTL_BOUNDARIES
                .into_iter()
                .take(KIOSK_RTL_BOUNDARIES.len() - 1),
        )
        .rev()
        .collect();
    assert_eq!(
        walked_back, expected,
        "Right on an RTL-base line is logical-backward, the exact reverse of Left"
    );
}

// "total مبلغ 250": Latin 0..5, space 5..6, Arabic 6..14 (four 2-byte chars),
// space 14..15, LTR digits 15..18.
const KIOSK_LTR: &str = "total مبلغ 250";
const KIOSK_LTR_BOUNDARIES: [usize; 14] = [1, 2, 3, 4, 5, 6, 8, 10, 12, 14, 15, 16, 17, 18];

#[test]
fn right_on_an_ltr_base_line_advances_in_logical_order() {
    let mut font_system = font_system();
    let mut buffer = buffer_of(&mut font_system, KIOSK_LTR, Direction::LeftToRight);
    let walked = walk(
        &mut font_system,
        &mut buffer,
        Cursor::new(0, 0),
        Motion::Right,
        KIOSK_LTR_BOUNDARIES.len(),
    );
    assert_eq!(
        walked, KIOSK_LTR_BOUNDARIES,
        "Right on an LTR-base line is logical-forward, no zigzag at the Arabic run boundary"
    );
}
