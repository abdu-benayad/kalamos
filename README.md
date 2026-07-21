# Kalamos

[![crates.io](https://img.shields.io/crates/v/kalamos.svg)](https://crates.io/crates/kalamos)
[![docs.rs](https://docs.rs/kalamos/badge.svg)](https://docs.rs/kalamos)
![license](https://img.shields.io/crates/l/kalamos.svg)

**RTL-first text shaping, bidirectional layout, and rasterization in pure Rust.**

*κάλαμος* — the reed pen. The Arabic *qalam* is borrowed from it: the word itself
crossed the boundary this library exists to span.

Kalamos provides text shaping, layout, and rendering behind one abstraction.
Shaping is HarfRust, and supports the full range of advanced shaping operations.
Rasterization is swash, with ligatures and colour emoji. Layout and font fallback
are custom, in safe Rust. Linux, macOS, and Windows are supported with the full
feature set; other platforms may need to supply their own font fallback.

## Right-to-left is a requirement, not a feature flag

Kalamos treats bidirectional and right-to-left correctness as the primary design
constraint rather than a case to be handled. In practice that means:

- **Base direction is forced, never guessed.** A field's bidi base can be set
  explicitly instead of inferred from content, so a mixed-script value does not
  silently reverse when its first strong character changes.
- **Direction-independent glyph positions.** Lines pin to logical-left alignment,
  so the caller owns cell alignment and there is no double-offset when an RTL line
  is laid out inside a finite width.
- **Ellipsization that does not eat runs.** Constraining a line to a width must not
  delete its logically-leading run — a defect that leaves the reported line width
  and the glyph extents mutually consistent, and therefore invisible to every
  geometric check. Regression-guarded by glyph census, not geometry.
- **Caret and hit-testing that respect run boundaries,** including affinity at the
  seam between opposing runs.

## Lineage

Kalamos began as a fork of [cosmic-text](https://github.com/pop-os/cosmic-text) by
Jeremy Soller and System76, and carries their copyright alongside its own under the
original MIT / Apache-2.0 dual licence. It is now maintained independently: it does
not track upstream, and upstream is not responsible for it. Bugs here are ours.

## Screenshots

Arabic translation of Universal Declaration of Human Rights
[![Arabic screenshot](screenshots/arabic.png)](screenshots/arabic.png)

Hindi translation of Universal Declaration of Human Rights
[![Hindi screenshot](screenshots/hindi.png)](screenshots/hindi.png)

Simplified Chinese translation of Universal Declaration of Human Rights
[![Simplified Chinses screenshot](screenshots/chinese-simplified.png)](screenshots/chinese-simplified.png)

[View Universal Declaration of Human Rights on OHCHR](https://www.ohchr.org/en/universal-declaration-of-human-rights)

## Roadmap

The following features must be supported before this is "ready":

- [x] Font loading (using fontdb)
  - [x] Preset fonts
  - [x] System fonts
- [x] Text styles (bold, italic, etc.)
  - [x] Per-buffer
  - [x] Per-span
- [x] Font shaping (using HarfRust)
  - [x] Cache results
  - [x] RTL
  - [x] Bidirectional rendering
- [x] Font fallback
  - [x] Choose font based on locale to work around "unification"
  - [x] Per-line granularity
  - [x] Per-character granularity
- [x] Font layout
  - [x] Click detection
  - [x] Simple wrapping
  - [ ] Wrapping with indentation
  - [ ] No wrapping
  - [ ] Ellipsize
- [x] Font rendering (using swash)
  - [x] Cache results
  - [x] Font hinting
  - [x] Ligatures
  - [x] Color emoji
- [x] Text editing
    - [x] Performance improvements
    - [x] Text selection
    - [x] Can automatically recreate https://unicode.org/udhr/ without errors (see below)
    - [x] Bidirectional selection
    - [ ] Copy/paste
- [x] no_std support (with `default-features = false`)
    - [ ] no_std font loading
    - [x] no_std shaping
    - [x] no_std layout
    - [ ] no_std rendering

The UDHR (Universal Declaration of Human Rights) test involves taking the entire
set of UDHR translations (almost 500 languages), concatenating them as one file
(which ends up being 8 megabytes!), then via the `editor-test` example,
automatically simulating the entry of that file into kalamos per-character,
with the use of backspace and delete tested per character and per line. Then,
the final contents of the buffer is compared to the original file. All of the
106746 lines are correct.

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
