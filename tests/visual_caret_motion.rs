// What this test guards
// ---------------------
// Horizontal arrow motion must move the caret in VISUAL order — one caret
// stop leftward/rightward on screen per keypress — not in logical order
// flipped by the line's base direction.
//
// Before the fix, `Motion::Left`/`Right` chose a direction from the whole
// line's base (`line_shape(..).rtl`) and then stepped LOGICALLY
// (`Motion::Next`/`Previous`). On a mixed-direction line that is wrong the
// moment the caret enters a run whose direction differs from the base: the
// caret stops following the run it is crossing into. In `abcسلام` (LTR base,
// Arabic run to the right) pressing Right climbs through `abc`, jumps to the
// far right of the Arabic run, then walks BACK left through it — the caret x
// goes up, then down. That non-monotonic x is the bug.
//
// The pinned contract: walking Right from the visual-left stop, the caret x
// is non-decreasing at every step (equal is legal where two stops share an x
// at a seam). The tell that separates a correct bidi walk from a broken one:
// the visual-RIGHT end of `abcسلام` is `(3, After)` — the leading (right)
// edge of the first Arabic letter — NOT the logical end `(len, Before)`,
// which sits visually mid-line. Watched red (x decreases mid-walk) before the
// fix; the walk lands one step short of the seam because logical stepping
// never reaches the visual end.

use kalamos::{
    fontdb, Affinity, Attrs, Buffer, Cursor, Direction, FontSystem, Metrics, Motion, Shaping, Wrap,
};

fn font_system() -> FontSystem {
    let mut font_system =
        FontSystem::new_with_locale_and_db("en-US".into(), fontdb::Database::new());
    font_system
        .db_mut()
        .load_font_data(std::fs::read("fonts/Inter-Regular.ttf").unwrap());
    font_system
        .db_mut()
        .load_font_data(std::fs::read("fonts/NotoSansArabic.ttf").unwrap());
    font_system
}

fn buffer_of(font_system: &mut FontSystem, text: &str) -> Buffer {
    let mut buffer = Buffer::new(font_system, Metrics::new(14.0, 20.0));
    buffer.set_wrap(Wrap::None);
    buffer.set_direction(Direction::Auto);
    buffer.set_text(text, &Attrs::new(), Shaping::Advanced, None);
    buffer.shape_until_scroll(font_system, false);
    buffer
}

/// Number of clusters on the (single) visual line = the number of caret stops
/// minus one = the number of Right steps from the visual-left to the
/// visual-right stop. Distinct `[start, end)` ranges among the run's glyphs.
fn cluster_count(buffer: &Buffer) -> usize {
    let run = buffer.layout_runs().next().expect("one visual line");
    let mut ranges: Vec<(usize, usize)> = run.glyphs.iter().map(|g| (g.start, g.end)).collect();
    ranges.sort_unstable();
    ranges.dedup();
    ranges.len()
}

fn caret_x(buffer: &Buffer, cursor: &Cursor) -> f32 {
    buffer.cursor_position(cursor).expect("cursor resolves").0
}

#[test]
fn right_walks_the_caret_monotonically_rightward_across_a_bidi_seam() {
    let mut font_system = font_system();
    let mut buffer = buffer_of(&mut font_system, "abcسلام");
    let steps = cluster_count(&buffer);
    assert!(steps >= 4, "expected several clusters, got {steps}");

    // Visual-left stop of an LTR-base line: the leading edge of the first letter.
    let mut cursor = Cursor::new_with_affinity(0, 0, Affinity::After);
    let mut cursor_x_opt = None;
    let mut x_prev = caret_x(&buffer, &cursor);

    for step in 0..steps {
        (cursor, cursor_x_opt) = buffer
            .cursor_motion(&mut font_system, cursor, cursor_x_opt, Motion::Right)
            .expect("right resolves");
        let x = caret_x(&buffer, &cursor);
        assert!(
            x >= x_prev - 0.01,
            "step {step}: caret jumped left (x {x} < prev {x_prev}) — motion is not visual-order"
        );
        x_prev = x;
    }

    assert_eq!(
        (cursor.line, cursor.index, cursor.affinity),
        (0, 3, Affinity::After),
        "the visual-right end of `abcسلام` is (3, After), the leading edge of the first Arabic letter — not the logical end"
    );
}

