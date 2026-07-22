// What this test guards
// ---------------------
// When Wrap::Glyph splits a word inside an incongruent span (RTL content on a
// forced-LTR base), the committed line ends at a resume point with glyph > 0,
// and try_ellipsize_last_line feeds that resume point into layout_spans.
//
// Forward (Ellipsize::End): the incongruent-resume word_indices arm excluded
// the partial resumed word, so the glyphs 0..start.glyph entered the visual
// line's RANGE but never its WIDTH. The overflow check then believed more
// content fit than did, and the "ellipsized" last line painted far past the
// buffer width — at width 50 the probe measured a 73.79 extent with nothing
// elided at all.
//
// Backward (Ellipsize::Start): remaining_content_exceeds counted words past
// the resume boundary — words already committed to the PREVIOUS line — as
// still-pending content, so the tail line ellipsized away content that fit.
// The congruent side has the mirror flaw and needs no RTL at all: a plain-LTR
// glyph-split word's PREFIX lives on the previous line, but the resumed word
// was counted at full width, so the tail line ellipsized content that fit.
//
// fit_glyphs' backward branch initialized glyph_end to word.glyphs.len()
// instead of the resume bound, which the Forward fix makes reachable: at a
// width too narrow for any resumed glyph it commits an inverted VlRange and
// PANICS downstream ("slice index starts at 9 but ends at 8", observed with
// only that init reverted). The tiny-width case pins that interaction.

