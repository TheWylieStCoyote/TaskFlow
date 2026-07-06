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
   Then browse the **[examples cookbook](examples.md)** for longer,
   real-world abbreviations that combine several operators at once.
3. **[Editor setup](editors.md)** — use codemmet as you type, in Neovim or any
   LSP-capable editor.

## Reference

- **[Abbreviation syntax](syntax.md)** — the mini-grammar: triggers, arguments,
  and the `>` `+` `*` `^` `()` composition operators.
- **[Language reference](languages.md)** — every trigger for each of the seven
  languages, with descriptions.
- **[CLI reference](cli.md)** — the `expand`, `wrap`, `list`, `check`, and `lsp`
  subcommands, their flags, and exit codes.
- **[Examples cookbook](examples.md)** — longer, real-world abbreviations that
  combine nesting, siblings, repeats, and climb-up across languages.
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
