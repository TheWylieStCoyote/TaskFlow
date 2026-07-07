# Contributing to codemmet

Thanks for your interest! codemmet is a Rust workspace — a CLI + LSP server, a
tree-sitter context provider, a trainer, and a Neovim plugin. This guide covers
the practical bits; the [design doc](PLAN.md) explains how it all fits together.

## Getting set up

You need a Rust toolchain (the version is pinned in
[`rust-toolchain.toml`](rust-toolchain.toml)) and a C compiler for the bundled
tree-sitter grammars. Then:

```sh
git clone https://github.com/TheWylieStCoyote/codemmit
cd codemmit
cargo build
cargo test
```

Optional but recommended: [`just`](https://github.com/casey/just) (task
runner), [`lefthook`](https://github.com/evilmartians/lefthook) (git hooks),
[`cargo-deny`](https://github.com/EmbarkStudios/cargo-deny), and
[`stylua`](https://github.com/JohnnyMorganz/StyLua) for the Lua plugin. Run
`lefthook install` once per clone to get formatting on commit and clippy/tests
on push.

## The quality bar (what CI enforces)

Before opening a PR, run what CI runs:

```sh
just ci   # fmt --check, clippy -D warnings, tests, release + minimal builds, cargo deny
```

or the pieces individually with `just fmt`, `just lint`, `just test`. Specifically:

- **rustfmt** — `cargo fmt --all` must leave no changes.
- **clippy** — pedantic + nursery, `-D warnings`, `--all-features --all-targets`.
- **tests** — `cargo test --all-features`. The insta snapshot suite under
  `crates/codemmet-core` is the living spec; review changes with `just snap`.
- **minimal build** — `cargo build -p codemmet --no-default-features` must keep
  compiling (the tree-sitter-free path).
- **cargo-deny** — advisories, licenses, and sources.

## Where things live

- `crates/codemmet-core` — abbreviation parser, snippet IR, renderer, TOML
  loader. No tree-sitter, no async, no CLI deps.
- `crates/codemmet-tsctx` — the tree-sitter `ContextProvider` (context,
  wrap-snapping, collection extraction, code-gen edits). All per-language
  node-kind knowledge lives here.
- `crates/codemmet-train` — the pure trainer state machine.
- `crates/codemmet` — the `codemmet` binary: CLI + LSP server + trainer front ends.
- `crates/codemmet-wasm` — the browser playground's C-ABI (`publish = false`).
- `lua/`, `plugin/` — the Neovim plugin.
- `docs/` — user documentation.

Adding a language means adding a snippet pack under
`crates/codemmet-core/snippets/` and one `LangSpec` in `codemmet-tsctx`.

## Pull requests

- Branch off `master`; keep a PR to one concern.
- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/)
  (`feat:`, `fix:`, `docs:`, `chore:`, `test:`, `ci:`) — the history is written
  this way.
- Add or update tests for behavior changes; snapshot changes should be
  intentional and reviewed.
- Update the relevant docs and the `[Unreleased]` section of
  [`CHANGELOG.md`](CHANGELOG.md).

## License

By contributing, you agree that your contributions are dual-licensed under the
[MIT](LICENSE-MIT) and [Apache-2.0](LICENSE-APACHE) licenses, matching the rest
of the project.
