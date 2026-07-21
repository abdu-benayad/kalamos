#!/usr/bin/env bash

function build {
    cargo build --release "$@"
    cargo clippy --no-deps "$@" -- -D warnings
}

set -ex

echo Check formatting
cargo fmt --check

echo Build with default features
build

echo Install target for no_std build
# This is necessary because Rust otherwise may silently use std regardless.
rustup target add thumbv8m.main-none-eabihf

echo Build with only no_std feature
build --no-default-features --features no_std --target thumbv8m.main-none-eabihf

echo Build with only std feature
build --no-default-features --features std

echo Build with only std and swash features
build --no-default-features --features std,swash

echo Build with only std and syntect features
build --no-default-features --features std,syntect

echo Build with only std and vi features
build --no-default-features --features std,vi

echo Build with all features
build --all-features

echo Lint every target, warnings denied
# The per-feature lints above build the library only. Tests and benches are
# separate targets, so nothing above ever compiled them under clippy and they
# drifted unchecked. This is the gate that keeps them honest.
cargo clippy --all-features --all-targets --no-deps -- -D warnings

echo Run tests
cargo test --all-features
