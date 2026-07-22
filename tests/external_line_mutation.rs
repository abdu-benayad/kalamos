// What this test guards
// ---------------------
// `Buffer::lines` is `pub` and the editor mutates it directly, so lazy
// shaping must notice lines that appear or get invalidated without any
// dirty flag. The old `resolve_dirty` fallback tested
// `needs_reshaping()`, which is true for `Cached::Unused` (previously
// shaped, then reset) but false for `Cached::Empty` (never shaped) —
// so a line pushed or inserted into a clean buffer was never shaped
// and `layout_runs` silently truncated at it. The same predicate also
// counted lines that `shape_until_scroll(prune)` had evicted *itself*,
// so one prune poisoned `redraw()` into returning true forever.
//
// Pinned here: externally added lines shape (append and mid-buffer
// insert), external truncation with a stale scroll recovers, and a
// pruned buffer's redraw settles once nothing changes.

use kalamos::{
    fontdb, Attrs, AttrsList, Buffer, BufferLine, FontSystem, LineEnding, Metrics, Shaping,
};

fn font_system() -> FontSystem {
    let mut font_system =
        FontSystem::new_with_locale_and_db("en-US".into(), fontdb::Database::new());
    font_system
        .db_mut()
        .load_font_data(std::fs::read("fonts/Inter-Regular.ttf").unwrap());
    font_system
}

fn line(text: &str) -> BufferLine {
    BufferLine::new(
        text,
        LineEnding::Lf,
        AttrsList::new(&Attrs::new()),
        Shaping::Advanced,
    )
}

fn run_texts(buffer: &Buffer) -> Vec<String> {
    buffer
        .layout_runs()
        .map(|run| run.text.to_string())
        .collect()
}

#[test]
fn pushed_line_is_shaped() {
    let mut font_system = font_system();
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 20.0));
    buffer.set_text("hello", &Attrs::new(), Shaping::Advanced, None);
    buffer.shape_until_scroll(&mut font_system, false);
    assert_eq!(run_texts(&buffer), ["hello"]);

    // Direct mutation of the public field: no dirty flag is set and the
    // new line starts `Cached::Empty`.
    buffer.lines.push(line("world"));
    buffer.shape_until_scroll(&mut font_system, false);

    assert_eq!(run_texts(&buffer), ["hello", "world"]);
}

#[test]
fn inserted_line_does_not_truncate_the_rest() {
    let mut font_system = font_system();
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 20.0));
    buffer.set_text("aa\nbb\ncc", &Attrs::new(), Shaping::Advanced, None);
    buffer.shape_until_scroll(&mut font_system, false);
    assert_eq!(run_texts(&buffer), ["aa", "bb", "cc"]);

    buffer.lines.insert(1, line("xx"));
    buffer.shape_until_scroll(&mut font_system, false);

    // The old predicate early-returned; the iterator then stopped at the
    // unshaped insert, dropping every line from it onward.
    assert_eq!(run_texts(&buffer), ["aa", "xx", "bb", "cc"]);
}

#[test]
fn external_truncation_recovers_stale_scroll() {
    let mut font_system = font_system();
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 20.0));
    buffer.set_size(None, Some(40.0)); // ~2 lines visible
    buffer.set_text("a\nb\nc\nd\ne\nf", &Attrs::new(), Shaping::Advanced, None);
    buffer.shape_until_scroll(&mut font_system, false);

    let mut scroll = buffer.scroll();
    scroll.line = 4;
    buffer.set_scroll(scroll);
    buffer.shape_until_scroll(&mut font_system, false);

    // Drop every line the scroll pointed at.
    buffer.lines.truncate(1);
    buffer.shape_until_scroll(&mut font_system, false);

    assert_eq!(run_texts(&buffer), ["a"]);
}

#[test]
fn redraw_settles_after_prune() {
    let mut font_system = font_system();
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 20.0));
    buffer.set_size(None, Some(40.0)); // ~2 lines visible
    buffer.set_text(
        "a\nb\nc\nd\ne\nf\ng\nh",
        &Attrs::new(),
        Shaping::Advanced,
        None,
    );

    // Scroll down so pruning evicts lines both before and after the window.
    let mut scroll = buffer.scroll();
    scroll.line = 3;
    buffer.set_scroll(scroll);
    buffer.shape_until_scroll(&mut font_system, true);

    // A stable frame: nothing changed since the last shaping pass.
    buffer.set_redraw(false);
    buffer.shape_until_scroll(&mut font_system, true);

    assert!(
        !buffer.redraw(),
        "prune-evicted lines outside the window must not re-flag redraw"
    );
}