#[test]
fn left_walks_the_caret_monotonically_leftward_across_a_bidi_seam() {
    let mut font_system = font_system();
    let mut buffer = buffer_of(&mut font_system, "abcسلام");
    let steps = cluster_count(&buffer);

    // Start at the visual-right stop and walk back to the visual-left one.
    let mut cursor = Cursor::new_with_affinity(0, 3, Affinity::After);
    let mut cursor_x_opt = None;
    let mut x_prev = caret_x(&buffer, &cursor);
    // The first Left steps from the visual-right end must cross INTO the Arabic
    // run (byte index > 3), not jump straight to `abc`. Logical stepping skips
    // the run entirely, so this is the red-on-old check.
    let mut entered_arabic_run = false;

    for step in 0..steps {
        (cursor, cursor_x_opt) = buffer
            .cursor_motion(&mut font_system, cursor, cursor_x_opt, Motion::Left)
            .expect("left resolves");
        entered_arabic_run |= cursor.index > 3;
        let x = caret_x(&buffer, &cursor);
        assert!(
            x <= x_prev + 0.01,
            "step {step}: caret jumped right (x {x} > prev {x_prev}) — motion is not visual-order"
        );
        x_prev = x;
    }

    assert!(
        entered_arabic_run,
        "walking Left from the visual-right end must traverse the Arabic run's caret stops, not skip to `abc`"
    );
    assert_eq!(
        (cursor.line, cursor.index, cursor.affinity),
        (0, 0, Affinity::After),
        "walking Left across the whole line lands at the visual-left stop"
    );
}

#[test]
fn ltr_right_walk_stays_monotonic() {
    // Fence: a pure-LTR line must remain correct (Right = increasing x, ending
    // at the logical end).
    let mut font_system = font_system();
    let mut buffer = buffer_of(&mut font_system, "abcdef");
    let steps = cluster_count(&buffer);

    let mut cursor = Cursor::new_with_affinity(0, 0, Affinity::After);
    let mut cursor_x_opt = None;
    let mut x_prev = caret_x(&buffer, &cursor);
    for step in 0..steps {
        (cursor, cursor_x_opt) = buffer
            .cursor_motion(&mut font_system, cursor, cursor_x_opt, Motion::Right)
            .expect("right resolves");
        let x = caret_x(&buffer, &cursor);
        assert!(x >= x_prev - 0.01, "LTR step {step}: x {x} < prev {x_prev}");
        x_prev = x;
    }
    assert_eq!(
        cursor.index, 6,
        "LTR Right ends at the logical (and visual) end"
    );
    let _ = cursor_x_opt;
}

#[test]
fn rtl_right_walk_stays_monotonic() {
    // Fence: a pure-RTL line — Right still means increasing visual x (which is
    // logical backward here). Monotonic non-decreasing x must hold.
    let mut font_system = font_system();
    let mut buffer = buffer_of(&mut font_system, "سلام");
    let steps = cluster_count(&buffer);

    // Visual-left stop of an RTL-base line is its logical END.
    let logical_end = buffer
        .layout_runs()
        .next()
        .unwrap()
        .glyphs
        .iter()
        .map(|g| g.end)
        .max()
        .unwrap();
    let mut cursor = Cursor::new_with_affinity(0, logical_end, Affinity::Before);
    let mut cursor_x_opt = None;
    let mut x_prev = caret_x(&buffer, &cursor);
    for step in 0..steps {
        (cursor, cursor_x_opt) = buffer
            .cursor_motion(&mut font_system, cursor, cursor_x_opt, Motion::Right)
            .expect("right resolves");
        let x = caret_x(&buffer, &cursor);
        assert!(x >= x_prev - 0.01, "RTL step {step}: x {x} < prev {x_prev}");
        x_prev = x;
    }
    // Do not assert the exact end byte: a lam-alef ligature makes the
    // cluster/grapheme counts differ, and the fence's job is the monotonicity
    // invariant, not an endpoint. `steps` moves made progress rightward.
    assert!(
        x_prev
            > caret_x(
                &buffer,
                &Cursor::new_with_affinity(0, logical_end, Affinity::Before)
            )
    );
}

