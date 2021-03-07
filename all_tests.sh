#!/usr/bin/env sh

set -e

# This is the same list of checks that the Github workflow does.
cargo test -- --test-threads=1
cargo miri test -- --test-threads=1
cargo clippy

RUSTFLAGS="-C debug-assertions=n" cargo test --release -- --test-threads=1
RUSTFLAGS="-C debug-assertions=n" cargo miri test --release -- --test-threads=1
RUSTFLAGS="-C debug-assertions=n" cargo clippy --release
