// What this test guards
// ---------------------
// Tab stops are computed on a grid of `tab_width * space_advance`,
// where the space's advance already includes letter_spacing. A
// letter_spacing that exactly cancels the space advance collapses the
// grid to zero: `x / 0` minted inf (or NaN for a leading tab), the
// `floor(..) + 1) * 0` product went NaN, the tab's x_advance poisoned
// the running x, and every glyph after the tab — and the line width —
// went NaN. Wrapping silently stopped, since NaN comparisons are all
// false.
//
// The pin measures the space's em advance with a probe (font_size 16
// so `w / 16` roundtrips exactly in binary float) and negates it, so
// the cancellation is exact by construction; a precondition assert
// keeps the pin honest if advance arithmetic ever changes. A
// degenerate grid now leaves the tab with its shaped advance; the
// fence pins that an ordinary tab still lands on the grid.

use kalamos::{fontdb, Attrs, Buffer, FontSystem, Metrics, Shaping, Wrap};

const FONT_SIZE: f32 = 16.0;

fn font_system() -> FontSystem {
    let mut font_system =
        FontSystem::new_with_locale_and_db("en-US".into(), fontdb::Database::new());
    font_system
        .db_mut()
        .load_font_data(std::fs::read("fonts/Inter-Regular.ttf").unwrap());
    font_system
}

/// The width of a lone space at `letter_spacing`, in pixels.
fn space_width(font_system: &mut FontSystem, letter_spacing: f32) -> f32 {
    let mut buffer = Buffer::new(font_system, Metrics::new(FONT_SIZE, 20.0));
    buffer.set_wrap(Wrap::None);
    buffer.set_text(
        " ",
        &Attrs::new().letter_spacing(letter_spacing),
        Shaping::Advanced,
        None,
    );
    let layout = buffer.line_layout(font_system, 0).expect("one line");
    layout[0].glyphs[0].w
}

#[test]
fn zero_width_tab_grid_keeps_every_position_finite() {
    let mut font_system = font_system();

    // Exact cancellation: the em advance is w / 16, and 16 is a power
    // of two, so negating it reproduces the same f32 the shaper adds.
    let cancel = -(space_width(&mut font_system, 0.0) / FONT_SIZE);
    assert_eq!(
        space_width(&mut font_system, cancel),
        0.0,
        "precondition: the letter spacing cancels the space advance exactly"
    );

    for text in ["\tb", "a\tb and more words after the tab"] {
        let mut buffer = Buffer::new(&mut font_system, Metrics::new(FONT_SIZE, 20.0));
        buffer.set_wrap(Wrap::None);
        buffer.set_text(
            text,
            &Attrs::new().letter_spacing(cancel),
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut font_system, false);

        // Guard against a vacuous pass: NaN geometry can also erase
        // whole layout rows, which would skip the assertions below.
        let glyph_count: usize = buffer.layout_runs().map(|run| run.glyphs.len()).sum();
        assert_eq!(
            glyph_count,
            text.chars().count(),
            "{text:?}: every scalar keeps its glyph"
        );

        for run in buffer.layout_runs() {
            assert!(
                run.line_w.is_finite(),
                "{text:?}: line width must stay finite, got {}",
                run.line_w
            );
            for glyph in run.glyphs {
                assert!(
                    glyph.x.is_finite() && glyph.w.is_finite(),
                    "{text:?}: glyph {}..{} has non-finite geometry: x={} w={}",
                    glyph.start,
                    glyph.end,
                    glyph.x,
                    glyph.w
                );
            }
        }
    }
}

#[test]
fn ordinary_tab_still_lands_on_the_grid() {
    let mut font_system = font_system();
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(FONT_SIZE, 20.0));
    buffer.set_wrap(Wrap::None);
    buffer.set_text("a\tb", &Attrs::new(), Shaping::Advanced, None);
    buffer.shape_until_scroll(&mut font_system, false);

    let run = buffer.layout_runs().next().expect("one run");
    let b = run
        .glyphs
        .iter()
        .find(|glyph| &run.text[glyph.start..glyph.end] == "b")
        .expect("the b glyph");
    let a = run
        .glyphs
        .iter()
        .find(|glyph| &run.text[glyph.start..glyph.end] == "a")
        .expect("the a glyph");
    // The default tab width is 8 spaces; whatever the exact metrics,
    // the glyph after the tab must sit well past 'a' plus one space.
    assert!(
        b.x > a.x + a.w * 3.0,
        "b at {} should sit on a tab stop, not one space after a (a at {}, w {})",
        b.x,
        a.x,
        a.w
    );
}
