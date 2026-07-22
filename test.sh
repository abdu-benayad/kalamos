#!/usr/bin/env bash

set -ex

echo Run CI script
./ci.sh

echo Build documentation
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps
