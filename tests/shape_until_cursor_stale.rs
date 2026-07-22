// What this test guards
// ---------------------
// `shape_until_cursor` used to `.expect(...)` on its first
// `layout_cursor` call, which returns `None` exactly when `cursor.line`
// is out of bounds — reachable whenever text shrinks under a held
// cursor: `set_text` with fewer lines, or external truncation of the
// pub `lines` field. The panic contradicted the function's own second
// half, which already tolerates `None` from the same call when
// adjusting horizontal scroll.
//
// The contract now: an unresolvable cursor means there is nothing to
// scroll to — the scroll window is still shaped, the cursor adjustment
// is skipped, and the buffer stays usable. `valid_cursor_still_scrolls`
// is the fence that the graceful path did not eat the function's actual
// job.

use kalamos::{fontdb, Attrs, Buffer, Cursor, FontSystem, Metrics, Shaping, Wrap};

fn font_system() -> FontSystem {
    let mut font_system =
        FontSystem::new_with_locale_and_db("en-US".into(), fontdb::Database::new());
    font_system
        .db_mut()
        .load_font_data(std::fs::read("fonts/Inter-Regular.ttf").unwrap());
    font_system
}

#[test]
fn stale_cursor_after_text_shrinks_does_not_panic() {
    let mut font_system = font_system();
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 20.0));
    buffer.set_wrap(Wrap::None);
    buffer.set_text("one\ntwo\nthree", &Attrs::new(), Shaping::Advanced, None);
    buffer.shape_until_scroll(&mut font_system, false);
    let cursor = Cursor::new(2, 3);

    buffer.set_text("one", &Attrs::new(), Shaping::Advanced, None);
    buffer.shape_until_cursor(&mut font_system, cursor, false);

    // The window itself was still shaped and the buffer is usable.
    assert_eq!(buffer.layout_runs().count(), 1);
    assert_eq!(buffer.scroll().line, 0);
}

#[test]
fn stale_cursor_after_external_truncation_does_not_panic() {
    let mut font_system = font_system();
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 20.0));
    buffer.set_wrap(Wrap::None);
    buffer.set_text("one\ntwo\nthree", &Attrs::new(), Shaping::Advanced, None);
    buffer.shape_until_scroll(&mut font_system, false);
    let cursor = Cursor::new(2, 3);

    // `lines` is pub and directly mutable; no dirty flag is set.
    buffer.lines.truncate(1);
    buffer.shape_until_cursor(&mut font_system, cursor, false);

    assert_eq!(buffer.layout_runs().count(), 1);
}

#[test]
fn valid_cursor_still_scrolls() {
    let mut font_system = font_system();
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 20.0));
    buffer.set_wrap(Wrap::None);
    // 40 lines, window two lines tall: scrolling to the last line must
    // move the window.
    let text = (0..40).map(|i| format!("line {i}")).collect::<Vec<_>>();
    buffer.set_text(&text.join("\n"), &Attrs::new(), Shaping::Advanced, None);
    buffer.set_size(None, Some(40.0));

    buffer.shape_until_cursor(&mut font_system, Cursor::new(39, 0), false);

    assert!(
        buffer.scroll().line > 30,
        "scroll followed the cursor to the last line, got line {}",
        buffer.scroll().line
    );
}
