// What this test guards
// ---------------------
// `BidiParagraphs` used to have two diverging implementations: an ASCII
// fast path that split only on '\n' (never on a lone '\r'), and a
// `unicode_bidi::BidiInfo` slow path that split at *every* class-B char
// with no CR LF coalescing — so any non-ASCII character anywhere in the
// input made every CRLF yield a phantom empty paragraph, i.e. a visible
// blank line per line break in exactly the RTL content this crate exists
// for. There is now one implementation; these tests pin its contract:
// CR LF is one separator, lone CR splits, every UAX #9 class-B character
// splits, and ASCII vs non-ASCII input cannot disagree.

use kalamos::{Attrs, BidiParagraphs, Buffer, Metrics, Shaping};

fn paragraphs(text: &str) -> Vec<&str> {
    BidiParagraphs::new(text).collect()
}

#[test]
fn crlf_is_one_separator() {
    assert_eq!(paragraphs("abc\r\ndef"), ["abc", "def"]);
}

#[test]
fn crlf_with_non_ascii_content_matches_ascii_behavior() {
    // The original defect: this input took the BidiInfo path and yielded
    // ["ابجد", "", "abc"] — a phantom empty paragraph from the CRLF.
    assert_eq!(paragraphs("ابجد\r\nabc"), ["ابجد", "abc"]);
}

#[test]
fn lone_cr_splits() {
    // The old ASCII fast path kept the CR inside the paragraph.
    assert_eq!(paragraphs("a\rb"), ["a", "b"]);
}

#[test]
fn lf_then_crlf_is_two_paragraphs() {
    // An empty LF-terminated line followed by an empty CRLF-terminated one.
    assert_eq!(paragraphs("\n\r\n"), ["", ""]);
}

#[test]
fn every_class_b_separator_splits() {
    // PS (U+2029) and NEL (U+0085) are Paragraph_Separator too.
    assert_eq!(paragraphs("a\u{2029}b\u{0085}c"), ["a", "b", "c"]);
}

#[test]
fn trailing_separator_yields_no_empty_paragraph() {
    assert_eq!(paragraphs("a\n"), ["a"]);
    assert_eq!(paragraphs("a\n\n"), ["a", ""]);
    assert_eq!(paragraphs(""), Vec::<&str>::new());
}

#[test]
fn rich_text_crlf_produces_no_phantom_buffer_line() {
    // End-to-end pin on the consumer: two CRLF-joined Arabic spans must
    // produce exactly two buffer lines, not three. Line splitting happens
    // eagerly in set_rich_text; no font or shaping is involved.
    let mut buffer = Buffer::new_empty(Metrics::new(14.0, 20.0));
    let attrs = Attrs::new();
    buffer.set_rich_text(
        [("ابجد\r\n", attrs.clone()), ("هوز", attrs.clone())],
        &attrs,
        Shaping::Advanced,
        None,
    );
    assert_eq!(buffer.lines.len(), 2);
}
