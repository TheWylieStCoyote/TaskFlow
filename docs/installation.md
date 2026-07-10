# Installation

This page shows you how to install codemmet — from a prebuilt binary or from source — put it onto your `PATH`, and verify that it works. codemmet is a Rust command-line tool and Language Server Protocol server that expands terse Emmet-style abbreviations into code for Rust, Python, C, C++, JavaScript, TypeScript, and Go.

## Prebuilt binaries

The quickest path: every [tagged release](https://github.com/TheWylieStCoyote/codemmet/releases) ships a `codemmet` binary for Linux (`x86_64`), macOS (`x86_64` and `aarch64`), and Windows (`x86_64`), built by the [release workflow](../.github/workflows/release.yml). Download the archive for your platform, extract it, and move `codemmet` somewhere on your `PATH` (e.g. `/usr/local/bin` or `~/.local/bin`). These binaries include the tree-sitter grammars, so they need no toolchain to run. Then skip to [Verify the install](#verify-the-install).

To build it yourself instead, read on.

## Prerequisites

You need a recent stable Rust toolchain. codemmet uses Rust edition 2024, so you need Rust 1.85 or newer. Install it with [rustup](https://rustup.rs/) if you do not already have it.

The default build compiles the tree-sitter grammars for every supported language directly into the binary. Those grammars include C sources, so you also need a working C toolchain (a C compiler such as `gcc` or `clang`) to build the default configuration. If you do not have a C compiler, use the minimal text-only build described below, which does not need one.

## Build from source

Clone the repository and build the release binary:

```sh
git clone https://github.com/TheWylieStCoyote/codemmet
cd codemmet
cargo build --release
```

Or install straight from the repository without cloning:

```sh
cargo install --git https://github.com/TheWylieStCoyote/codemmet codemmet
```

This produces the binary at `target/release/codemmet`. You can run it directly from that path, or install it onto your `PATH` as shown next.

## Install onto your PATH

To install the binary so you can run `codemmet` from anywhere, use `cargo install`:

```sh
cargo install --path crates/codemmet
```

This compiles the release build and places the `codemmet` binary into `~/.cargo/bin`, which rustup adds to your `PATH`.

## Minimal text-only build

The tree-sitter grammars are enabled by the default-on `tree-sitter` cargo feature. If you want a smaller build, or you do not have a C compiler available, you can build without them:

```sh
cargo build --release -p codemmet --no-default-features
```

This build works without any C grammars, so it needs no C toolchain. In exchange, it loses the syntax-aware and context-aware features that depend on tree-sitter, such as inserting `&self` inside an `impl` block and suppressing completions inside string literals. The core abbreviation expansion still works for every language. See the [language reference](languages.md) and [editor setup](editors.md) for details on which behaviors depend on tree-sitter.

## Verify the install

Confirm the version:

```sh
codemmet --version
```

Then run a quick expansion to confirm the tool works end to end:

```sh
codemmet expand --lang rust "fn:main"
```

If codemmet is installed correctly, this prints a small Rust `main` function to standard output.

## Optional developer tools

If you plan to build on or contribute to codemmet, a few optional tools are used in development: `just` (task runner), `cargo-deny` (dependency and license checks), `stylua` (Lua formatting), and `lefthook` (git hooks). These are optional and are not required to install or use codemmet.

## Next steps

- Read the [CLI reference](cli.md) to learn every subcommand and option.
- Set up your editor with the [editor setup](editors.md) guide.
- Work through the [tutorials](tutorials.md) to learn the abbreviation syntax hands-on.
