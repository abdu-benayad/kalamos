// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::collections::BTreeMap;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use core::hash::{Hash, Hasher};
use core::ops::Range;
use rangemap::RangeMap;
use smol_str::SmolStr;

use crate::{CacheKeyFlags, Metrics};

pub use fontdb::{Family, Stretch, Style, Weight};

/// Text color
#[derive(Clone, Copy, Debug, PartialOrd, Ord, Eq, Hash, PartialEq)]
pub struct Color(pub u32);

impl Color {
    /// Create new color with red, green, and blue components
    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 0xFF)
    }

    /// Create new color with red, green, blue, and alpha components
    #[inline]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self(((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32))
    }

    /// Get a tuple over all of the attributes, in `(r, g, b, a)` order.
    #[inline]
    pub const fn as_rgba_tuple(self) -> (u8, u8, u8, u8) {
        (self.r(), self.g(), self.b(), self.a())
    }

    /// Get an array over all of the components, in `[r, g, b, a]` order.
    #[inline]
    pub const fn as_rgba(self) -> [u8; 4] {
        [self.r(), self.g(), self.b(), self.a()]
    }

    /// Get the red component
    #[inline]
    pub const fn r(&self) -> u8 {
        ((self.0 & 0x00_FF_00_00) >> 16) as u8
    }

    /// Get the green component
    #[inline]
    pub const fn g(&self) -> u8 {
        ((self.0 & 0x00_00_FF_00) >> 8) as u8
    }

    /// Get the blue component
    #[inline]
    pub const fn b(&self) -> u8 {
        (self.0 & 0x00_00_00_FF) as u8
    }

    /// Get the alpha component
    #[inline]
    pub const fn a(&self) -> u8 {
        ((self.0 & 0xFF_00_00_00) >> 24) as u8
    }
}

/// An owned version of [`Family`]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum FamilyOwned {
    Name(SmolStr),
    Serif,
    SansSerif,
    Cursive,
    Fantasy,
    Monospace,
}

impl FamilyOwned {
    pub fn new(family: Family) -> Self {
        match family {
            Family::Name(name) => Self::Name(SmolStr::from(name)),
            Family::Serif => Self::Serif,
            Family::SansSerif => Self::SansSerif,
            Family::Cursive => Self::Cursive,
            Family::Fantasy => Self::Fantasy,
            Family::Monospace => Self::Monospace,
        }
    }

    pub fn as_family(&self) -> Family<'_> {
        match self {
            Self::Name(name) => Family::Name(name),
            Self::Serif => Family::Serif,
            Self::SansSerif => Family::SansSerif,
            Self::Cursive => Family::Cursive,
            Self::Fantasy => Family::Fantasy,
            Self::Monospace => Family::Monospace,
        }
    }
}

/// f32 bits with one representation per semantic value: -0.0 folds into
/// +0.0 and every NaN payload folds into the canonical quiet NaN, so
/// bit-based Eq/Hash agree with f32 `==` wherever `==` can distinguish.
fn canonical_bits(value: f32) -> u32 {
    const CANONICAL_NAN_BITS: u32 = 0x7fc0_0000;

    if value.is_nan() {
        CANONICAL_NAN_BITS
    } else {
        // Adding +0.0 canonicalizes -0.0 to +0.0
        (value + 0.0).to_bits()
    }
}

/// Metrics, but implementing Eq and Hash using u32 representation of f32
///
/// Construction canonicalizes the bits (`canonical_bits`), so the roundtrip back
/// to [`Metrics`] returns +0.0 for -0.0 and the canonical NaN for any NaN —
/// indistinguishable under f32 `==`, which is the point: values that compare
/// equal share one cache identity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CacheMetrics {
    font_size_bits: u32,
    line_height_bits: u32,
}

impl From<Metrics> for CacheMetrics {
    fn from(metrics: Metrics) -> Self {
        Self {
            font_size_bits: canonical_bits(metrics.font_size),
            line_height_bits: canonical_bits(metrics.line_height),
        }
    }
}

impl From<CacheMetrics> for Metrics {
    fn from(metrics: CacheMetrics) -> Self {
        Self {
            font_size: f32::from_bits(metrics.font_size_bits),
            line_height: f32::from_bits(metrics.line_height_bits),
        }
    }
}
/// A 4-byte `OpenType` feature tag identifier
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FeatureTag([u8; 4]);

