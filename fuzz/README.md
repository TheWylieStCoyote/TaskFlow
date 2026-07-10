# Fuzzing

Coverage-guided fuzz targets for the parser and the expansion pipeline, using
[`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz) (libFuzzer). This crate
is detached from the main workspace and only builds under `cargo-fuzz`, which
needs a nightly toolchain and the sanitizer runtime — it is not part of normal
`cargo build`/CI.

For fast, in-CI generative testing that runs on stable, see the property tests
in [`crates/codemmet-core/tests/fuzz.rs`](../crates/codemmet-core/tests/fuzz.rs).

## Run

```sh
cargo install cargo-fuzz          # one-time
cargo +nightly fuzz run parse     # fuzz the abbreviation parser
cargo +nightly fuzz run expand    # fuzz the full expand pipeline
cargo +nightly fuzz run filefmt   # fuzz the .codemmet line classifier
```

Or a bounded smoke run of all targets via the justfile: `just fuzz` (default
100k iterations each), or `just fuzz 1000000` for longer.

Targets:

- **`parse`** — `codemmet_core::abbr::parse` on arbitrary UTF-8.
- **`expand`** — `Engine::expand` (Rust pack) on arbitrary UTF-8, both tabstop
  modes.
- **`filefmt`** — `codemmet_core::filefmt::lines` (the `.codemmet` file-mode
  line classifier) on arbitrary UTF-8.

The invariant for all of them is simply: never panic.
