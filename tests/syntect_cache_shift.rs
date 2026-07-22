#![cfg(feature = "syntect")]
// What this test guards
// ---------------------
// SyntaxEditor keeps one syntax_cache entry per line INDEX, and stamps each
// BufferLine's metadata with its index at highlight time. Deleting a line
// shifts every later line up while the cache stays put; the reparse chain
// stops at the first index whose new parse state equals the OLD occupant's
// cached state, which strands stale entries at the shifted positions.
// Nothing looks wrong yet — shifted lines carry their attrs with them.
//
// The damage lands on the NEXT edit: a reparsed line seeds its parse state
// from syntax_cache[line_i - 1] — new coordinates, old contents. Typing
// inside a /* block comment */ whose opener shifted made the edited line
// parse as code and lose its comment color.
//
// The skip check now requires metadata() == Some(line_i), so a shifted
// line's stamp no longer matches and one realigning reparse restores the
// invariant before any edit can seed from a stale entry.

use kalamos::{
    fontdb, Action, Attrs, Buffer, Color, Cursor, Edit, FontSystem, Metrics, Shaping, SyntaxEditor,
    SyntaxSystem,
};

fn line_color(editor: &SyntaxEditor, line_i: usize, byte_i: usize) -> Option<Color> {
    editor.with_buffer(|buffer| buffer.lines[line_i].attrs_list().get_span(byte_i).color_opt)
}

#[test]
fn comment_body_keeps_its_color_after_a_shift_and_an_edit() {
    let mut font_system = FontSystem::new_with_locale_and_db("en-US".into(), {
        let mut db = fontdb::Database::new();
        db.load_font_data(std::fs::read("fonts/FiraMono-Medium.ttf").unwrap());
        db
    });
    let syntax_system = SyntaxSystem::new();

    let mut buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 20.0));
    buffer.set_size(Some(1000.0), Some(1000.0));
    let mut editor = SyntaxEditor::new(buffer, &syntax_system, "base16-eighties.dark")
        .expect("the default theme set contains base16-eighties.dark");
    editor.syntax_by_extension("rs");

    editor.with_buffer_mut(|buffer| {
        buffer.set_text(
            "let p = 0;\nlet q = 0;\n/*\ninside\n*/\n",
            &Attrs::new(),
            Shaping::Advanced,
            None,
        );
    });
    editor.shape_as_needed(&mut font_system, false);

    // Ground truth from the initial full highlight: the comment body's color
    // differs from plain code — otherwise the pin cannot bite.
    let comment_color = line_color(&editor, 3, 0);
    let code_color = line_color(&editor, 1, 0);
    assert_ne!(
        comment_color, code_color,
        "precondition: the theme colors comment bodies differently from code"
    );

    // Step 1 — shift: delete the whole first line. "let q = 0;" lands on
    // index 0 and parses to the same state the deleted line had, so the
    // chain stops immediately and every later cache entry is stale by one.
    editor.delete_range(Cursor::new(0, 0), Cursor::new(1, 0));
    editor.shape_as_needed(&mut font_system, false);

    // Step 2 — edit inside the comment body (now line 2). The reparse seeds
    // from syntax_cache[1], which describes the old "let q = 0;" (outside
    // any comment) instead of the shifted "/*" opener.
    editor.set_cursor(Cursor::new(2, 0));
    editor.action(&mut font_system, Action::Insert('x'));
    editor.shape_as_needed(&mut font_system, false);

    editor.with_buffer(|buffer| {
        assert_eq!(buffer.lines[2].text(), "xinside", "the edit landed");
    });
    assert_eq!(
        line_color(&editor, 2, 0),
        comment_color,
        "the comment body must keep its comment color after the shift + edit"
    );
}
