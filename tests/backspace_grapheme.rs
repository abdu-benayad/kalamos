// What this test guards
// ---------------------
// Backspace deletes one scalar backwards; Delete deletes one grapheme
// forwards. The asymmetry is DELIBERATE: scalar-wise backward deletion
// is the established Arabic-input convention — the last harakah can be
// corrected without killing its base letter — and this library is
// RTL-first. `backspace_removes_diacritic_only` and
// `delete_removes_whole_cluster` are the fences that keep that
// convention from being "fixed" into symmetry.
//
// The genuine defect was stranding: deleting one scalar can fuse the
// surrounding text into a new cluster, leaving the caret mid-grapheme
// at a position no motion can reach. Two flag emoji are the crisp
// repro: "🇸🇦🇪🇬" with the caret between the flags loses one regional
// indicator to Backspace, the remainder refuses into "🇸🇪" + "🇬", and
// the caret used to sit at byte 4 — inside the Sweden flag. The caret
// now snaps to the largest grapheme boundary at or before its position
// in the resulting text (a no-op whenever it is already on a boundary).

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
fn backspace_removes_diacritic_only() {
    let mut font_system = font_system();
    // BEH (2 bytes) + FATHA (2 bytes): one grapheme, two scalars.
    let mut editor = editor_with("\u{0628}\u{064E}");
    editor.set_cursor(Cursor::new(0, 4));

    editor.action(&mut font_system, Action::Backspace);

    editor.with_buffer(|buffer| assert_eq!(buffer.lines[0].text(), "\u{0628}"));
    assert_eq!(editor.cursor(), Cursor::new(0, 2));
}

#[test]
fn delete_removes_whole_cluster() {
    let mut font_system = font_system();
    let mut editor = editor_with("\u{0628}\u{064E}x");
    editor.set_cursor(Cursor::new(0, 0));

    editor.action(&mut font_system, Action::Delete);

    editor.with_buffer(|buffer| assert_eq!(buffer.lines[0].text(), "x"));
    assert_eq!(editor.cursor(), Cursor::new(0, 0));
}

#[test]
fn backspace_between_flags_does_not_strand_the_caret() {
    let mut font_system = font_system();
    // SA flag (RI-S RI-A) + EG flag (RI-E RI-G): each regional
    // indicator is 4 bytes, each flag one grapheme.
    let mut editor = editor_with("\u{1F1F8}\u{1F1E6}\u{1F1EA}\u{1F1EC}");
    editor.set_cursor(Cursor::new(0, 8));

    editor.action(&mut font_system, Action::Backspace);

    // RI-A is gone; the remainder refuses into the SE flag + RI-G.
    editor.with_buffer(|buffer| {
        assert_eq!(buffer.lines[0].text(), "\u{1F1F8}\u{1F1EA}\u{1F1EC}");
    });
    // Byte 4 is inside the fused SE flag; the only boundary at or
    // before it is 0.
    assert_eq!(
        editor.cursor(),
        Cursor::new(0, 0),
        "the caret must land on a grapheme boundary of the resulting text"
    );
}

#[test]
fn backspace_ascii_unchanged() {
    let mut font_system = font_system();
    let mut editor = editor_with("ab");
    editor.set_cursor(Cursor::new(0, 2));

    editor.action(&mut font_system, Action::Backspace);

    editor.with_buffer(|buffer| assert_eq!(buffer.lines[0].text(), "a"));
    assert_eq!(editor.cursor(), Cursor::new(0, 1));
}
