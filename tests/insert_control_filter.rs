// What this test guards
// ---------------------
// Action::Insert refuses control characters, with tab and newline as the
// deliberate exceptions. The inherited filter also exempted U+0092 — the
// C1 control "Private Use Two" — which is a Windows-1252 confusion: 0x92
// is the right single quote (U+2019) in cp1252, and U+2019 is not a
// control character, so it never needed an exemption. The typo let an
// invisible C1 control into the text through the one path built to keep
// controls out.

use kalamos::{fontdb, Action, Attrs, Buffer, Cursor, Edit, Editor, FontSystem, Metrics, Shaping};

fn font_system() -> FontSystem {
    FontSystem::new_with_locale_and_db("en-US".into(), fontdb::Database::new())
}

fn editor_with(text: &str) -> Editor<'static> {
    let mut buffer = Buffer::new_empty(Metrics::new(14.0, 20.0));
    buffer.set_text(text, &Attrs::new(), Shaping::Advanced, None);
    Editor::new(buffer)
}

#[test]
fn inserting_u0092_is_refused_like_any_other_control() {
    let mut font_system = font_system();
    let mut editor = editor_with("ab");
    editor.set_cursor(Cursor::new(0, 1));

    editor.action(&mut font_system, Action::Insert('\u{92}'));

    editor.with_buffer(|buffer| {
        assert_eq!(
            buffer.lines[0].text(),
            "ab",
            "a C1 control must not reach the text"
        );
    });
    assert_eq!(
        editor.cursor(),
        Cursor::new(0, 1),
        "a refused insert must not move the cursor"
    );
}

#[test]
fn tab_still_inserts() {
    let mut font_system = font_system();
    let mut editor = editor_with("ab");
    editor.set_cursor(Cursor::new(0, 1));

    editor.action(&mut font_system, Action::Insert('\t'));

    editor.with_buffer(|buffer| assert_eq!(buffer.lines[0].text(), "a\tb"));
}

#[test]
fn ordinary_non_ascii_still_inserts() {
    let mut font_system = font_system();
    let mut editor = editor_with("ab");
    editor.set_cursor(Cursor::new(0, 1));

    editor.action(&mut font_system, Action::Insert('\u{2019}'));

    editor.with_buffer(|buffer| {
        assert_eq!(
            buffer.lines[0].text(),
            "a\u{2019}b",
            "the character the exemption probably meant inserts fine without one"
        );
    });
}
