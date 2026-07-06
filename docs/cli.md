# CLI reference

codemmet is a command-line tool that expands terse Emmet-style abbreviations into code. This page documents every subcommand, its options, and its output. Diagnostics always go to standard error; the expansion text is the only thing written to standard output, so you can safely pipe or capture it.

The available subcommands are:

| Command | Purpose |
| --- | --- |
| `expand` | Expand an abbreviation to code on standard output. |
| `wrap` | Wrap a selection (read from standard input) in an expanded abbreviation. |
| `list` | List every trigger registered for a language. |
| `check` | Validate the user snippet override directory. |
| `lsp` | Run the Language Server Protocol server over standard input and output. |

Every command accepts `-h` or `--help`. The top-level command also accepts `-V` or `--version`.

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

### Reading from standard input

Omit the abbreviation or pass `-` to read it from standard input, which lets you pipe the abbreviation in:

```sh
echo 'fori10>pr' | codemmet expand --lang go -
```

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

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success. |
| 1 | Abbreviation parse error, such as malformed syntax like `>pr`. |
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

Every command writes only the expansion text to standard output; all diagnostics go to standard error. The `expand` command reads its abbreviation from standard input when you omit it or pass `-`, and `wrap` always reads its selection from standard input. This makes codemmet easy to use in pipelines and from editors.

## See also

- [abbreviation syntax](syntax.md) for the full abbreviation grammar.
- [language reference](languages.md) for the triggers and behaviors of each language.
- [editor setup](editors.md) to run the Language Server Protocol server in your editor.
- [tutorials](tutorials.md) for hands-on examples.
