#!/usr/bin/env bash
# Build the playground WebAssembly module: crates/codemmet-wasm compiled for
# wasm32-unknown-unknown, size-optimized, copied next to index.html. No
# wasm-bindgen toolchain required — the crate exports a tiny hand-rolled C-ABI
# (see crates/codemmet-wasm/src/lib.rs). Regenerate with `just playground`.
set -euo pipefail
cd "$(dirname "$0")/.."

rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true

# Size flags are passed per-invocation so the shared release profile (used by
# the native binary) is untouched.
cargo build --release -p codemmet-wasm --target wasm32-unknown-unknown \
  --config 'profile.release.opt-level="z"' \
  --config 'profile.release.lto=true' \
  --config 'profile.release.codegen-units=1' \
  --config 'profile.release.strip=true' \
  --config 'profile.release.panic="abort"'

cp target/wasm32-unknown-unknown/release/codemmet_wasm.wasm playground/codemmet.wasm
echo "wrote playground/codemmet.wasm ($(du -h playground/codemmet.wasm | cut -f1))"
