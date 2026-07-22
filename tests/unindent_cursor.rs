// What this test guards
// ---------------------
// `Action::Unindent` deletes the whitespace run `[last_indent,
// after_whitespace)` and then shifts the cursor and selection endpoint.
// The old adjustment subtracted the full deleted width from any index
// greater than `last_indent` — including an index *inside* the deleted
// run, which underflowed `usize` (panic in debug, a wrapped index that
// detonates at the next slice in release). Reproduced before fixing:
// 8-space indent, caret at index 3, Shift+Tab.
//
// The contract pinned here: an index at or past the deleted run shifts
// back by the run's width; an index inside the run clamps to the
// deletion start; an index at or before `last_indent` is untouched.

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
fn cursor_inside_deleted_whitespace_clamps_to_deletion_start() {
    let mut font_system = font_system();
    // One full 8-column indent; tab_width defaults to 8, so the whole
    // run [0, 8) is deleted.
    let mut editor = editor_with("        y");
    editor.set_cursor(Cursor::new(0, 3));

    editor.action(&mut font_system, Action::Unindent);

    editor.with_buffer(|buffer| assert_eq!(buffer.lines[0].text(), "y"));
    assert_eq!(editor.cursor(), Cursor::new(0, 0));
}

#[test]
fn cursor_past_deleted_whitespace_shifts_by_deleted_width() {
    let mut font_system = font_system();
    let mut editor = editor_with("        y");
    // Caret after `y` (index 9).
    editor.set_cursor(Cursor::new(0, 9));

    editor.action(&mut font_system, Action::Unindent);

    editor.with_buffer(|buffer| assert_eq!(buffer.lines[0].text(), "y"));
    assert_eq!(editor.cursor(), Cursor::new(0, 1));
}

#[test]
fn cursor_at_deletion_start_is_untouched() {
    let mut font_system = font_system();
    let mut editor = editor_with("        y");
    editor.set_cursor(Cursor::new(0, 0));

    editor.action(&mut font_system, Action::Unindent);

    assert_eq!(editor.cursor(), Cursor::new(0, 0));
}

#[test]
fn selection_endpoint_inside_deleted_whitespace_clamps() {
    let mut font_system = font_system();
    let mut editor = editor_with("        y");
    // Anchor inside the doomed whitespace, caret on `y`.
    editor.set_selection(Selection::Normal(Cursor::new(0, 5)));
    editor.set_cursor(Cursor::new(0, 8));

    editor.action(&mut font_system, Action::Unindent);

    match editor.selection() {
        Selection::Normal(anchor) => assert_eq!(anchor, Cursor::new(0, 0)),
        other => panic!("selection changed variant: {other:?}"),
    }
    assert_eq!(editor.cursor(), Cursor::new(0, 0));
}

#[test]
fn partial_indent_deletes_back_to_previous_stop() {
    let mut font_system = font_system();
    // Six spaces under tab_width 4: stops at 0 and 4, so [4, 6) is
    // deleted. Caret at 5 sits inside the deleted run → clamps to 4.
    let mut editor = editor_with("      y");
    editor.with_buffer_mut(|buffer| buffer.set_tab_width(4));
    editor.set_cursor(Cursor::new(0, 5));

    editor.action(&mut font_system, Action::Unindent);

    editor.with_buffer(|buffer| assert_eq!(buffer.lines[0].text(), "    y"));
    assert_eq!(editor.cursor(), Cursor::new(0, 4));
}
