//! Golden values for [`Font::decoration_metrics`], the paint-time view of a face's decoration
//! geometry.
//!
//! The query exists so a caller can place a decoration line *without* asking for one through
//! [`Attrs`]: a link underline that appears on hover, say, where routing the decoration through
//! the attributes would mean reshaping — and re-caching — the text on every hover. Such a caller
//! draws the line itself from these EM-unit numbers.
//!
//! **These are goldens, not a cross-check.** Asserting that the shaper's baked-in metrics equal
//! the query's would pass unconditionally: the shaper *calls* the query, so any drift moves both
//! sides together. (Verified — perturbing the query's offset left that assertion green.) Fixed
//! numbers per face are what actually catches a changed normalization, and asserting the shaped
//! span against the same numbers catches it on either side of the split.
//!
//! Two faces, deliberately: Noto Sans and Inter declare *different* underline geometry, so a
//! regression that returned one constant for every font — or that dropped the `/ upem`
//! normalization — fails here rather than passing on a font that happened to agree.

use std::path::PathBuf;

use kalamos::{
    fontdb::Database, Attrs, Buffer, DecorationMetrics, Family, FontDecorationMetrics, FontSystem,
    Metrics, Shaping, UnderlineStyle,
};

/// What one face is expected to declare. Read off the committed `.ttf`s; a diff here means
/// either the font file changed or the EM normalization did.
struct Golden {
    family: &'static str,
    text: &'static str,
    metrics: FontDecorationMetrics,
}

const GOLDENS: &[Golden] = &[
    Golden {
        family: "Noto Sans",
        text: "Underlined",
        metrics: FontDecorationMetrics {
            underline: DecorationMetrics {
                offset: -0.1,
                thickness: 0.05,
            },
            strikethrough: DecorationMetrics {
                offset: 0.322,
                thickness: 0.05,
            },
            ascent: 1.069,
        },
    },
    Golden {
        family: "Inter",
        text: "Underlined",
        metrics: FontDecorationMetrics {
            underline: DecorationMetrics {
                offset: -0.164_772_73,
                thickness: 0.068_181_82,
            },
            strikethrough: DecorationMetrics {
                offset: 0.327_414_78,
                thickness: 0.068_181_82,
            },
            ascent: 0.968_75,
        },
    },
];

/// The Arabic face, checked for shape rather than exact numbers: it is here to prove the query
/// works on an RTL face at all, and its ascent differs sharply from the Latin ones.
const ARABIC_FAMILY: &str = "Noto Sans Arabic";

/// A font system holding only the repo's own fonts, so the faces under test are the committed
/// ones rather than whatever the host has installed.
fn repo_font_system() -> FontSystem {
    let repo_dir = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let mut font_db = Database::new();
    font_db.load_fonts_dir(PathBuf::from(&repo_dir).join("fonts"));
    FontSystem::new_with_locale_and_db("En-US".into(), font_db)
}

/// Shape one underlined line in `family` and return the metrics the shaper baked into its
/// decoration span alongside the ones [`Font::decoration_metrics`] reports for the same face.
///
/// Panics if the text resolved to a *different* family than the one requested. Font matching
/// falls back silently, and a golden compared against an unrequested fallback face is a test
/// asserting nothing — an earlier draft of this file "passed" for two families that are not in
/// `fonts/` at all.
fn shaped_and_queried(text: &str, family: &str) -> (FontDecorationMetrics, FontDecorationMetrics) {
    let mut font_system = repo_font_system();
    let attrs = Attrs::new()
        .family(Family::Name(family))
        .underline(UnderlineStyle::Single);

    let mut buffer = Buffer::new(&mut font_system, Metrics::new(20.0, 26.0));
    {
        let mut borrowed = buffer.borrow_with(&mut font_system);
        borrowed.set_size(Some(400.0), Some(100.0));
        borrowed.set_text(text, &attrs, Shaping::Advanced, None);
        borrowed.shape_until_scroll(true);
    }

    let run = buffer
        .layout_runs()
        .next()
        .expect("one line of text lays out to at least one run");
    assert_eq!(
        run.decorations.len(),
        1,
        "{family}: the whole line carries one underline span, so there is something to compare"
    );
    let span = &run.decorations[0];
    let shaped = span.data.font;

    let glyph = run
        .glyphs
        .get(span.glyph_range.start)
        .expect("a decoration span covers at least one glyph");
    let resolved = font_system
        .db()
        .face(glyph.font_id)
        .expect("the run was shaped against a face that is in the database")
        .families
        .clone();
    assert!(
        resolved.iter().any(|(name, _)| name == family),
        "{family}: shaped against {resolved:?} instead — the request fell back, \
         so any golden below would be checking the wrong face"
    );

    let font = font_system
        .get_font(glyph.font_id, glyph.font_weight)
        .expect("the font the run was shaped against is still loadable");

    (shaped, font.decoration_metrics())
}

#[test]
fn the_query_reports_each_face_s_own_declared_geometry() {
    for golden in GOLDENS {
        let (shaped, queried) = shaped_and_queried(golden.text, golden.family);
        assert_eq!(
            queried, golden.metrics,
            "{}: Font::decoration_metrics drifted from the face's declared geometry",
            golden.family
        );
        assert_eq!(
            shaped, golden.metrics,
            "{}: the shaper baked in metrics the face does not declare",
            golden.family
        );
    }

    let [noto, inter] = [&GOLDENS[0], &GOLDENS[1]];
    assert_ne!(
        noto.metrics, inter.metrics,
        "the two goldens must differ, or a query returning one constant for every font passes"
    );
}

#[test]
fn the_metrics_place_a_usable_underline() {
    let families = GOLDENS
        .iter()
        .map(|g| (g.text, g.family))
        .chain([("مسطر", ARABIC_FAMILY)]);

    for (text, family) in families {
        let (_, queried) = shaped_and_queried(text, family);

        assert!(
            queried.underline.thickness > 0.0,
            "{family}: an underline with no thickness draws nothing"
        );
        // `render_decoration` places the line at `line_y - offset * font_size`, so a line below
        // the baseline — where an underline belongs — needs a negative offset.
        assert!(
            queried.underline.offset < 0.0,
            "{family}: underline offset {} would draw the line above the baseline",
            queried.underline.offset
        );
        assert!(
            queried.ascent > 0.0,
            "{family}: a non-positive ascent puts the overline on or under the baseline"
        );
        // A decoration a half-em thick means the EM normalization is wrong — raw font units
        // leaking through, most likely.
        assert!(
            queried.underline.thickness < 0.5,
            "{family}: underline thickness {} em is not a thickness, it is a bar",
            queried.underline.thickness
        );
    }
}
