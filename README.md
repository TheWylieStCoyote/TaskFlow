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

**Status: all planned milestones (M0–M4) complete.** Supported languages:
Rust, Python, C, C++ (inherits C), JavaScript, TypeScript (inherits
JavaScript), and Go — ~250 builtin triggers. Tree-sitter powers
context-aware expansion (`fn` inside an `impl` block expands with `&self`,
`def` in a Python class gets `self`), completion suppression inside strings
and comments, and statement-boundary snapping for wrap selections. See
[PLAN.md](PLAN.md) for the design document.

## Neovim

Install with lazy.nvim (builds the binary on install):

```lua
{
  "<your-github>/codemmet",
  build = "cargo build --release",
  ft = { "rust", "python", "c", "cpp", "javascript", "typescript", "go" },
  opts = {
    cmd = { "codemmet", "lsp" },              -- or the target/release path
    keymaps = { expand = "<A-e>", wrap = "<leader>cw" },
  },
}
```

Abbreviations appear as snippet completions in any completion setup
(nvim-cmp, blink.cmp, built-in). `<A-e>` expands the abbreviation before
the cursor explicitly; select lines and `<leader>cw` wraps them in a
construct (`fori10`, `if`, `try`, …). `:CodemmetExpand` and
`:CodemmetWrap` are the command equivalents. Requires Neovim ≥ 0.10.

Any other LSP-capable editor works too: point it at `codemmet lsp` and
wire `codemmet.expand` / `codemmet.wrap` to keybindings.

## CLI

```sh
codemmet expand --lang rust "fn:main>fori10>prf:\"i={}\",i"
codemmet expand --lang python --tabstops keep "def:handler>try"
echo "class:Point>init" | codemmet expand --lang py -
codemmet list --lang rust          # all triggers with descriptions
codemmet check                     # validate user snippet overrides
```

`--tabstops keep` emits LSP snippet syntax (`${1:placeholder}`, `$0`);
the default `strip` emits plain text (`--json` adds the cursor position).
User overrides live in `~/.config/codemmet/snippets/<lang>.toml` and can
override or `disabled = true` any builtin trigger.

## Abbreviation syntax at a glance

| Operator | Meaning              | Example                  |
| -------- | -------------------- | ------------------------ |
| `word`   | trigger + glued args | `fori10`                 |
| `:`, `,` | explicit args        | `fn:main`, `td:"a, b"`   |
| `>`      | nest as child        | `fn:main>fori10`         |
| `+`      | sibling statement    | `if+el`                  |
| `*`      | repeat               | `match:x>arm*3`          |
| `()`     | group                | `(fori10>pr)+ret`        |
| `^`      | climb up a level     | `fn:main>fori10>pr^ret`  |

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
