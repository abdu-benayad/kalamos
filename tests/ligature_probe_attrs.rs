// What this test guards
// ---------------------
// ShapeSpan::build probes punctuation pairs at line-break opportunities to
// decide whether a break would split a coding ligature ("->", "!=", ...).
// The probe fetched the BREAK POSITION's attrs for font matching but shaped
// with the line's attrs_list at offsets 0..2 — i.e. with the attrs of the
// line's FIRST span. With different font features per span, the probe
// decided from the wrong span: a line opening with a calt-disabled span
// made the probe miss the ligature at a later default-attrs span, the
// break stayed allowed, "->" split into two words, and two separately
// shaped words can never ligate — the arrow glyph vanished from rendering.
//
// Inter substitutes "->" via calt (probed: disabling calt yields 2 glyphs;
// disabling liga/clig/dlig does not), which is what these tests lean on.

use kalamos::{
    fontdb, Attrs, Buffer, FeatureTag, FontFeatures, FontSystem, Metrics, Shaping, Wrap,
};

fn font_system() -> FontSystem {
    let mut font_system =
        FontSystem::new_with_locale_and_db("en-US".into(), fontdb::Database::new());
    font_system
        .db_mut()
        .load_font_data(std::fs::read("fonts/Inter-Regular.ttf").unwrap());
    font_system
}

fn no_calt() -> Attrs<'static> {
    let mut features = FontFeatures::new();
    features.disable(FeatureTag::CONTEXTUAL_ALTERNATES);
    Attrs::new().font_features(features)
}

/// Whether any single glyph covers the full byte range of "->" in `text`.
fn arrow_ligated(font_system: &mut FontSystem, spans: &[(&str, Attrs)]) -> bool {
    let text: String = spans.iter().map(|(s, _)| *s).collect();
    let arrow_start = text.find("->").expect("text contains an arrow");
    let arrow_end = arrow_start + 2;

    let mut buffer = Buffer::new_empty(Metrics::new(16.0, 20.0));
    buffer.set_wrap(Wrap::None);
    buffer.set_rich_text(
        spans.iter().map(|(s, a)| (*s, a.clone())),
        &Attrs::new(),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(font_system, false);

    let mut found = false;
    let mut runs = 0;
    for run in buffer.layout_runs() {
        runs += 1;
        found |= run
            .glyphs
            .iter()
            .any(|g| g.start == arrow_start && g.end == arrow_end);
    }
    assert!(runs > 0, "layout must produce runs");
    found
}

#[test]
fn arrow_survives_a_calt_disabled_leading_span() {
    let mut font_system = font_system();
    assert!(
        arrow_ligated(
            &mut font_system,
            &[("xx ", no_calt()), ("-> yy", Attrs::new())]
        ),
        "the arrow's own span has calt enabled; the probe must judge the \
         break with THAT span's attrs, not the line's first span"
    );
}

#[test]
fn arrow_forms_under_uniform_default_attrs() {
    let mut font_system = font_system();
    assert!(
        arrow_ligated(
            &mut font_system,
            &[("xx ", Attrs::new()), ("-> yy", Attrs::new())]
        ),
        "control: with calt everywhere, '->' shapes as one glyph"
    );
}

#[test]
fn arrow_respects_calt_disabled_at_its_own_span() {
    let mut font_system = font_system();
    assert!(
        !arrow_ligated(
            &mut font_system,
            &[("xx ", Attrs::new()), ("-> yy", no_calt())]
        ),
        "the honest negative: calt off at the arrow's own span means no \
         arrow, whatever the line opens with"
    );
}
