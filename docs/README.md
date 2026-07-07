# codemmet documentation

**codemmet** expands terse, Emmet-style abbreviations into real code. Type
`fn:main>fori10>pr` in a Rust buffer and get a `main` function containing a
for-loop containing a `println!`, with tabstops to jump through:

```rust
fn main() {
    for i in 0..10 {
        println!("");
    }
}
```

It ships as a single binary that is both a command-line tool and a language
server, plus a Neovim plugin. Seven languages are supported out of the box:
Rust, Python, C, C++, JavaScript, TypeScript, and Go.

## Start here

New to codemmet? Read these in order:

1. **[Installation](installation.md)** — build the binary and put it on your PATH.
2. **[Tutorials](tutorials.md)** — a hands-on walkthrough from your first
   expansion to wrapping existing code and scripting.
   Then browse the **[examples](examples/README.md)** for longer, real-world
   abbreviations, with a page per language.
3. **[Editor setup](editors.md)** — use codemmet as you type, in Neovim or any
   LSP-capable editor.
4. **[Trainer](trainer.md)** — learn the syntax interactively with
   `codemmet train`: guided lessons, drills, and spaced repetition.

## Reference

- **[Abbreviation syntax](syntax.md)** — the mini-grammar: triggers, arguments,
  and the `>` `+` `*` `^` `()` composition operators.
- **[Language reference](languages.md)** — every trigger for each of the seven
  languages, with descriptions.
- **[Context-aware & data-driven expansion](context-codegen.md)** — how codemmet
  reads the surrounding code: scope-specific variants (`fn` in `impl` gains
  `&self`) and repeats driven by enum variants, struct fields, or in-scope lists
  (`match:Msg>arm*`).
- **[CLI reference](cli.md)** — the `expand`, `wrap`, `list`, `check`, and `lsp`
  subcommands, their flags, and exit codes.
- **[Trainer](trainer.md)** — the `train` subcommand: modes, keys, spaced
  repetition, and progress files.
- **[Examples](examples/README.md)** — longer, real-world abbreviations that
  combine nesting, siblings, repeats, and climb-up, with a page per language.
- **[Custom snippets](snippets.md)** — author your own triggers and override or
  disable the built-in ones with TOML snippet packs.
- **[Troubleshooting](troubleshooting.md)** — fixes for the most common issues.

## The idea in one minute

You type an **abbreviation** — a **trigger** (a short keyword like `fori`)
optionally followed by **arguments** and joined to other triggers with
**composition operators**. codemmet resolves it against the active language's
**snippet pack** and produces an **expansion**: the generated code, with
**tabstops** (`$1`, `${1:placeholder}`, `$0`) marking the spots you fill in.

| You type | You get |
| --- | --- |
| `fn:main` | a `main` function |
| `fori10` | a `for` loop counting to 10 |
| `fn:main>fori10>pr` | the loop, and a `println!`, nested inside `main` |
| `if+el` | an `if` block followed by an `else` block |
| `match:cmd>arm*3` | a `match` with three arms |

See the [abbreviation syntax](syntax.md) reference for the full grammar and the
[tutorials](tutorials.md) to try it yourself.
