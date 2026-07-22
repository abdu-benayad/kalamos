// SPDX-License-Identifier: MIT OR Apache-2.0

use unicode_bidi::{bidi_class, BidiClass};

/// An iterator over the paragraphs in the input text.
///
/// A paragraph ends at any character of bidi class `B` (UAX #9
/// `Paragraph_Separator`: LF, CR, NEL, PS, and the information separators),
/// with CR LF consumed as a single separator. This is
/// [`core::str::Lines`] extended to every paragraph separator — where the
/// two disagree (lone CR, NEL, PS), the separator wins, matching how
/// [`crate::LineEnding`] terminates lines in `set_text`.
///
/// Yields the paragraph *content only*; separators are consumed, never
/// returned. Text ending in a separator does not yield a trailing empty
/// paragraph, exactly like [`core::str::Lines`].
#[derive(Debug)]
pub struct BidiParagraphs<'text> {
    /// The not-yet-yielded tail of the input.
    remaining: &'text str,
}

impl<'text> BidiParagraphs<'text> {
    /// Create an iterator over the paragraphs of `text`.
    pub fn new(text: &'text str) -> Self {
        Self { remaining: text }
    }
}

impl<'text> Iterator for BidiParagraphs<'text> {
    type Item = &'text str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining.is_empty() {
            return None;
        }
        let separator = self
            .remaining
            .char_indices()
            .find(|&(_, c)| bidi_class(c) == BidiClass::B);
        match separator {
            Some((start, c)) => {
                let paragraph = &self.remaining[..start];
                let after = start + c.len_utf8();
                // CR LF is one separator, not two.
                let after = if c == '\r' && self.remaining[after..].starts_with('\n') {
                    after + 1
                } else {
                    after
                };
                self.remaining = &self.remaining[after..];
                Some(paragraph)
            }
            None => {
                let paragraph = self.remaining;
                self.remaining = "";
                Some(paragraph)
            }
        }
    }
}
