# CLAUDE.md

Guidance for Claude Code working in this repository.

## What this is

**kalamos** — RTL-first text shaping, bidirectional layout, and rasterization.
Published on crates.io as `kalamos`; the name is Greek (*κάλαμος*, "reed pen").
Its first consumer is `abdu-egui-ui` (at `../abdu-egui-ui`).

It is a **hard fork of cosmic-text**, not a patch series on top of one. The RTL
correctness work had nowhere to go: pop-os/cosmic-text's PR template requires
disclosing AI-assisted contributions and says such PRs may be closed without
comment. Three fixes were opened upstream and withdrawn. The fork is the answer
and it is permanent.

**`upstream` is provenance, not authority.** The `upstream` remote
(pop-os/cosmic-text) exists so lineage stays checkable and a rebase stays
possible. That is its whole job. *"That's what upstream does"* justifies nothing
here — this codebase carries real upstream bugs and inherited sloppiness, and
several were fixed only after someone stopped treating the original as a
reference implementation. Do not open PRs upstream. Do not defer to it.

Remotes: `origin` → abdu-benayad/kalamos (the home) · `upstream` → pop-os
(provenance) · `cosmic-text-rtl` → the old abdu-benayad/cosmic-text, kept only
because the retired `cosmic-text-rtl` crate still needs a tombstone release.

**Commit identity is `abdulbari Ben ayad <abdu.benayad@gmail.com>` with no
Claude co-author trailer.** The repo-local git config is already set, so a plain
`git commit` is correct — do not add a trailer, and do not override the author.

## Why the fork exists — the RTL work

Three behavioural fixes over stock cosmic-text, none of them upstream:

- **`efd71e23`** — `cursor_glyph` honors `Affinity::Before` at run boundaries.
  Stock always painted the After side, so the caret lied at every bidi seam.
- **`71b0a028`** — `hit()` before-line detection for aligned lines.
- **`dd319ad4`** — the fresh-line sentinel aliased span 0 in *width and overflow*
  math. Upstream's `4fe1195e` fixed the visible glyph-drop half; this is the
  residual, and it is **not** in the published `cosmic-text-rtl 0.19.0`.

`tests/leading_run_survives_width_bound.rs` and `tests/ellipsize_incongruent.rs`
pin these. Read the header comments before touching either — they record which
test guards which fix, and one of them deliberately does *not* guard the commit
you would assume.

## Commands

```bash
bash ci.sh                      # THE gate — run this, not a hand-rolled subset
cargo test --all-features       # tests alone
cargo doc --all-features --no-deps
```

`ci.sh` builds and lints every feature combination, then runs one
`clippy --all-features --all-targets --no-deps -- -D warnings` and the suite.
It needs `rustup target add thumbv8m.main-none-eabihf` for the no_std lane
(it installs it itself, but the target must be fetchable).

**Do not invent a narrower gate and call it green.** The `--all-targets` line is
load-bearing: for years the only lint was lib-scoped, so tests and benches drifted
completely unchecked and accumulated 22 errors nobody saw. A gate that skips
targets is how that happened.

## Traps

**`src/swash.rs` is not swash.** It is this crate's ~300-line adapter *around* the
independent crates.io `swash`, named after the dependency it wraps. It reads like
vendored code and is not. `swash` is optional and does render+scale only.

**Two font stacks are linked at once.** swash (Brokaw) for rasterization, and
skrifa + harfrust (Linebender) for font access and shaping. They overlap. Whether
swash still earns its place is an open design question, not a settled one.

**`std` and `no_std` are not mutually exclusive to cargo.** `--all-features`
enables *both*, which is a contradictory combination the code has to tolerate.
Gate `core_maths::CoreFloat` imports on `#[cfg(not(feature = "std"))]` — the house
pattern in `render.rs`, `layout.rs`, `buffer.rs` and `shape.rs`. Inventing a
different spelling for one file is how a dead import survived.

**Binary fixtures are git-lfs** (`*.png`, `*.ttf` — see `.gitattributes`). A clean
`git status` does **not** mean the objects are present: when the file on disk *is*
the pointer text and the index records that same pointer, git reports clean while
the real bytes are missing. Tests read `fonts/NotoSansArabic.ttf` directly, so a
pointer file fails them confusingly. `git lfs fetch --all upstream` if in doubt —
and a first push to any new remote needs every historical object, not just the
checkout's.

**`#[expect]` is wrong in `tests/common/mod.rs`.** That module compiles into
several test binaries and none uses every helper, so `expect(dead_code)` would
fire `unfulfilled_lint_expectation` in the ones that do. It is the rare case where
`allow` is correct; the comment there says so.

**`Cargo.lock` is gitignored** (library convention) — it is not part of any commit.

**The published crate excludes the binary fixtures** (`fonts/*`, `screenshots/*`,
`tests/images/*`). The crates.io artifact is source-only; clone the repo to run
the suite.

## Conventions

- The crate `deny`s `clippy::unwrap_used`, `missing_debug_implementations` and
  more. These live as **inner attributes at `src/lib.rs:56-87`**, not a `[lints]`
  table — deliberately, since a package-level `[lints]` would push
  `deny(unwrap_used)` onto tests and benches where `unwrap()` is legitimate.
- Suppressions carry a `reason`. Prefer `#[expect]`; use `allow` only where
  `expect` is technically wrong (see above) and say why inline.
- `cargo fmt` before committing — `ci.sh` checks it first and fails fast.
- Every commit leaves `ci.sh` green. There is no PR to catch a broken one.

## Current state / open work

- **crates.io `0.1.0` predates most of this repo** — it was published before the
  example retirement, the lint work and the README rewrite. A `0.1.1` syncs it.
- **GitHub Actions was red on workflow permissions, not code** — fixed in the
  tree, unverified until the next push. `rust.yml` ran `actions-rs/clippy-check@v1`
  (archived 2023, needs `checks: write`) and a `pages.yml` deployed rustdoc
  (needs `contents: write`); new repos default `GITHUB_TOKEN` to read-only, so
  both failed before `ci.sh` ever started. The clippy step is gone — `ci.sh`
  already lints strictly — `pages.yml` is deleted since docs.rs serves the docs,
  and the workflow now declares `permissions: contents: read` explicitly.
- **Zero examples.** Five inherited demos on winit 0.29 / orbclient were retired;
  nothing replaced them. An RTL-first example written *for* this library is the
  gap.
- **UDHR has no harness.** The README's UDHR result is upstream provenance —
  the harness that produced it was a windowed orbclient demo and went with the
  examples. Re-landing it as a headless test under `tests/` is open work.
- **17 unresolved intra-doc links**, which is why `test.sh` does not yet run
  rustdoc under `-D warnings`.
- **`CHANGELOG.md` still ends at cosmic-text `0.19.0` (2026-04-22)** and has no
  kalamos entry.
