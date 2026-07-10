# Development commands. Run `just --list` for an overview.

# Format Rust and Lua sources
fmt:
    cargo fmt --all
    if [ -d lua ]; then stylua lua/ plugin/; fi

# Lint everything the way CI does
lint:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    if [ -d lua ]; then stylua --check lua/ plugin/; fi

# Run the test suite
test:
    cargo test --workspace --all-features

# Review pending insta snapshots
snap:
    cargo insta review

# Smoke-run the coverage-guided fuzz targets (needs nightly + `cargo install
# cargo-fuzz`). Pass a run count, e.g. `just fuzz 1000000` for a longer session.
fuzz runs="100000":
    cd fuzz && cargo +nightly fuzz run parse   -- -runs={{runs}}
    cd fuzz && cargo +nightly fuzz run expand  -- -runs={{runs}}
    cd fuzz && cargo +nightly fuzz run filefmt -- -runs={{runs}}

# Regenerate the README demo GIF (needs ffmpeg + a DejaVu Sans Mono font)
demo:
    bash demo/render.sh

# Build the browser playground's WebAssembly module
playground:
    bash playground/build.sh

# Serve the playground locally at http://localhost:8000
playground-serve: playground
    python3 -m http.server -d playground 8000

# Everything CI runs, locally
ci: lint test
    cargo build --workspace --release
    cargo build -p codemmet --no-default-features
    cargo deny check
