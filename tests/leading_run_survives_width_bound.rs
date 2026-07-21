use kalamos::{
    fontdb, Attrs, Buffer, Ellipsize, EllipsizeHeightLimit, FontSystem, Metrics, Shaping,
};

/// A digits-first Arabic line: the logically-leading run is the numeral, which
/// under RTL is the *visually rightmost* glyphs on the line.
const DIGITS_FIRST_ARABIC: &str = "١١ يونيو ٢٠٢٦ إلى ١٨ يونيو ٢٠٢٦";

fn font_system() -> FontSystem {
    let mut font_system = FontSystem::new_with_locale_and_db("ar".into(), fontdb::Database::new());
    font_system
        .db_mut()
        .load_font_data(std::fs::read("fonts/NotoSansArabic.ttf").unwrap());
    font_system
        .db_mut()
        .load_font_data(std::fs::read("fonts/Inter-Regular.ttf").unwrap());
    font_system
}

/// Total glyphs laid out, and the lowest source byte any glyph maps to.
///
/// The census is the point. A dropped run is invisible to every geometric
/// assertion, because the reported line width and the glyph extents are *both*
/// derived from the glyphs that survived — they agree with each other perfectly
/// while the text is missing characters. Only counting glyphs against an
/// unconstrained layout of the same string can see it.
fn census(buffer: &Buffer) -> (usize, usize) {
    let glyphs = buffer.layout_runs().map(|run| run.glyphs.len()).sum();
    let first_byte = buffer
        .layout_runs()
        .flat_map(|run| run.glyphs.iter().map(|g| g.start))
        .min()
        .expect("a laid-out line has glyphs");
    (glyphs, first_byte)
}

fn lay_out(font_system: &mut FontSystem, width: Option<f32>, ellipsize: bool) -> Buffer {
    let mut buffer = Buffer::new(font_system, Metrics::new(16.0, 24.0));
    buffer.set_size(width, None);
    if ellipsize {
        buffer.set_ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1)));
    }
    buffer.set_text(DIGITS_FIRST_ARABIC, &Attrs::new(), Shaping::Advanced, None);
    buffer.shape_until_scroll(font_system, false);
    buffer
}

/// **Setting a width bound must not delete the line's leading run.**
///
/// Released cosmic-text 0.19 fails this: with an ellipsize limit and *any* finite
/// width — including a width far wider than the text — the fresh-line sentinel
/// aliases span 0, and the logically-leading run is dropped from layout
/// entirely. On this fixture 0.19 lays out 29 glyphs starting at source byte 4
/// instead of 31 from byte 0: the numeral `١١` is simply gone.
///
/// **What this does and does not guard.** The drop was fixed by upstream's
/// `4fe1195e`, which is in this crate's base — so this test passes with or
/// without our own fresh-line sentinel change, and it is *not* that change's
/// guard. `tests/ellipsize_incongruent.rs` is (verified: reverting the sentinel
/// commit turns it red, and turns this file green). This test exists because the
/// invariant is worth pinning independently of who fixed it, and because it is
/// the tripwire for anyone building against released 0.19 instead of this crate.
///
/// It is deliberately tested at a *comfortable* width. The trigger is the width
/// bound, not the squeeze — so a fixture that only ever tests genuinely
/// truncating cells will never reproduce it, and neither will one that checks
/// geometry instead of counting glyphs. Both of those mistakes were made
/// downstream before this was found.
#[test]
fn a_width_bound_that_fits_keeps_every_glyph() {
    let mut font_system = font_system();

    let reference = lay_out(&mut font_system, None, false);
    let (want_glyphs, want_first_byte) = census(&reference);
    assert!(want_glyphs > 0, "the reference line must produce glyphs");
    assert_eq!(
        want_first_byte, 0,
        "the unconstrained line must lay out from the first source byte"
    );

    let intrinsic = reference
        .layout_runs()
        .map(|run| run.line_w)
        .fold(0.0, f32::max);

    // 40pt of headroom: nothing here needs to be elided away.
    let constrained = lay_out(&mut font_system, Some(intrinsic + 40.0), true);
    let (got_glyphs, got_first_byte) = census(&constrained);

    assert_eq!(
        got_glyphs,
        want_glyphs,
        "constraining a line that already fits dropped {} glyph(s)",
        want_glyphs.saturating_sub(got_glyphs)
    );
    assert_eq!(
        got_first_byte, want_first_byte,
        "the constrained line starts at source byte {got_first_byte}, not \
         {want_first_byte} — the logically-leading run was dropped from layout"
    );
}

/// The same invariant where the line genuinely truncates: an ellipsized line may
/// lose glyphs from its logical *tail*, never from its logical *head*.
#[test]
fn a_truncated_line_loses_its_tail_not_its_head() {
    let mut font_system = font_system();

    let reference = lay_out(&mut font_system, None, false);
    let (full_glyphs, _) = census(&reference);
    let intrinsic = reference
        .layout_runs()
        .map(|run| run.line_w)
        .fold(0.0, f32::max);

    for factor in [0.7_f32, 0.5, 0.3] {
        let width = intrinsic * factor;
        let constrained = lay_out(&mut font_system, Some(width), true);
        let (glyphs, first_byte) = census(&constrained);

        assert!(
            glyphs < full_glyphs,
            "at {factor} of the intrinsic width the line must actually truncate \
             ({glyphs} vs {full_glyphs})"
        );
        assert_eq!(
            first_byte, 0,
            "end-ellipsization at {factor} of the intrinsic width must trim the \
             logical tail; starting at byte {first_byte} means the head was dropped"
        );
    }
}
