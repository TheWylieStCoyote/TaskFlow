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

# Everything CI runs, locally
ci: lint test
    cargo build --workspace --release
    cargo build -p codemmet --no-default-features
    cargo deny check