impl FeatureTag {
    pub const fn new(tag: &[u8; 4]) -> Self {
        Self(*tag)
    }

    /// Kerning adjusts spacing between specific character pairs
    pub const KERNING: Self = Self::new(b"kern");
    /// Standard ligatures (fi, fl, etc.)
    pub const STANDARD_LIGATURES: Self = Self::new(b"liga");
    /// Contextual ligatures (context-dependent ligatures)
    pub const CONTEXTUAL_LIGATURES: Self = Self::new(b"clig");
    /// Contextual alternates (glyph substitutions based on context)
    pub const CONTEXTUAL_ALTERNATES: Self = Self::new(b"calt");
    /// Discretionary ligatures (optional stylistic ligatures)
    pub const DISCRETIONARY_LIGATURES: Self = Self::new(b"dlig");
    /// Small caps (lowercase to small capitals)
    pub const SMALL_CAPS: Self = Self::new(b"smcp");
    /// All small caps (uppercase and lowercase to small capitals)
    pub const ALL_SMALL_CAPS: Self = Self::new(b"c2sc");
    /// Stylistic Set 1 (font-specific alternate glyphs)
    pub const STYLISTIC_SET_1: Self = Self::new(b"ss01");
    /// Stylistic Set 2 (font-specific alternate glyphs)
    pub const STYLISTIC_SET_2: Self = Self::new(b"ss02");

    pub const fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }
}

/// `OpenType` feature settings: one value per [`FeatureTag`].
///
/// The map model makes contradictory states unrepresentable — a tag has
/// exactly one value, the last one written — and gives a canonical order,
/// so two `FontFeatures` built from the same per-tag values compare and
/// hash equal regardless of write order (they are cache-key material via
/// [`Attrs`]).
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct FontFeatures {
    features: BTreeMap<FeatureTag, u32>,
}

impl FontFeatures {
    pub const fn new() -> Self {
        Self {
            features: BTreeMap::new(),
        }
    }

    /// Set `tag` to `value`, replacing any earlier value for the same tag
    pub fn set(&mut self, tag: FeatureTag, value: u32) -> &mut Self {
        self.features.insert(tag, value);
        self
    }

    /// The settings, in canonical (tag) order
    pub fn iter(&self) -> impl Iterator<Item = (FeatureTag, u32)> + '_ {
        self.features.iter().map(|(tag, value)| (*tag, *value))
    }

    /// Enable a feature (set to 1)
    pub fn enable(&mut self, tag: FeatureTag) -> &mut Self {
        self.set(tag, 1)
    }

    /// Disable a feature (set to 0)
    pub fn disable(&mut self, tag: FeatureTag) -> &mut Self {
        self.set(tag, 0)
    }
}

/// A wrapper for letter spacing to get around that f32 doesn't implement Eq and Hash
#[derive(Clone, Copy, Debug)]
pub struct LetterSpacing(pub f32);

impl PartialEq for LetterSpacing {
    fn eq(&self, other: &Self) -> bool {
        if self.0.is_nan() {
            other.0.is_nan()
        } else {
            self.0 == other.0
        }
    }
}

impl Eq for LetterSpacing {}