/// **The RTL-base counterpart to the seam test above, and the case the other four miss.**
///
/// The fences above cover an LTR base with an embedded Arabic run, a pure-LTR line and a
/// pure-RTL line. None covers an **RTL base with an embedded LTR run** — `المبلغ 250`, an
/// Arabic field holding digits, which is the ordinary kiosk string. Two defects lived in
/// that gap, and both are visible only here:
///
/// 1. At `(0, Before)` the visual step reported "line edge" and returned `None`, because
///    the no-upstream arm of `visual_cluster_step` had its RTL and LTR cases inverted. The
///    logical fallback answered instead and happened to give the right byte, so nothing
///    looked wrong — the visual walk was not being taken at all for the first step.
/// 2. At the visual-LEFT stop — which on this line is *mid-text*, inside the digit run, not
///    the logical end — the edge fallback stepped logically by the line's **base**
///    direction. That walked the caret back into the middle of the line, visually
///    rightward; the next `Left` walked it back onto the edge; and so on forever. The
///    caret oscillated between two stops under a held arrow key.
///
/// So this walks further than there are clusters: past the visual edge, where the caret must
/// simply stop. `x` never increasing is what fails on (2); reaching x ≈ 0 at all is what
/// fails on (1).
#[test]
fn left_walk_on_an_rtl_base_line_ends_at_the_visual_edge_and_stays() {
    let mut font_system = font_system();
    let mut buffer = buffer_of(&mut font_system, "المبلغ 250");
    // `buffer_of` sets `Direction::Auto`; the leading Arabic already resolves the base to
    // RTL, which is the configuration under test.
    let run = buffer.layout_runs().next().expect("one visual line");
    assert!(run.rtl, "the line under test must have an RTL base");
    let line_left = run.glyphs.iter().map(|g| g.x).fold(f32::INFINITY, f32::min);

    // Deliberately more steps than clusters, so the walk runs past the visual edge.
    let steps = cluster_count(&buffer) + 3;
    let mut cursor = Cursor::new(0, 0);
    let mut cursor_x_opt = None;
    let mut x_prev = caret_x(&buffer, &cursor);
    for step in 0..steps {
        (cursor, cursor_x_opt) = buffer
            .cursor_motion(&mut font_system, cursor, cursor_x_opt, Motion::Left)
            .expect("left resolves");
        let x = caret_x(&buffer, &cursor);
        assert!(
            x <= x_prev + 0.01,
            "step {step}: caret jumped right (x {x} > prev {x_prev}) — the edge fallback \
             stepped by the line's base direction instead of stopping"
        );
        x_prev = x;
    }
    assert!(
        (x_prev - line_left).abs() < 0.01,
        "the walk ended at x {x_prev}, not the line's visual-left edge {line_left} — the \
         first step never took the visual path"
    );
}

/// The `Right` direction of the same line: from the visual-left stop back to the
/// visual-right one, then terminal. Without it, a fix that made `Left` terminal by
/// disabling the fallback in *both* directions would still pass the test above.
#[test]
fn right_walk_on_an_rtl_base_line_ends_at_the_visual_edge_and_stays() {
    let mut font_system = font_system();
    let mut buffer = buffer_of(&mut font_system, "المبلغ 250");
    let run = buffer.layout_runs().next().expect("one visual line");
    let line_right = run
        .glyphs
        .iter()
        .map(|g| g.x + g.w)
        .fold(f32::NEG_INFINITY, f32::max);

    // Walk all the way left first, so the start is the visual-left stop whatever its bytes.
    let steps = cluster_count(&buffer) + 3;
    let mut cursor = Cursor::new(0, 0);
    let mut cursor_x_opt = None;
    for _ in 0..steps {
        (cursor, cursor_x_opt) = buffer
            .cursor_motion(&mut font_system, cursor, cursor_x_opt, Motion::Left)
            .expect("left resolves");
    }

    let mut x_prev = caret_x(&buffer, &cursor);
    for step in 0..steps {
        (cursor, cursor_x_opt) = buffer
            .cursor_motion(&mut font_system, cursor, cursor_x_opt, Motion::Right)
            .expect("right resolves");
        let x = caret_x(&buffer, &cursor);
        assert!(
            x >= x_prev - 0.01,
            "step {step}: caret jumped left (x {x} < prev {x_prev})"
        );
        x_prev = x;
    }
    assert!(
        (x_prev - line_right).abs() < 0.01,
        "the walk ended at x {x_prev}, not the line's visual-right edge {line_right}"
    );
}
