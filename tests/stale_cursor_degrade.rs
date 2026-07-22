// What this test guards
// ---------------------
// Editor cursors are unvalidated: `set_cursor` accepts any Cursor, and
// `with_buffer_mut` lets a caller rewrite the text under a live cursor —
// so a cursor whose byte index is past the end of its line, or inside a
// multi-byte character, is constructible through the public API alone.
// The cursor→text slices in editor.rs used to panic on such a cursor
// ("byte index N is out of bounds" / "not a char boundary").
//
// The contract pinned here: a malformed cursor DEGRADES instead of
// panicking. copy_selection skips the unreachable segment (contributing
// an empty string, keeping every segment a valid cursor still reaches);
// Backspace becomes a no-op. The degrade logs a warning; it must never
// take down the caller.
//
// These pins were watched red against the panicking slices before the
// conversion to .get().

use kalamos::{
    fontdb, Action, Attrs, Buffer, Cursor, Edit, Editor, FontSystem, Metrics, Selection, Shaping,
};

fn font_system() -> FontSystem {
    FontSystem::new_with_locale_and_db("en-US".into(), fontdb::Database::new())
}

fn editor_with(text: &str) -> Editor<'static> {
    let mut buffer = Buffer::new_empty(Metrics::new(14.0, 20.0));
    buffer.set_text(text, &Attrs::new(), Shaping::Advanced, None);
    Editor::new(buffer)
}

#[test]
fn copy_selection_with_an_out_of_bounds_end_index_degrades_to_empty() {
    let mut editor = editor_with("abc");
    editor.set_selection(Selection::Normal(Cursor::new(0, 0)));
    editor.set_cursor(Cursor::new(0, 99));

    assert_eq!(editor.copy_selection(), Some(String::new()));
}

#[test]
fn copy_selection_with_a_mid_char_index_degrades_to_empty() {
    // "éé": char boundaries at bytes 0, 2, 4 — byte 1 is inside the first é.
    let mut editor = editor_with("éé");
    editor.set_selection(Selection::Normal(Cursor::new(0, 0)));
    editor.set_cursor(Cursor::new(0, 1));

    assert_eq!(editor.copy_selection(), Some(String::new()));
}

#[test]
fn copy_selection_with_an_out_of_bounds_start_keeps_the_later_lines() {
    let mut editor = editor_with("ab\ncd");
    editor.set_cursor(Cursor::new(0, 99));
    editor.set_selection(Selection::Normal(Cursor::new(1, 1)));

    // The first line's segment is unreachable and skipped; its line break
    // and the last line's valid prefix survive.
    assert_eq!(editor.copy_selection(), Some("\nc".into()));
}

#[test]
fn copy_selection_with_an_out_of_bounds_end_keeps_the_earlier_lines() {
    let mut editor = editor_with("ab\ncd");
    editor.set_cursor(Cursor::new(0, 0));
    editor.set_selection(Selection::Normal(Cursor::new(1, 99)));

    assert_eq!(editor.copy_selection(), Some("ab\n".into()));
}

#[test]
fn indent_with_an_out_of_bounds_cursor_leaves_the_line_unchanged() {
    let mut font_system = font_system();
    let mut editor = editor_with("abc");
    editor.set_cursor(Cursor::new(0, 99));

    editor.action(&mut font_system, Action::Indent);

    let text = editor.with_buffer(|buffer| buffer.lines[0].text().to_string());
    assert_eq!(text, "abc");
}

#[test]
fn backspace_with_an_out_of_bounds_cursor_is_a_no_op() {
    let mut font_system = font_system();
    let mut editor = editor_with("abc");
    editor.set_cursor(Cursor::new(0, 99));

    editor.action(&mut font_system, Action::Backspace);

    let text = editor.with_buffer(|buffer| buffer.lines[0].text().to_string());
    assert_eq!(text, "abc");
}