impl Hash for LetterSpacing {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        canonical_bits(self.0).hash(hasher);
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum UnderlineStyle {
    #[default]
    None,
    Single,
    Double,
    // TODO: Wavy
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct TextDecoration {
    pub underline: UnderlineStyle,
    pub underline_color_opt: Option<Color>,
    pub strikethrough: bool,
    pub strikethrough_color_opt: Option<Color>,
    pub overline: bool,
    pub overline_color_opt: Option<Color>,
}

impl TextDecoration {
    pub const fn new() -> Self {
        Self {
            underline: UnderlineStyle::None,
            underline_color_opt: None,
            strikethrough: false,
            strikethrough_color_opt: None,
            overline: false,
            overline_color_opt: None,
        }
    }

    pub const fn has_decoration(&self) -> bool {
        !matches!(self.underline, UnderlineStyle::None) || self.strikethrough || self.overline
    }
}

/// Offset and thickness for a text decoration line, in EM units.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DecorationMetrics {
    /// Offset from baseline in EM units
    pub offset: f32,
    /// Thickness in EM units
    pub thickness: f32,
}

/// The decoration geometry a font itself declares, in EM units — where that font wants an
/// underline, a strikethrough and (via [`ascent`](Self::ascent)) an overline drawn, at what
/// thickness.
///
/// Independent of whether any text asked for a decoration: it is a property of the face, not
/// of a [`TextDecoration`]. Read it with [`Font::decoration_metrics`](crate::Font::decoration_metrics)
/// to place a decoration line a caller draws itself — a link underline painted at hover time,
/// say, where routing the decoration through [`Attrs`] would mean reshaping the text.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FontDecorationMetrics {
    /// Underline offset and thickness.
    pub underline: DecorationMetrics,
    /// Strikethrough offset and thickness.
    pub strikethrough: DecorationMetrics,
    /// Font ascent in EM units (`ascent / upem`). Used for overline positioning.
    pub ascent: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GlyphDecorationData {
    /// The text decoration configuration from the user
    pub text_decoration: TextDecoration,
    /// Where the span's font wants those decorations drawn
    pub font: FontDecorationMetrics,
}

/// Text attributes
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Attrs<'a> {
    //TODO: should this be an option?
    pub color_opt: Option<Color>,
    pub family: Family<'a>,
    pub stretch: Stretch,
    pub style: Style,
    pub weight: Weight,
    pub metadata: usize,
    pub cache_key_flags: CacheKeyFlags,
    pub metrics_opt: Option<CacheMetrics>,
    /// Letter spacing (tracking) in EM
    pub letter_spacing_opt: Option<LetterSpacing>,
    pub font_features: FontFeatures,
    pub text_decoration: TextDecoration,
}

impl<'a> Attrs<'a> {
    /// Create a new set of attributes with sane defaults
    ///
    /// This defaults to a regular Sans-Serif font.
    pub const fn new() -> Self {
        Self {
            color_opt: None,
            family: Family::SansSerif,
            stretch: Stretch::Normal,
            style: Style::Normal,
            weight: Weight::NORMAL,
            metadata: 0,
            cache_key_flags: CacheKeyFlags::empty(),
            metrics_opt: None,
            letter_spacing_opt: None,
            font_features: FontFeatures::new(),
            text_decoration: TextDecoration::new(),
        }
    }

    /// Set [Color]
    pub const fn color(mut self, color: Color) -> Self {
        self.color_opt = Some(color);
        self
    }

    /// Set [Family]
    pub const fn family(mut self, family: Family<'a>) -> Self {
        self.family = family;
        self
    }

    /// Set [Stretch]
    pub const fn stretch(mut self, stretch: Stretch) -> Self {
        self.stretch = stretch;
        self
    }

    /// Set [Style]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set [Weight]
    pub const fn weight(mut self, weight: Weight) -> Self {
        self.weight = weight;
        self
    }

    /// Set metadata
    pub const fn metadata(mut self, metadata: usize) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set [`CacheKeyFlags`]
    pub const fn cache_key_flags(mut self, cache_key_flags: CacheKeyFlags) -> Self {
        self.cache_key_flags = cache_key_flags;
        self
    }

    /// Set [`Metrics`], overriding values in buffer
    pub fn metrics(mut self, metrics: Metrics) -> Self {
        self.metrics_opt = Some(metrics.into());
        self
    }

    /// Set letter spacing (tracking) in EM
    pub const fn letter_spacing(mut self, letter_spacing: f32) -> Self {
        self.letter_spacing_opt = Some(LetterSpacing(letter_spacing));
        self
    }

    /// Set [`FontFeatures`]
    pub fn font_features(mut self, font_features: FontFeatures) -> Self {
        self.font_features = font_features;
        self
    }

    pub const fn underline(mut self, style: UnderlineStyle) -> Self {
        self.text_decoration.underline = style;
        self
    }

    pub const fn underline_color(mut self, color: Color) -> Self {
        self.text_decoration.underline_color_opt = Some(color);
        self
    }

    pub const fn strikethrough(mut self) -> Self {
        self.text_decoration.strikethrough = true;
        self
    }

    pub const fn strikethrough_color(mut self, color: Color) -> Self {
        self.text_decoration.strikethrough_color_opt = Some(color);
        self
    }

    pub const fn overline(mut self) -> Self {
        self.text_decoration.overline = true;
        self
    }

