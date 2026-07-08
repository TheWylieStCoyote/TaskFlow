# CLI reference

codemmet is a command-line tool that expands terse Emmet-style abbreviations into code. This page documents every subcommand, its options, and its output. Diagnostics always go to standard error; the expansion text is the only thing written to standard output, so you can safely pipe or capture it.

The available subcommands are:

| Command | Purpose |
| --- | --- |
| `expand` | Expand an abbreviation to code on standard output. |
| `render` | Render a multi-line `.codemmet` source file to code on standard output. |
| `wrap` | Wrap a selection (read from standard input) in an expanded abbreviation. |
| `capture` | Generalize example code into a reusable snippet and write it to a pack. |
| `list` | List every trigger registered for a language. |
| `check` | Validate the user snippet override directory. |
| `lsp` | Run the Language Server Protocol server over standard input and output. |
| `train` | Practice the abbreviation syntax interactively. See the [trainer guide](trainer.md). |

Every command accepts `-h` or `--help`. The top-level command also reports its
version two ways: `-V` prints the short version (`codemmet 0.1.0`), and
`--version` prints the full version including the git commit it was built from
(`codemmet 0.1.0 (1b87ea9)`). A build made with uncommitted changes is marked
`-dirty` (`codemmet 0.1.0 (1b87ea9-dirty)`); a build with no git checkout (for
example, installed from crates.io) shows just `codemmet 0.1.0`.

## expand

Expand an abbreviation and print the resulting code to standard output.

```sh
codemmet expand [OPTIONS] --lang <ID> [ABBR]
```

The `[ABBR]` positional argument is the abbreviation to expand. If you omit it, or pass `-`, codemmet reads the abbreviation from standard input.

| Option | Description |
| --- | --- |
| `--lang <ID>` | Required. The language, given as a language id, a file extension, or a Language Server Protocol language id. For example `rust`, `rs`, `python`, and `py` all work. |
| `--tabstops <keep\|strip>` | Default `strip`. With `keep`, codemmet emits Language Server Protocol snippet syntax such as `${1:placeholder}` and `$0`. With `strip`, codemmet emits plain text: placeholders become their placeholder text and the tabstop markers vanish. |
| `--indent <STR>` | The literal indent unit, either a single tab or a run of spaces. Defaults to the language profile's indent. |
| `--base-indent <STR>` | Default empty. A prefix prepended to every continuation line. The first line is not prefixed, because the caller is assumed to already be at the correct column. |
| `--json` | Print one JSON object instead of plain text. See below. |

With `--tabstops keep`, codemmet emits snippet syntax:

```sh
codemmet expand --lang rust --tabstops keep "fn:main>fori10>pr"
```

```rust
fn main($1) {
    for ${2:i} in 0..10 {
        println!("$0");
    }
}
```

With the default `strip` mode, the same abbreviation becomes plain code:

```sh
codemmet expand --lang rust "fn:main>fori10>pr"
```

```rust
fn main() {
    for i in 0..10 {
        println!("");
    }
}
```

An explicit-argument example:

```sh
codemmet expand --lang rust 'prf:"i={}",i'
```

```rust
println!("i={}", i);
```

### JSON output

With `--json`, codemmet prints a single JSON object with the keys `text`, `cursor`, and `is_snippet`. In `strip` mode, `cursor` is an object `{"line","character"}` where both are 0-based and `character` is a byte offset:

```sh
codemmet expand --lang rust --json "pr"
```

```json
{"cursor":{"character":10,"line":0},"is_snippet":false,"text":"println!(\"\");"}
```

In `keep` mode, `cursor` is `null` and `text` carries the snippet syntax:

```sh
codemmet expand --lang rust --json --tabstops keep "pr"
```

```json
{"cursor":null,"is_snippet":true,"text":"println!(\"$0\");"}
```

