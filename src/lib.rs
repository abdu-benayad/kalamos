// SPDX-License-Identifier: MIT OR Apache-2.0

//! # COSMIC Text
//!
//! This library provides advanced text handling in a generic way. It provides abstractions for
//! shaping, font discovery, font fallback, layout, rasterization, and editing. Shaping utilizes
//! harfrust, font discovery utilizes fontdb, and the rasterization is optional and utilizes
//! swash. The other features are developed internal to this library.
//!
//! It is recommended that you start by creating a [`FontSystem`], after which you can create a
//! [`Buffer`], provide it with some text, and then inspect the layout it produces. At this
//! point, you can use the `SwashCache` to rasterize glyphs into either images or pixels.
//!
//! ```
//! use kalamos::{Attrs, Color, FontSystem, SwashCache, Buffer, Metrics, Shaping};
//!
//! // A FontSystem provides access to detected system fonts, create one per application
//! let mut font_system = FontSystem::new();
//!
//! // A SwashCache stores rasterized glyphs, create one per application
//! let mut swash_cache = SwashCache::new();
//!
//! // Text metrics indicate the font size and line height of a buffer
//! let metrics = Metrics::new(14.0, 20.0);
//!
//! // A Buffer provides shaping and layout for a UTF-8 string, create one per text widget
//! let mut buffer = Buffer::new(&mut font_system, metrics);
//!
//! // Borrow buffer together with the font system for more convenient method calls
//! let mut buffer = buffer.borrow_with(&mut font_system);
//!
//! // Attributes indicate what font to choose
//! let attrs = Attrs::new();
//!
//! // Set size and text
//! buffer.set_size(Some(80.0), Some(25.0));
//! buffer.set_text("Hello, Rust! 🦀\n", &attrs, Shaping::Advanced, None);
//!
//! // Inspect the output runs
//! for run in buffer.layout_runs() {
//!     for glyph in run.glyphs.iter() {
//!         println!("{:#?}", glyph);
//!     }
//! }
//!
//! // Create a default text color
//! let text_color = Color::rgb(0xFF, 0xFF, 0xFF);
//!
//! // Draw the buffer (for performance, instead use SwashCache directly)
//! buffer.draw(&mut swash_cache, text_color, |x, y, w, h, color| {
//!     // Fill in your code here for drawing rectangles
//! });
//! ```

// Not interested in these lints
#![allow(clippy::new_without_default)]
// Inherited debt, allowed with eyes open
//
// 212 sites (measured 2026-07 via --force-warn). Overflows are only checked in
// debug builds; paydown is per-module and opportunistic, during refactors that
// touch the arithmetic anyway — not a bulk checked_*/saturating_* sweep, which
// would bury the few real overflow risks under hundreds of impossible ones.
#![allow(clippy::arithmetic_side_effects)]
// Out-of-bounds indexing is a hidden panic path. The crate-wide gate is deny;
// the six modules still carrying inherited index sites are `allow`ed at their
// `mod` items below (123 sites total, measured 2026-07 via --force-warn with
// --all-targets: in-crate #[cfg(test)] modules count, tests/ does not, and
// sites carrying their own per-site #[expect] proof are not debt), so
// every clean module — and every new one — is born protected. A blind
// `x[i]` → `.get(i)` sweep would trade silently-correct hot-path code for
// noisy panic paths or bogus Options; sites get replaced per-module when
// their surrounding logic is redesigned, and each module's allow is removed
// when it reaches zero. The module attributes are `allow`, not `expect`,
// deliberately: an `expect` would go red on the unrelated commit that happens
// to pay off a module's last site mid-refactor.
#![deny(clippy::indexing_slicing)]
// Soundness issues
//
// Dereferencing unaligned pointers may be undefined behavior
#![deny(clippy::cast_ptr_alignment)]
// Avoid panicking without information about the panic — and even with it, a
// panic is a last resort: every expect() carries an #[expect] with the local
// invariant that makes the panic unreachable
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
// A library never panics on purpose, ships no scaffolding, and does not write
// to a terminal it doesn't own (rasterization diagnostics go through `log`)
#![deny(clippy::panic)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::print_stdout)]
#![deny(clippy::print_stderr)]
// Ensure all types have a debug impl
#![deny(missing_debug_implementations)]
// This is usually a serious issue - a missing import of a define where it is interpreted
// as a catch-all variable in a match, for example
#![deny(unreachable_patterns)]
// Ensure that all must_use results are used
#![deny(unused_must_use)]
// Style issues
//
// Documentation not ideal
#![warn(clippy::doc_markdown)]
// Document possible errors
#![warn(clippy::missing_errors_doc)]
// Document possible panics
#![warn(clippy::missing_panics_doc)]
// Ensure semicolons are present
#![warn(clippy::semicolon_if_nothing_returned)]
// Ensure numbers are readable
#![warn(clippy::unreadable_literal)]
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

#[cfg(not(any(feature = "std", feature = "no_std")))]
compile_error!("Either the `std` or `no_std` feature must be enabled");

pub use self::attrs::*;
mod attrs;

pub use self::bidi_para::*;
mod bidi_para;

pub use self::buffer::*;
#[allow(
    clippy::indexing_slicing,
    reason = "inherited index debt: 14 sites (2026-07)"
)]
mod buffer;

pub use self::buffer_line::*;
mod buffer_line;

pub use self::cached::*;
mod cached;

pub use self::glyph_cache::*;
mod glyph_cache;

pub use self::cursor::*;
mod cursor;

pub use self::edit::*;
#[allow(
    clippy::indexing_slicing,
    reason = "inherited index debt: 45 sites (2026-07) — editor 16, vi 19, syntect 7, mod 3"
)]
mod edit;

pub use self::font::*;
#[allow(
    clippy::indexing_slicing,
    reason = "inherited index debt: 12 sites (2026-07) — fallback 9, cache 3 (2 in its test module)"
)]
mod font;

pub use self::layout::*;
mod layout;

pub use self::line_ending::*;
mod line_ending;

pub use self::render::*;
#[allow(
    clippy::indexing_slicing,
    reason = "inherited index debt: 1 site (2026-07)"
)]
mod render;

pub use self::shape::*;
#[allow(
    clippy::indexing_slicing,
    reason = "inherited index debt: 46 sites (2026-07); 3 more carry per-site #[expect] proofs"
)]
mod shape;

pub use self::shape_run_cache::*;
mod shape_run_cache;

#[cfg(feature = "swash")]
pub use self::swash::*;
#[cfg(feature = "swash")]
#[allow(
    clippy::indexing_slicing,
    reason = "inherited index debt: 5 sites (2026-07)"
)]
mod swash;

mod math;

type BuildHasher = core::hash::BuildHasherDefault<rustc_hash::FxHasher>;

#[cfg(feature = "std")]
type HashMap<K, V> = std::collections::HashMap<K, V, BuildHasher>;
#[cfg(not(feature = "std"))]
type HashMap<K, V> = hashbrown::HashMap<K, V, BuildHasher>;
