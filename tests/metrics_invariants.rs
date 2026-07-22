// What this test guards
// ---------------------
// Two related defects around Metrics validation:
//
// * `Motion::Vertical` computed `px / line_height as i32` — the cast
//   truncates any line height in (0, 1) to zero and the integer
//   division panicked. A line height of 0.5 is a legal metric (scaled
//   UIs); the division now happens in f32.
// * The constructor boundaries guarded with `assert_ne!(x, 0.0)`,
//   which NaN, ±∞, and negatives all pass — NaN because it compares
//   unequal to everything, including zero. `Buffer::new_empty` also
//   validated only line height, not font size, while `set_metrics`
//   checked both. Both boundaries now require strictly positive,
//   finite values for both fields.

use kalamos::{fontdb, Attrs, Buffer, Cursor, FontSystem, Metrics, Motion, Shaping, Wrap};

fn font_system() -> FontSystem {
    let mut font_system =
        FontSystem::new_with_locale_and_db("en-US".into(), fontdb::Database::new());
    font_system
        .db_mut()
        .load_font_data(std::fs::read("fonts/Inter-Regular.ttf").unwrap());
    font_system
}

#[test]
fn vertical_motion_with_subunit_line_height() {
    let mut font_system = font_system();
    // line_height 0.5 passes every constructor check — it is a valid
    // metric — and used to truncate to 0 in the integer division.
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(12.0, 0.5));
    buffer.set_wrap(Wrap::None);
    buffer.set_text("a\nb\nc\nd", &Attrs::new(), Shaping::Advanced, None);
    buffer.shape_until_scroll(&mut font_system, false);

    // 1px at 0.5px per line = 2 lines down.
    let (cursor, _) = buffer
        .cursor_motion(
            &mut font_system,
            Cursor::new(0, 0),
            None,
            Motion::Vertical(1),
        )
        .expect("vertical motion resolves");

    assert_eq!(cursor.line, 2);
}

#[test]
#[should_panic(expected = "line height must be strictly positive")]
fn new_empty_rejects_zero_line_height() {
    let _ = Buffer::new_empty(Metrics::new(14.0, 0.0));
}

#[test]
#[should_panic(expected = "line height must be strictly positive")]
fn new_empty_rejects_nan_line_height() {
    let _ = Buffer::new_empty(Metrics::new(14.0, f32::NAN));
}

#[test]
#[should_panic(expected = "font size must be strictly positive")]
fn new_empty_rejects_zero_font_size() {
    // The old check validated only line height here.
    let _ = Buffer::new_empty(Metrics::new(0.0, 20.0));
}

#[test]
#[should_panic(expected = "line height must be strictly positive")]
fn set_metrics_rejects_negative_line_height() {
    let mut font_system = font_system();
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 20.0));
    buffer.set_metrics(Metrics::new(14.0, -20.0));
}

#[test]
#[should_panic(expected = "font size must be strictly positive")]
fn set_metrics_rejects_infinite_font_size() {
    let mut font_system = font_system();
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 20.0));
    buffer.set_metrics(Metrics::new(f32::INFINITY, 20.0));
}
