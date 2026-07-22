// What this test guards
// ---------------------
// `ShapeWord::build` has a fast path for simple ASCII words that skips
// per-grapheme attribute iteration when every *explicit* span
// overlapping the word carries compatible attrs. But `AttrsList` spans
// need not cover the word: `get_span` resolves uncovered bytes to the
// list's defaults, and the slow path honors that by starting from
// `attrs_list.defaults()`. The fast path never consulted the defaults,
// so a word straddling a span→gap boundary shaped entirely in the
// span's font — "hello" with a Fira Mono span over bytes 0..3 and
// Inter defaults produced five Fira Mono glyphs.
//
// The fast path now requires full span coverage of the word, or
// defaults compatible with the word's starting attrs. The fence pins
// that a fully-covered word still takes the fast path result (one font
// throughout).

use kalamos::{fontdb, Attrs, AttrsList, Buffer, Family, FontSystem, Metrics, Shaping, Wrap};

fn font_system() -> FontSystem {
    let mut font_system =
        FontSystem::new_with_locale_and_db("en-US".into(), fontdb::Database::new());
    for path in ["fonts/Inter-Regular.ttf", "fonts/FiraMono-Medium.ttf"] {
        font_system
            .db_mut()
            .load_font_data(std::fs::read(path).unwrap());
    }
    font_system
}

fn face_id(font_system: &FontSystem, post_script_name: &str) -> fontdb::ID {
    font_system
        .db()
        .faces()
        .find(|face| face.post_script_name == post_script_name)
        .map(|face| face.id)
        .unwrap_or_else(|| panic!("fixture face {post_script_name} is loaded"))
}

/// Shape "hello" with Inter defaults and a Fira Mono span over `span`,
/// returning each glyph's byte range and resolved font.
fn glyph_fonts(
    span: std::ops::Range<usize>,
) -> (Vec<(usize, usize, fontdb::ID)>, fontdb::ID, fontdb::ID) {
    let mut font_system = font_system();
    let inter = face_id(&font_system, "Inter-Regular");
    let fira = face_id(&font_system, "FiraMono-Medium");

    let default_attrs = Attrs::new().family(Family::Name("Inter"));
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 20.0));
    buffer.set_wrap(Wrap::None);
    buffer.set_text("hello", &default_attrs, Shaping::Advanced, None);

    let mut attrs_list = AttrsList::new(&default_attrs);
    attrs_list.add_span(span, &Attrs::new().family(Family::Name("Fira Mono")));
    buffer.lines[0].set_attrs_list(attrs_list);

    let layout = buffer.line_layout(&mut font_system, 0).unwrap();
    let glyphs = layout[0]
        .glyphs
        .iter()
        .map(|glyph| (glyph.start, glyph.end, glyph.font_id))
        .collect();
    (glyphs, inter, fira)
}

#[test]
fn gap_bytes_shape_in_the_defaults_font() {
    let (glyphs, inter, fira) = glyph_fonts(0..3);

    assert_eq!(glyphs.len(), 5, "five ASCII glyphs");
    for (start, end, font_id) in glyphs {
        let expected = if start < 3 { fira } else { inter };
        assert_eq!(
            font_id, expected,
            "bytes {start}..{end}: the span covers 0..3, the gap 3..5 \
             falls back to the Inter defaults"
        );
    }
}

#[test]
fn fully_covered_word_keeps_one_font() {
    let (glyphs, _inter, fira) = glyph_fonts(0..5);

    assert_eq!(glyphs.len(), 5, "five ASCII glyphs");
    for (start, end, font_id) in glyphs {
        assert_eq!(font_id, fira, "bytes {start}..{end} are inside the span");
    }
}
