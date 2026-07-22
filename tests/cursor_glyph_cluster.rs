// What this test guards
// ---------------------
// A cluster shaped into several glyphs shares ONE byte range across all of
// them (shape.rs merges the ranges). `cursor_glyph`'s end-matching passes
// used to return the FIRST glyph whose `end` equals the cursor index, so a
// Before caret at the cluster's logical end painted at the first glyph's
// edge — mid-cluster. The caret must sit at the cluster's far visual edge:
// max(x + w) over the matching glyphs for LTR, min(x) for RTL.
//
// "Far edge", not "last matching glyph": glyph storage order inside a
// cluster is shaper output order, and a zero-width mark stored after its
// base would put a last-match caret at the mark's x. The Arabic fence at
// the bottom pins that case.
//
// The repro needs a multi-glyph cluster with non-zero glyph widths. The
// repo has no Indic font, but an emoji ZWJ sequence is one extended
// grapheme cluster, and shaping it with a font that lacks emoji yields
// tofu + ZWJ + tofu + ... — five glyphs, one shared byte range.

use kalamos::{fontdb, Affinity, Attrs, Buffer, Cursor, FontSystem, Metrics, Shaping, Wrap};

const FAMILY_EMOJI: &str = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";

fn shaped_buffer(text: &str, font: &str, locale: &str) -> (FontSystem, Buffer) {
    let mut font_system =
        FontSystem::new_with_locale_and_db(locale.into(), fontdb::Database::new());
    font_system
        .db_mut()
        .load_font_data(std::fs::read(font).unwrap());
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(16.0, 20.0));
    buffer.set_wrap(Wrap::None);
    buffer.set_text(text, &Attrs::new(), Shaping::Advanced, None);
    buffer.shape_until_scroll(&mut font_system, false);
    (font_system, buffer)
}

/// The glyphs of the cluster ending at `end`, asserted non-degenerate:
/// at least two glyphs, and disagreeing on the candidate caret x — so the
/// assertions below cannot pass by every strategy picking the same glyph.
fn cluster_edges(buffer: &Buffer, end: usize, rtl: bool) -> (f32, f32) {
    let run = buffer.layout_runs().next().expect("one layout run");
    let cluster: Vec<_> = run.glyphs.iter().filter(|g| g.end == end).collect();
    assert!(
        cluster.len() >= 2,
        "precondition: the cluster shaped into multiple glyphs (got {})",
        cluster.len()
    );
    let first = cluster.first().expect("non-empty");
    let (first_edge, far_edge) = if rtl {
        (
            first.x,
            cluster.iter().map(|g| g.x).fold(f32::INFINITY, f32::min),
        )
    } else {
        (
            first.x + first.w,
            cluster
                .iter()
                .map(|g| g.x + g.w)
                .fold(f32::NEG_INFINITY, f32::max),
        )
    };
    assert_ne!(
        first_edge, far_edge,
        "precondition: first-match and far-edge disagree, the pin can bite"
    );
    (first_edge, far_edge)
}

#[test]
fn before_caret_at_ltr_cluster_end_sits_at_the_cluster_right_edge() {
    let text = format!("a{FAMILY_EMOJI}b");
    let (_, buffer) = shaped_buffer(&text, "fonts/Inter-Regular.ttf", "en-US");
    let cluster_end = 1 + FAMILY_EMOJI.len();
    let (_, right_edge) = cluster_edges(&buffer, cluster_end, false);

    let cursor = Cursor::new_with_affinity(0, cluster_end, Affinity::Before);
    let (x, _) = buffer.cursor_position(&cursor).expect("caret resolves");
    assert_eq!(
        x, right_edge,
        "the Before caret at the cluster's end is its RIGHT edge on an LTR line"
    );
}

#[test]
fn before_caret_at_rtl_embedded_cluster_end_sits_at_the_cluster_left_edge() {
    // The emoji cluster embedded in Arabic takes the RTL level; its glyphs
    // are stored x-descending, so first-match lands on the RIGHTMOST tofu.
    let text = format!("\u{633}\u{644}\u{627}\u{645} {FAMILY_EMOJI} \u{645}");
    let (_, buffer) = shaped_buffer(&text, "fonts/NotoSansArabic.ttf", "ar");
    let cluster_start = "\u{633}\u{644}\u{627}\u{645} ".len();
    let cluster_end = cluster_start + FAMILY_EMOJI.len();
    let (_, left_edge) = cluster_edges(&buffer, cluster_end, true);

    let cursor = Cursor::new_with_affinity(0, cluster_end, Affinity::Before);
    let (x, _) = buffer.cursor_position(&cursor).expect("caret resolves");
    assert_eq!(
        x, left_edge,
        "the Before caret at the cluster's end is its LEFT edge on an RTL line"
    );
}

#[test]
fn zero_width_mark_does_not_displace_the_before_caret() {
    // Beh + fatha + alef: the fatha is a zero-width mark sharing the beh's
    // cluster range. Every sane strategy agrees on x here (mark.x == base.x
    // with Noto Sans Arabic), so this is a fence: it pins that the caret at
    // the cluster end stays at the base's left edge — a "last matching
    // glyph" implementation that trusted storage order would be one mark
    // reposition away from breaking it.
    let (_, buffer) = shaped_buffer("\u{628}\u{64E}\u{627}", "fonts/NotoSansArabic.ttf", "ar");
    let run = buffer.layout_runs().next().expect("one layout run");
    let cluster: Vec<_> = run.glyphs.iter().filter(|g| g.end == 4).collect();
    assert!(
        cluster.len() >= 2,
        "precondition: base + mark share the cluster range"
    );
    let base_left_edge = cluster.iter().map(|g| g.x).fold(f32::INFINITY, f32::min);

    let cursor = Cursor::new_with_affinity(0, 4, Affinity::Before);
    let (x, _) = buffer.cursor_position(&cursor).expect("caret resolves");
    assert_eq!(x, base_left_edge);
}
