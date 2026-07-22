use kalamos::{fontdb, Align, Attrs, Buffer, FontSystem, Metrics, Shaping, Wrap};

fn font_system() -> FontSystem {
    let mut font_system =
        FontSystem::new_with_locale_and_db("en-US".into(), fontdb::Database::new());
    font_system
        .db_mut()
        .load_font_data(std::fs::read("fonts/Inter-Regular.ttf").unwrap());
    font_system
}

fn arabic_font_system() -> FontSystem {
    let mut font_system = FontSystem::new_with_locale_and_db("ar".into(), fontdb::Database::new());
    font_system
        .db_mut()
        .load_font_data(std::fs::read("fonts/NotoSansArabic.ttf").unwrap());
    font_system
}

fn rtl_buffer(font_system: &mut FontSystem, align: Option<Align>) -> Buffer {
    let mut buffer = Buffer::new(font_system, Metrics::new(14.0, 20.0));
    buffer.set_size(Some(400.0), None);
    buffer.set_wrap(Wrap::None);
    buffer.set_text("سلام", &Attrs::new(), Shaping::Advanced, None);
    buffer.lines[0].set_align(align);
    buffer.shape_until_scroll(font_system, false);
    buffer
}

#[test]
fn click_before_a_right_aligned_line_hits_its_start() {
    // A short right-aligned line starts at glyphs[0].x > 0. A click in the
    // leading gap (left of the text, but x >= 0) is "before the line" and must
    // place the cursor at the line START. The old test was hardcoded `x < 0.0`,
    // so such clicks fell through to the past-all-glyphs arm and landed at the
    // line END instead.
    let mut font_system = font_system();
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 20.0));
    buffer.set_size(Some(400.0), None);
    buffer.set_wrap(Wrap::None);
    buffer.set_text("hi", &Attrs::new(), Shaping::Advanced, None);
    buffer.lines[0].set_align(Some(Align::Right));
    buffer.shape_until_scroll(&mut font_system, false);

    let run = buffer
        .layout_runs()
        .next()
        .expect("expected at least one layout run");
    let first_glyph_x = run.glyphs.first().expect("expected glyphs").x;
    assert!(
        first_glyph_x > 10.0,
        "the right-aligned line must start well right of x=0 (got {first_glyph_x})"
    );

    let cursor = buffer
        .hit(first_glyph_x / 2.0, 10.0)
        .expect("expected a hit");
    assert_eq!(
        cursor.index, 0,
        "a click in the leading gap of a right-aligned line must hit the line start"
    );
}

#[test]
fn click_before_a_flush_left_line_still_hits_its_start() {
    // Guard for the flush-left case the old `x < 0.0` did handle: a click at
    // negative x still resolves to the line start.
    let mut font_system = font_system();
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 20.0));
    buffer.set_size(Some(400.0), None);
    buffer.set_wrap(Wrap::None);
    buffer.set_text("hi", &Attrs::new(), Shaping::Advanced, None);
    buffer.shape_until_scroll(&mut font_system, false);

    let cursor = buffer.hit(-5.0, 10.0).expect("expected a hit");
    assert_eq!(cursor.index, 0);
}

// The three tests below are fences, not fix-proofs: the RTL before-line
// check used to fire for any x right of glyphs[0].x — including inside
// glyph[0]'s own box — and was rescued by the containment arm. Tightening
// it to the exact complement (x > glyphs[0].x + glyphs[0].w) must not
// change behavior anywhere; these pin that.

#[test]
fn click_in_the_leading_gap_of_a_left_aligned_rtl_line_hits_its_start() {
    // An RTL line forced Align::Left hugs x=0, leaving the leading gap on
    // the RIGHT (RTL reads right-to-left, so "before the line" is right of
    // the rightmost glyph). A click in that gap must land at the logical
    // line START.
    let mut font_system = arabic_font_system();
    let buffer = rtl_buffer(&mut font_system, Some(Align::Left));

    let run = buffer.layout_runs().next().expect("one layout run");
    assert!(run.rtl, "precondition: the line is RTL");
    let first = run.glyphs.first().expect("glyphs present");
    let right_edge = first.x + first.w;
    assert!(
        right_edge < 390.0,
        "precondition: a leading gap exists right of the text (edge {right_edge})"
    );

    let cursor = buffer
        .hit((right_edge + 400.0) / 2.0, 10.0)
        .expect("expected a hit");
    assert_eq!(
        cursor.index, 0,
        "a click right of a left-aligned RTL line is before the line: logical start"
    );
}

#[test]
fn click_in_the_trailing_gap_of_a_right_aligned_rtl_line_hits_its_end() {
    // Same geometry, opposite side: past the LEFTMOST glyph is past the end
    // of an RTL line. Only x beyond the line's visual left edge may reach
    // the past-all-glyphs arm.
    let mut font_system = arabic_font_system();
    let buffer = rtl_buffer(&mut font_system, Some(Align::Right));

    let run = buffer.layout_runs().next().expect("one layout run");
    assert!(run.rtl, "precondition: the line is RTL");
    let left_edge = run.glyphs.iter().map(|g| g.x).fold(f32::INFINITY, f32::min);
    assert!(
        left_edge > 10.0,
        "precondition: a trailing gap exists left of the text (edge {left_edge})"
    );
    let text_len = run.text.len();

    let cursor = buffer.hit(left_edge / 2.0, 10.0).expect("expected a hit");
    assert_eq!(
        cursor.index, text_len,
        "a click left of a right-aligned RTL line is past the line: logical end"
    );
}

#[test]
fn click_inside_the_first_rtl_glyph_resolves_by_containment_not_the_gap_arm() {
    // x inside glyphs[0]'s own box: the old over-broad check tentatively
    // claimed this as "before the line" and relied on the containment arm
    // to override it. Clicking the LEFT half of the rightmost glyph must
    // advance past its cluster (index > 0) — the before-line default
    // (index 0, After) must not leak through.
    let mut font_system = arabic_font_system();
    let buffer = rtl_buffer(&mut font_system, Some(Align::Left));

    let run = buffer.layout_runs().next().expect("one layout run");
    assert!(run.rtl, "precondition: the line is RTL");
    let first = run.glyphs.first().expect("glyphs present");
    assert!(
        first.w > 1.0,
        "precondition: glyph box wide enough to halve"
    );

    let cursor = buffer
        .hit(first.x + first.w * 0.25, 10.0)
        .expect("expected a hit");
    assert!(
        cursor.index > 0,
        "the left half of the rightmost RTL glyph is past its cluster, got index {}",
        cursor.index
    );
}