use kalamos::{
    fontdb, Attrs, Buffer, Direction, Ellipsize, EllipsizeHeightLimit, FontSystem, Metrics,
    Shaping, Wrap,
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

fn layout(font_system: &mut FontSystem, text: &str, width: f32, ellipsize: Ellipsize) -> Buffer {
    let mut buffer = Buffer::new(font_system, Metrics::new(14.0, 20.0));
    buffer.set_size(Some(width), None);
    buffer.set_wrap(Wrap::Glyph);
    buffer.set_direction(Direction::LeftToRight);
    buffer.set_ellipsize(ellipsize);
    buffer.set_text(text, &Attrs::new(), Shaping::Advanced, None);
    buffer.shape_until_scroll(font_system, false);
    buffer
}

/// Extent of the widest ink edge in the last layout run. Trailing blanks may
/// hang past the width by design; ink may not. The ellipsis glyph carries an
/// empty byte range (start == end) and is ink.
fn last_run_ink_extent(buffer: &Buffer) -> (usize, f32) {
    let mut runs = 0;
    let mut extent = 0.0_f32;
    for run in buffer.layout_runs() {
        runs += 1;
        extent = run
            .glyphs
            .iter()
            .filter(|g| g.start == g.end || !run.text[g.start..g.end].trim().is_empty())
            .map(|g| g.x + g.w)
            .fold(0.0_f32, f32::max);
    }
    (runs, extent)
}

#[test]
fn forward_resume_keeps_the_ellipsized_line_within_the_width() {
    let mut font_system = font_system();
    // Both shapes drive the same path: line 1 glyph-wraps mid-word inside the
    // Arabic span, so the last line resumes at (span, word, glyph > 0).
    for text in ["A سلامسلامسلام zz", "A سلام سلام سلام zz"] {
        for width in [50.0_f32, 60.0] {
            let buffer = layout(
                &mut font_system,
                text,
                width,
                Ellipsize::End(EllipsizeHeightLimit::Lines(2)),
            );
            let (runs, extent) = last_run_ink_extent(&buffer);
            assert_eq!(runs, 2, "the geometry must wrap into exactly two lines");
            assert!(extent > 0.0, "the last line must produce ink");
            assert!(
                extent <= width + 0.5,
                "the ellipsized last line must stay within the buffer width: \
                 extent {extent} > width {width} for {text:?}"
            );
        }
    }
}

/// (run count, byte indices of `text` no glyph covers, ellipsis present).
/// The ellipsis glyph is recognizable by its empty byte range.
fn coverage(buffer: &Buffer, text: &str) -> (usize, Vec<usize>, bool) {
    let mut covered = vec![false; text.len()];
    let mut ellipsis = false;
    let mut runs = 0;
    for run in buffer.layout_runs() {
        runs += 1;
        for glyph in run.glyphs.iter() {
            ellipsis |= glyph.start == glyph.end;
            covered
                .iter_mut()
                .take(glyph.end)
                .skip(glyph.start)
                .for_each(|byte_covered| *byte_covered = true);
        }
    }
    let missing = (0..text.len()).filter(|&b| !covered[b]).collect();
    (runs, missing, ellipsis)
}

#[test]
fn backward_resume_does_not_ellipsize_content_that_fits() {
    let mut font_system = font_system();
    // At width 65 the tail — the rest of the glyph-split Arabic word plus
    // " zz" — fits the last line exactly, with less than an ellipsis of
    // slack. Counting the previous line's committed words as still-pending
    // makes remaining_content_exceeds fire, and the tail line ellipsizes
    // away two bytes that fit. (The window 63..67 was found by a fine sweep;
    // outside it the slack hides the over-count.)
    let text = "A سلام سلام سلام zz";
    let width = 65.0_f32;
    let buffer = layout(
        &mut font_system,
        text,
        width,
        Ellipsize::Start(EllipsizeHeightLimit::Lines(2)),
    );

    let (runs, missing, ellipsis) = coverage(&buffer, text);
    assert_eq!(runs, 2, "the geometry must wrap into exactly two lines");
    assert!(
        missing.is_empty() && !ellipsis,
        "everything fits at this width; nothing may be elided: \
         missing bytes {missing:?}, ellipsis present: {ellipsis}"
    );
    let (_, extent) = last_run_ink_extent(&buffer);
    assert!(
        extent <= width + 0.5,
        "the un-ellipsized tail must still fit: extent {extent} > width {width}"
    );
}

#[test]
fn congruent_resume_counts_only_the_word_suffix_as_remaining() {
    let mut font_system = font_system();
    // Plain LTR, no bidi: line 1 glyph-splits the l-word, so the tail line
    // resumes mid-word and owns only the word's SUFFIX. Counting the resumed
    // word at full width makes remaining_content_exceeds fire while the whole
    // tail — suffix plus "aaa bb" — fits, and three bytes that fit get
    // ellipsized away. (Window 52.5..53.0 found by a fine sweep; below it the
    // tail genuinely overflows, far above it the slack hides the over-count.)
    let text = "llllllllllllllll aaa bb";
    let width = 52.5_f32;
    let buffer = layout(
        &mut font_system,
        text,
        width,
        Ellipsize::Start(EllipsizeHeightLimit::Lines(2)),
    );

    let (runs, missing, ellipsis) = coverage(&buffer, text);
    assert_eq!(runs, 2, "the geometry must wrap into exactly two lines");
    assert!(
        missing.is_empty() && !ellipsis,
        "everything fits at this width; nothing may be elided: \
         missing bytes {missing:?}, ellipsis present: {ellipsis}"
    );
}

#[test]
fn no_resumed_glyph_fits_after_the_ellipsis_reservation() {
    let mut font_system = font_system();
    // Arabic-only content on a forced-LTR base: span 0 itself is
    // incongruent, so line 1 glyph-wraps mid-word at tiny widths and the
    // last line resumes with glyph > 0 while width - ellipsis_w is too
    // narrow for ANY resumed glyph. fit_glyphs must land on the resume
    // bound: the len() init commits an inverted range and panics downstream.
    let text = "سلامسلامسلام";
    let mut width = 12.0_f32;
    while width <= 16.0 {
        let buffer = layout(
            &mut font_system,
            text,
            width,
            Ellipsize::End(EllipsizeHeightLimit::Lines(2)),
        );
        let (runs, extent) = last_run_ink_extent(&buffer);
        assert_eq!(runs, 2, "the geometry must wrap into exactly two lines");
        assert!(
            extent <= width + 0.5,
            "an ellipsis-only last line must stay within the width: \
             extent {extent} > width {width}"
        );
        width += 1.0;
    }
}