    pub const fn overline_color(mut self, color: Color) -> Self {
        self.text_decoration.overline_color_opt = Some(color);
        self
    }

    /// Check if this set of attributes can be shaped with another
    pub fn compatible(&self, other: &Self) -> bool {
        self.family == other.family
            && self.stretch == other.stretch
            && self.style == other.style
            && self.weight == other.weight
    }
}

/// Font-specific part of [`Attrs`] to be used for matching
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FontMatchAttrs {
    family: FamilyOwned,
    stretch: Stretch,
    style: Style,
    weight: Weight,
}

impl<'a> From<&Attrs<'a>> for FontMatchAttrs {
    fn from(attrs: &Attrs<'a>) -> Self {
        Self {
            family: FamilyOwned::new(attrs.family),
            stretch: attrs.stretch,
            style: attrs.style,
            weight: attrs.weight,
        }
    }
}

/// An owned version of [`Attrs`]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AttrsOwned {
    //TODO: should this be an option?
    pub color_opt: Option<Color>,
    pub family_owned: FamilyOwned,
    pub stretch: Stretch,
    pub style: Style,
    pub weight: Weight,
    pub metadata: usize,
    pub cache_key_flags: CacheKeyFlags,
    pub metrics_opt: Option<CacheMetrics>,
    /// Letter spacing (tracking) in EM
    pub letter_spacing_opt: Option<LetterSpacing>,
    pub font_features: FontFeatures,
    pub text_decoration: TextDecoration,
}

impl AttrsOwned {
    pub fn new(attrs: &Attrs) -> Self {
        Self {
            color_opt: attrs.color_opt,
            family_owned: FamilyOwned::new(attrs.family),
            stretch: attrs.stretch,
            style: attrs.style,
            weight: attrs.weight,
            metadata: attrs.metadata,
            cache_key_flags: attrs.cache_key_flags,
            metrics_opt: attrs.metrics_opt,
            letter_spacing_opt: attrs.letter_spacing_opt,
            font_features: attrs.font_features.clone(),
            text_decoration: attrs.text_decoration,
        }
    }

    pub fn as_attrs(&self) -> Attrs<'_> {
        Attrs {
            color_opt: self.color_opt,
            family: self.family_owned.as_family(),
            stretch: self.stretch,
            style: self.style,
            weight: self.weight,
            metadata: self.metadata,
            cache_key_flags: self.cache_key_flags,
            metrics_opt: self.metrics_opt,
            letter_spacing_opt: self.letter_spacing_opt,
            font_features: self.font_features.clone(),
            text_decoration: self.text_decoration,
        }
    }
}

/// List of text attributes to apply to a line
//TODO: have this clean up the spans when changes are made
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AttrsList {
    defaults: AttrsOwned,
    pub(crate) spans: RangeMap<usize, AttrsOwned>,
}

impl AttrsList {
    /// Create a new attributes list with a set of default [Attrs]
    pub fn new(defaults: &Attrs) -> Self {
        Self {
            defaults: AttrsOwned::new(defaults),
            spans: RangeMap::new(),
        }
    }

