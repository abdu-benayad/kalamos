#!/usr/bin/env bash

set -ex

echo Run CI script
./ci.sh

# Not yet RUSTDOCFLAGS="-D warnings": 18 inherited rustdoc warnings still to
# clear before that gate can go strict.
echo Build documentation
cargo doc --all-features --no-deps
