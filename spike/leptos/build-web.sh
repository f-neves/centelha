#!/usr/bin/env bash
# Gera dist/: HTML estatico com marcadores de hidratacao + wasm do cliente.
set -euo pipefail
cd "$(dirname "$0")"
export PATH="$USERPROFILE/.cargo/bin:$PATH" CARGO_BUILD_JOBS=6
cargo run -p ssg --release
cargo build -p card --lib --target wasm32-unknown-unknown --release --features entry --target-dir target-web
wasm-bindgen --target web --no-typescript --out-dir dist/pkg target-web/wasm32-unknown-unknown/release/card.wasm
tools/binaryen-version_133/bin/wasm-opt.exe -Oz --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals --enable-reference-types --enable-multivalue -o dist/pkg/card_bg.wasm dist/pkg/card_bg.wasm