The object also carries `requires`: the [import requirements](snippets.md#import-requirements-requires) the expansion collected, each with its raw `spec` and the formatted import `line` (`null` when the language has no import formatter). The CLI has no file to edit, so plain-text output ignores requirements entirely — stdout stays pipeable code:

```sh
codemmet expand --lang rust --json "hmap"
```

```json
{"cursor":{"character":29,"line":0},"is_snippet":false,"requires":[{"line":"use std::collections::HashMap;","spec":"std::collections::HashMap"}],"text":"let mut map = HashMap::new();"}
```

### Reading from standard input

Omit the abbreviation or pass `-` to read it from standard input, which lets you pipe the abbreviation in:

```sh
echo 'fori10>pr' | codemmet expand --lang go -
```

## render (file mode)

Render a **codemmet source file** — a multi-line file of abbreviations — to
code on standard output. Where `expand` is one abbreviation per invocation,
`render` is a whole file per invocation, designed for the append pipeline:

```sh
cat main.rs.codemmet | codemmet >> main.rs
```

Three spellings run the same thing:

```sh
codemmet render main.rs.codemmet     # explicit subcommand
codemmet main.rs.codemmet            # shorthand: a .codemmet positional
cat main.rs.codemmet | codemmet      # bare filter: piped stdin, no arguments
```

| Option | Description |
| --- | --- |
| `[FILE]` | The `.codemmet` source file. Omit it, or pass `-`, to read standard input. |
| `--lang <ID>` | The language, as for `expand`. A `#lang` directive overrides it; the file name's inner extension (`NAME.EXT.codemmet`) is the fallback. An error when none of the three resolves. |
| `--indent <STR>` | Same as for `expand`. |
| `--repeat <NAME=v1,v2,…>` | Seed a collection for data-driven repeats, as for `expand`. A same-name `#repeat` directive overrides it. |
| `--tabstops <keep\|strip>` | Default `strip`: plain code. With `keep`, the whole file is emitted as **one valid LSP snippet document**: every line's tabstops are renumbered across the file in document order (mirrors stay linked within their own line), every `$0` except the last line's demotes to a plain numbered stop — so tabbing walks construct to construct — and exactly one final `$0` survives. |

There is no `--json` and no `--base-indent`: leading whitespace on an
abbreviation line is that line's base indent (see below).

With `--tabstops keep`, two lines that each expand with their own `$1` and
`$0` merge into one navigable snippet (the editor integration uses this to
re-enter snippet mode after a buffer render):

```sh
printf 'fn:main\npr\n' | codemmet render --lang rust --tabstops keep -
```

```text
fn main($1) {
    $2
}
println!("$0");
```

Progressive context is unaffected: the context buffer is always the stripped
(real-code) output, whatever `--tabstops` says. Keep-mode lines are not
right-trimmed — a `$0` alone on an indented line is a cursor position, not
trailing whitespace.

### The `.codemmet` file format

One abbreviation per line. Blank lines pass through as blank output lines
(except before any output exists). Lines starting with `#` are directives or
comments:

- `#lang <id>` — the language, once per file, before the first abbreviation.
- `#repeat NAME=v1,v2,…` — define a collection for data-driven repeats, with
  exactly the `--repeat` flag's syntax (including `value:type` members). It
  applies from its line down, and a re-declaration shadows the earlier one.
- `# a comment` — `#` followed by whitespace (or alone) is a comment,
  dropped from the output. `#word` with an unknown word is a hard error, so
  a misspelled directive never silently disappears.

Leading whitespace on an abbreviation line is used as that line's base
indent — it indents the emitted block textually; it does **not** nest it
inside the previous block (use `>` within one line for structural nesting).
Input may be CRLF; output is always LF. Emitted lines are right-trimmed
(in the default strip mode; keep-mode snippet bodies stay verbatim).

```text
#lang rust
#repeat Msg=Quit,Move,Write

use:std::fmt

enum:Msg>variant*@Msg

impl:Msg>fn:handle
match:self>arm*@Msg
```

### Progressive context

The output rendered so far is the context buffer for later lines, as if you
were typing each abbreviation at the bottom of the file in an editor. On
default builds (with tree-sitter), an enum or struct emitted by an earlier
line feeds a later data-driven repeat — so this needs no `#repeat` header:

```text
#lang rust
enum:Msg>variant*[Quit,Move]

match:m>arm*@Msg
```

```rust
enum Msg {
    Quit,
    Move,
}

match m {
    Quit => ,
    Move => ,
}
```

Explicit data beats inferred data: a `#repeat` (or `--repeat`) collection
shadows a same-name buffer collection. On minimal builds (no tree-sitter),
`#repeat`/`--repeat` work unchanged and buffer-derived repeats degrade to the
usual single-copy fallback.

### Atomic output

The whole file renders to memory first; standard output is written only when
every line succeeded. A failure anywhere prints a diagnostic with the source
name and line number to standard error — `error: main.rs.codemmet:7: …` —
emits **zero bytes** on standard output, and exits non-zero, so a
`>> main.rs` append never leaves a half-written file. Rendering stops at the
first error; exit codes follow the usual conventions (see below), with
`.codemmet` format errors (an unknown directive, a misplaced `#lang`)
counted as parse errors (exit code 1).

In Neovim, the shipped plugin makes `.codemmet` a first-class filetype:
syntax highlighting, completion in the inner language while you author, and
`:CodemmetRenderTo` / render-on-save driving this same `render` command — see
[editor setup](editors.md#authoring-codemmet-files-file-mode).

## wrap

Wrap a selection in an expanded abbreviation. codemmet reads the selection text from standard input, inserts it at the abbreviation's final-cursor position inside the expansion, and prints the result to standard output.

```sh
codemmet wrap [OPTIONS] --lang <ID> --abbr <ABBR>
```

| Option | Description |
| --- | --- |
| `--lang <ID>` | Required. Same meaning as for `expand`. |
| `--abbr <ABBR>` | Required. The abbreviation to wrap the selection in. |
| `--tabstops <keep\|strip>` | Same as for `expand`. |
| `--indent <STR>` | Same as for `expand`. |
| `--base-indent <STR>` | Same as for `expand`. |

There is no `--json` option for `wrap`.

```sh
printf 'let x = compute();\nprintln!("{x}");' | codemmet wrap --lang rust --abbr fori10 --tabstops keep
```

```rust
for ${1:i} in 0..10 {
    let x = compute();
    println!("{x}");$0
}
```

## capture

Turn a chunk of existing code into a reusable, parameterized snippet and write it into a snippet pack. This is the "I have a pattern; let me make more of it" workflow: capture the shape of something you have, then `expand` it back with a different name.

```sh
codemmet capture [OPTIONS] --lang <ID> --name <NAME> [CODE]
```

The `[CODE]` positional argument is the code to capture. If you omit it, or pass `-`, codemmet reads the code from standard input.

| Option | Description |
| --- | --- |
| `--lang <ID>` | Required. Same meaning as for `expand`. |
| `--name <NAME>` | Required. The trigger name to register the captured snippet under. |
| `--description <TEXT>` | Optional description, shown by `list` and in editor completion. |
| `--param <IDENT>` | Force an identifier to become a `${param}`, even if it occurs once. Repeatable. |
| `--literal <IDENT>` | Keep an identifier literal, even if it repeats. Repeatable. |
| `--user` | Write to the user override directory (`--config` or the default) instead of the project pack. |
| `--force` | Overwrite an existing trigger of the same name. |

**Generalization.** codemmet scans the code for identifiers. An identifier that occurs **more than once** and is not a language keyword becomes a parameter (repeated occurrences become a mirror, so they stay in sync); everything else stays literal. Each parameter's default is the original identifier, so expanding the trigger with no arguments reproduces the original code. Matching is whole-token (`foo` and `handle_foo` are different), so use `--param` / `--literal` to correct the guess. The captured body is dedented and re-indented to tabs, so it adapts to any target indentation.

**Where it writes.** Without `--user`, the snippet lands in the `.codemmet/snippets/<lang>.toml` pack at or above the working directory (created under the project's `.git` root if none exists yet). The written pack always passes `codemmet check`. An existing trigger of the same name is an error (exit code 3) unless you pass `--force`.

```sh
printf 'fn make_widget() -> Widget {\n    Widget::default()\n}\n' \
  | codemmet capture --lang rust --name mkwidget --param Widget
```

```text
captured `mkwidget` (1 parameter(s): Widget) into /repo/.codemmet/snippets/rust.toml
```

```sh
codemmet expand --lang rust mkwidget          # reproduces the original
codemmet expand --lang rust mkwidgetGadget    # glued identifier arg → Gadget
```

The glued `Gadget` binds to the `Widget` parameter. `make_widget` is a single token, so it is not touched (whole-token matching):

```rust
fn make_widget() -> Gadget {
    Gadget::default()
}
```

In Neovim, the same workflow is `:CodemmetCapture` on a visual selection (see [editor setup](editors.md)).

## scaffold

Expand a [multi-file snippet](snippets.md#multi-file-snippets) into a file manifest, optionally writing it to disk. The invocation is a single trigger with arguments (no `>`/`+`/`*` chains).

```sh
codemmet scaffold --lang python "djfeature:blog" --json     # print the manifest
codemmet scaffold --lang python "djfeature:blog" --write    # apply it
```

| Flag | Meaning |
| --- | --- |
| `--lang <ID>` | Language id, extension, or LSP id (required). |
| `--root <DIR>` | Base directory for `path` targets (default: the working directory). |
| `--from <FILE>` | The current file, for `self`/sibling (`.cpp`) targets. |
| `--tabstops keep\|strip` | Tabstop handling for the **primary** file (default `strip`; others are always plain). |
| `--indent <STR>` | Indent unit (a tab or a run of spaces); defaults to the language profile. |
| `--json` | Print `[{path, action, content, primary}]` for an editor to apply. |
| `--write` | Apply the manifest: **create** new files (with any `on_create` header), **append** to existing ones. |

With neither `--json` nor `--write`, it prints a human-readable dry run. A section whose `on_exists = "inject"` currently appends (structural injection is a later phase). Exit code 2 on an unrenderable invocation, 3 on a config/filesystem error.

## list

List every trigger registered for a language, sorted, one per line. Each line is the trigger and its description separated by a tab.

```sh
codemmet list [OPTIONS] --lang <ID>
```

| Option | Description |
| --- | --- |
| `--lang <ID>` | Required. Same meaning as for `expand`. |
| `--json` | Print a JSON array of `{"trigger","description"}` objects instead of tab-separated lines. |

```sh
codemmet list --lang rust
```

The plain-text output is one `trigger<TAB>description` per line, sorted by trigger.

## check

Validate the user snippet override directory. codemmet uses the directory given by `--config`, or the default override directory if `--config` is not set.

```sh
codemmet check [OPTIONS]
```

On success, `check` prints a summary and exits with code 0:

```text
ok: N languages, M triggers
```

If the directory contains a `*.toml` file whose name does not match a supported language pack, `check` warns about that file on standard error but still exits 0. A parse error or schema error in a correctly-named file causes an exit code of 3.

`check` also validates every snippet's `requires` specs against its language's import-spec grammar (a malformed spec is a schema error, exit 3), and warns — without failing — when a pack declares `requires` for a language whose import insertion is not implemented yet (JavaScript/TypeScript), since editors silently ignore those.

## lsp

Run the Language Server Protocol server over standard input and output.

```sh
codemmet lsp [OPTIONS]
```

The `--stdio` flag is accepted for compatibility with editor clients, but standard input and output is the only transport. For real editor configuration, see the [editor setup](editors.md) guide.

## The `--config` option and the override directory

The global `--config <DIR>` option sets the snippet override directory, where you can place snippet packs that override or extend the built-in triggers. Its default is the platform config directory plus `codemmet/snippets`:

| Platform | Default override directory |
| --- | --- |
| Linux | `~/.config/codemmet/snippets/` |
| macOS | `~/Library/Application Support/codemmet/snippets/` |
| Windows | `%APPDATA%\codemmet\snippets\` |

Files in this directory are named `<language-id>.toml`, for example `rust.toml` or `go.toml`. Pass `--config <DIR>` to any subcommand to use a different directory. For the full snippet pack schema, see the [custom snippets](snippets.md) guide.

## Project-local packs and `--no-project`

The CLI also auto-loads **project-local** packs: it collects every `.codemmet/snippets/` directory from the working directory up to the filesystem root and merges them at the highest priority (project overrides user overrides builtins), with the nearest pack winning per-trigger conflicts. This lets a repository — including a monorepo with a shared root pack and refining subproject packs — ship its own triggers. Pass the global `--no-project` flag to any subcommand to ignore project packs and use only the user and built-in ones. See [custom snippets](snippets.md) for the full precedence rules (including personal path-mapped packs) and the LSP's opt-in behavior.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success. |
| 1 | Abbreviation parse error, such as malformed syntax like `>pr` — or, in file mode, a `.codemmet` format error such as an unknown directive or a misplaced `#lang`. |
| 2 | Resolution or render error: an unknown trigger, an argument binding failure, or an expansion that is too large. |
| 3 | Configuration or environment error: an unknown language, an invalid `--indent`, or a bad snippet pack. |

When a trigger is unknown, codemmet writes an error to standard error, suggesting the nearest trigger by edit distance, and exits with code 2:

```sh
codemmet expand --lang rust "forp"
```

```text
error: cannot expand `forp`: unknown trigger `forp` (did you mean `fore`?)
```

## Standard input and piping

Every command writes only the expansion text to standard output; all diagnostics go to standard error. The `expand` command reads its abbreviation from standard input when you omit it or pass `-`, `render` reads its source file the same way, and `wrap` always reads its selection from standard input. Bare `codemmet` with piped standard input and no arguments is the `render` filter (`cat main.rs.codemmet | codemmet >> main.rs`); run interactively with no arguments, it prints help. This makes codemmet easy to use in pipelines and from editors.

## See also

- [abbreviation syntax](syntax.md) for the full abbreviation grammar.
- [language reference](languages.md) for the triggers and behaviors of each language.
- [editor setup](editors.md) to run the Language Server Protocol server in your editor.
- [tutorials](tutorials.md) for hands-on examples.
