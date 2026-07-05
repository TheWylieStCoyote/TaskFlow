# codemmet

Emmet-style abbreviation expansion for programming languages.

Type a terse abbreviation, get structured code with tabstops:

```
fn:main>fori10>pr        (in a Rust buffer)
```

```rust
fn main() {
    for i in 0..10 {
        println!("|");
    }
}
```

Planned language support: Rust, Python, C, C++, JavaScript, TypeScript, Go.
Delivered as one binary that is both a CLI (`codemmet expand`, `wrap`, `list`,
`check`) and an LSP server (`codemmet lsp`), plus a Neovim plugin in this repo.
Later phases use tree-sitter for context-aware expansion (e.g. `fn` inside an
`impl` block gains `&self`) and wrap-with-abbreviation.

**Status: early scaffolding.** See [PLAN.md](PLAN.md) for the full design and
milestone roadmap.

## Abbreviation syntax at a glance

| Operator | Meaning              | Example                  |
| -------- | -------------------- | ------------------------ |
| `word`   | trigger + glued args | `fori10`                 |
| `:`, `,` | explicit args        | `fn:main`, `td:"a, b"`   |
| `>`      | nest as child        | `fn:main>fori10`         |
| `+`      | sibling statement    | `if+el`                  |
| `*`      | repeat               | `match:x>arm*3`          |
| `()`     | group                | `(fori10>pr)+ret`        |

## Development

Requires stable Rust (edition 2024). Optional tools: [`just`](https://github.com/casey/just),
[`lefthook`](https://github.com/evilmartians/lefthook), [`stylua`](https://github.com/JohnnyMorganz/StyLua),
[`cargo-deny`](https://github.com/EmbarkStudios/cargo-deny), [`cargo-insta`](https://insta.rs).

```sh
lefthook install   # once per clone: fmt on commit, clippy+tests on push
just lint          # what CI enforces
just test
just ci            # the full CI suite locally
```

Workspace layout:

- `crates/codemmet-core` — abbreviation parser, snippet IR, renderer, TOML
  snippet loader. No tree-sitter, no async, no CLI deps.
- `crates/codemmet-tsctx` — tree-sitter `ContextProvider` (phases M3/M4).
- `crates/codemmet` — the `codemmet` binary: CLI + LSP server.
- `lua/`, `plugin/` — Neovim plugin (phase M2).

## License

MIT or Apache-2.0, at your option.