    /// Get the default [Attrs]
    pub fn defaults(&self) -> Attrs<'_> {
        self.defaults.as_attrs()
    }

    /// Get the current attribute spans
    pub fn spans(&self) -> Vec<(&Range<usize>, &AttrsOwned)> {
        self.spans_iter().collect()
    }

    /// Get an iterator over the current attribute spans
    pub fn spans_iter(&self) -> impl Iterator<Item = (&Range<usize>, &AttrsOwned)> + '_ {
        self.spans.iter()
    }

    /// Clear the current attribute spans
    pub fn clear_spans(&mut self) {
        self.spans.clear();
    }

    /// Add an attribute span, removes any previous matching parts of spans
    pub fn add_span(&mut self, range: Range<usize>, attrs: &Attrs) {
        //do not support 1..1 or 2..1 even if by accident.
        if range.is_empty() {
            return;
        }

        self.spans.insert(range, AttrsOwned::new(attrs));
    }

    /// Get the attribute span for an index
    ///
    /// This returns a span that contains the index
    pub fn get_span(&self, index: usize) -> Attrs<'_> {
        self.spans
            .get(&index)
            .map(|v| v.as_attrs())
            .unwrap_or(self.defaults.as_attrs())
    }

    /// Split attributes list at an offset
    #[allow(clippy::missing_panics_doc)]
    pub fn split_off(&mut self, index: usize) -> Self {
        let mut new = Self::new(&self.defaults.as_attrs());
        let mut removes = Vec::new();

        //get the keys we need to remove or fix.
        for span in self.spans.iter() {
            if span.0.end <= index {
                continue;
            }

            if span.0.start >= index {
                removes.push((span.0.clone(), false));
            } else {
                removes.push((span.0.clone(), true));
            }
        }

        for (key, resize) in removes {
            #[expect(
                clippy::expect_used,
                reason = "every key in `removes` was collected from self.spans just above, \
                          and each distinct key is looked up and removed exactly once"
            )]
            let (range, attrs) = self
                .spans
                .get_key_value(&key.start)
                .map(|v| (v.0.clone(), v.1.clone()))
                .expect("attrs span not found");
            self.spans.remove(key);

            if resize {
                new.spans.insert(0..range.end - index, attrs.clone());
                self.spans.insert(range.start..index, attrs);
            } else {
                new.spans
                    .insert(range.start - index..range.end - index, attrs);
            }
        }
        new
    }

    /// Resets the attributes with new defaults.
    pub(crate) fn reset(mut self, default: &Attrs) -> Self {
        self.defaults = AttrsOwned::new(default);
        self.spans.clear();
        self
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // CacheMetrics is cache identity for Metrics, so values equal under
    // f32 == must be one identity: -0.0 folds into +0.0 and every NaN
    // payload folds into the canonical NaN. Raw to_bits() split them.

    #[test]
    fn negative_zero_and_positive_zero_are_one_cache_identity() {
        assert_eq!(
            CacheMetrics::from(Metrics::new(-0.0, 20.0)),
            CacheMetrics::from(Metrics::new(0.0, 20.0)),
        );
        assert_eq!(
            CacheMetrics::from(Metrics::new(16.0, -0.0)),
            CacheMetrics::from(Metrics::new(16.0, 0.0)),
        );
    }

    #[test]
    fn all_nan_payloads_are_one_cache_identity() {
        let other_nan = f32::from_bits(f32::NAN.to_bits() ^ 1);
        assert!(other_nan.is_nan(), "precondition: still a NaN");
        assert_eq!(
            CacheMetrics::from(Metrics::new(f32::NAN, 20.0)),
            CacheMetrics::from(Metrics::new(other_nan, 20.0)),
        );
    }

    #[test]
    fn ordinary_metrics_roundtrip_exactly() {
        let metrics = Metrics::new(14.5, 19.25);
        let roundtripped = Metrics::from(CacheMetrics::from(metrics));
        assert_eq!(roundtripped.font_size, metrics.font_size);
        assert_eq!(roundtripped.line_height, metrics.line_height);
    }

    // FontFeatures holds one value per tag: enable-then-disable is the same
    // state as disable alone, later writes win, and insertion order is not
    // observable. The old Vec model kept every write, so two FontFeatures
    // built from the same per-tag values compared (and hashed) unequal.

    #[test]
    fn enable_then_disable_is_disable() {
        let mut noisy = FontFeatures::new();
        noisy
            .enable(FeatureTag::KERNING)
            .disable(FeatureTag::KERNING);
        let mut quiet = FontFeatures::new();
        quiet.disable(FeatureTag::KERNING);
        assert_eq!(noisy, quiet, "only the last write per tag is state");
    }

    #[test]
    fn later_set_overwrites_earlier() {
        let mut twice = FontFeatures::new();
        twice
            .set(FeatureTag::STYLISTIC_SET_1, 2)
            .set(FeatureTag::STYLISTIC_SET_1, 5);
        let mut once = FontFeatures::new();
        once.set(FeatureTag::STYLISTIC_SET_1, 5);
        assert_eq!(twice, once);
    }

    #[test]
    fn insertion_order_is_not_observable() {
        let mut kern_first = FontFeatures::new();
        kern_first
            .enable(FeatureTag::KERNING)
            .enable(FeatureTag::STANDARD_LIGATURES);
        let mut liga_first = FontFeatures::new();
        liga_first
            .enable(FeatureTag::STANDARD_LIGATURES)
            .enable(FeatureTag::KERNING);
        assert_eq!(
            kern_first, liga_first,
            "the same per-tag values are the same state regardless of write order"
        );
    }
}
