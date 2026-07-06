# Custom snippets

codemmet ships with a builtin snippet pack for every language it supports. A
snippet pack is a TOML file of triggers; each trigger maps a keyword like `fori`
to an expansion. This page shows you how to author your own pack, override or
disable builtin triggers, and validate the result.

If any term here is unfamiliar, see the [abbreviation syntax](syntax.md) page
for how abbreviations, triggers, arguments, and composition operators fit
together, and the [language reference](languages.md) for the builtin triggers
per language. For a guided, hands-on introduction, start with the
[tutorials](tutorials.md).

## Where packs live

Your custom packs go in the override directory. The default location depends on
your operating system:

| Platform | Default override directory |
| --- | --- |
| Linux | `~/.config/codemmet/snippets/` |
| macOS | `~/Library/Application Support/codemmet/snippets/` |
| Windows | `%APPDATA%\codemmet\snippets\` |

You can point codemmet at a different directory in two ways:

- On the CLI, pass `--config <DIR>` to any command (`expand`, `wrap`, `list`,
  `check`).
- In the LSP server, set `snippets_dir` in the init options (see
  [editor setup](editors.md)).

Each file in the directory is named `<language-id>.toml`, for example
`rust.toml`, `go.toml`, or `python.toml`. The file name selects which language
the pack applies to.

## The merge-over-builtins model

A user pack does not replace the builtin pack. It is **merged over** it:

- A trigger you define with the same name as a builtin **replaces** that
  builtin.
- Every other builtin trigger for that language **remains available**.
- A trigger name that does not exist in the builtin pack is **added**.
- Marking a trigger `disabled = true` **removes** the builtin of that name.

This means a small pack that redefines two triggers leaves the other forty
builtins untouched.

## Your first custom snippet

Create the override directory and a `rust.toml` inside it:

```sh
mkdir -p ~/.config/codemmet/snippets
```

Put this in `~/.config/codemmet/snippets/rust.toml`:

```toml
[meta]
language = "rust"

[snippets.log]
description = "tracing::info! line"
params = [{ name = "msg", kind = "str", default = "message" }]
body = '''
tracing::info!("${msg}");$0
'''
```

Validate the directory:

```sh
codemmet check
```

```text
ok: 7 languages, 252 triggers (user snippets: /home/you/.config/codemmet/snippets)
```

Confirm the trigger is registered (`list` shows it merged in alphabetically):

```sh
codemmet list --lang rust
```

```text
...
letq    	let binding from a fallible expression with ?
log     	tracing::info! line
loop    	infinite loop
...
```

Expand it. The `msg` parameter is `str`, so it takes an explicit `:` argument:

```sh
codemmet expand --lang rust "log:hello"
```

```text
tracing::info!("hello");
```

With no argument, the parameter's `default` is used:

```sh
codemmet expand --lang rust "log"
```

```text
tracing::info!("message");
```

That is the whole loop: write TOML, run `check`, run `list`, run `expand`.

## Schema reference

Here is a fully commented pack showing every key. codemmet rejects unknown
keys, so this is the complete set.

```toml
[meta]
language = "rust"           # required; must equal the language id (and the file name)

[snippets.log]              # <trigger> must be a WORD: [A-Za-z_][A-Za-z0-9_]*
description = "..."         # optional; shown in `list` and editor completion
params = [                  # optional; declared parameters, in binding order
  { name = "msg", kind = "str", default = "message" },
]
scope = "loop"              # optional; the scope the ${child} content lives in
body = '''
tracing::info!("${msg}");$0
'''                         # required (unless disabled = true)

# optional context-conditional variants; first match wins, else the top-level body:
[[snippets.log.variants]]
context = "type-body"       # a normalized scope name, or "ts:<node_kind>"
body = '''...'''

[snippets.epr]
disabled = true             # in a user pack: remove/disable a builtin trigger
```

Field by field:

| Key | Where | Meaning |
| --- | --- | --- |
| `meta.language` | `[meta]` | Required. Must equal the language id and match the file name. |
| `description` | `[snippets.<trigger>]` | Optional. Shown by `list` and in editor completion. |
| `params` | `[snippets.<trigger>]` | Optional. Declared parameters, in binding order. |
| `params[].name` | param entry | The name referenced as `${name}` in the body. |
| `params[].kind` | param entry | One of `ident`, `int`, `str` (default `str`). Controls glued-argument binding. |
| `params[].default` | param entry | Placeholder shown when no argument is supplied. |
| `body` | `[snippets.<trigger>]` | Required unless `disabled`. The template. |
| `scope` | `[snippets.<trigger>]` | Optional. The scope `${child}` content lives in. |
| `variants` | `[[snippets.<trigger>.variants]]` | Optional context-conditional bodies. |
| `disabled` | `[snippets.<trigger>]` | In a user pack, removes the builtin of this name. |

## Parameters and argument binding

A **parameter** is a declared slot in a snippet. An abbreviation fills it with
a **glued argument** (an argument written right after the trigger, like the
`10` in `fori10`) or an **explicit argument** (a `:` or `,` argument, like the
`main` in `fn:main`).

The `kind` decides which glued arguments a parameter accepts:

| `kind` | Accepts as glued arg | Example |
| --- | --- | --- |
| `ident` | An identifier-like glued arg | `fori` binds `x` in `forx` |
| `int` | A numeric glued arg | `fori` binds `10` in `fori10` |
| `str` | Nothing glued; explicit args only | `log:hello` |

When more than one parameter is declared, a numeric glued arg binds to the
first `int` parameter and an identifier glued arg to the first `ident`
parameter. So the builtin `fori`, whose params are
`[{ name = "var", kind = "ident" }, { name = "end", kind = "int" }]`, reads
`fori10` as "var stays default, end is 10":

```sh
codemmet expand --lang rust "fori10"
```

```text
for i in 0..10 {
    
}
```

Explicit arguments bind positionally to `params` in order, regardless of kind.
This is how `str` parameters receive a value, and it is why `log:hello` fills
`msg`.

If an argument is not supplied, the parameter's `default` is used: in strip
mode it is inserted as literal text, in keep mode it becomes a tabstop
placeholder (see the tabstops section below).

## Body syntax

A body is LSP snippet syntax plus three codemmet extensions.

- `${name}` inserts the value of a declared parameter. Every `${name}` in a
  body must reference a declared parameter, or `check` fails.
- `${child}` marks where content nested under this trigger with `>` is placed.
  A body may contain **at most one** `${child}`.
- `${i}` is the **repetition index**: the 1-based copy number when the trigger
  is repeated with `*n` (and `1` otherwise). A body of `field${i}: ${1:Type},`
  expanded with `field*3` yields `field1:`, `field2:`, `field3:`. `i` is a
  reserved parameter name because of this.
- `$1`, `$2`, ... are ordered **tabstops** an editor jumps between with Tab.
- `${1:placeholder}` is a tabstop with default text.
- `$0` is the final cursor position.
- Repeating the same `${name}` or `$1` creates a **mirror**: all copies stay in
  sync. For example, a body of `let ${field} = self.${field};$0` expanded with
  `getset:count` produces `let count = self.count;`.
- Escape a literal dollar sign as `\$`. In strip mode it renders as `$`; in
  keep mode it stays escaped as `\$`.

### Indentation must be tabs

Indent the body with **tabs**, one tab per nesting level. The renderer converts
each tab to the active indent unit, so the same body produces four-space Rust
and tab-indented Go. Groups of exactly four spaces per level are also accepted,
but tabs are the rule and adapt cleanly to any target. Any other indentation
(for example two spaces) is rejected:

```text
error: invalid snippet `two` in /path/go.toml: line 2: indentation must be tabs or groups of 4 spaces
```

## Overriding a builtin

Define a trigger with the same name as a builtin and yours wins. This pack
redefines Rust's `pr`:

```toml
[meta]
language = "rust"

[snippets.pr]
description = "println! (dbg-friendly)"
body = '''
println!("{}", $0);
'''
```

```sh
codemmet expand --lang rust "pr"
```

```text
println!("{}", );
```

`list` shows your description in place of the builtin one, and every other Rust
trigger is still present.

## Disabling a builtin

Give the trigger `disabled = true` and no body to remove it:

```toml
[meta]
language = "rust"

[snippets.epr]
disabled = true
```

After `check` passes, the trigger is gone:

```sh
codemmet expand --lang rust "epr"
```

```text
error: cannot expand `epr`: unknown trigger `epr` (did you mean `pr`?)
```

## Advanced: context-aware snippets

The `scope` and `variants` keys let one trigger expand differently depending on
where it lands. This is optional; skip it until you need it.

Every snippet can declare the `scope` its `${child}` content lives in — one of
`root`, `type-body`, `fn-body`, `loop`, `conditional`, `match-body`, or
`try-body`. When you nest a trigger under another with `>`, codemmet knows the
surrounding scope and can pick a matching **variant**.

The builtin Rust `fn` uses this to add `&self` when a function is written
inside an `impl` block. Here is the same idea as a user pack:

```toml
[meta]
language = "rust"

[snippets.fn]
description = "function definition"
params = [{ name = "name", kind = "ident", default = "name" }]
scope = "fn-body"
body = '''
fn ${name}($1) {
	${child}
}
'''

[[snippets.fn.variants]]
context = "type-body"
body = '''
fn ${name}(&self$1) {
	${child}
}
'''
```

At the top level, the plain body is used:

```sh
codemmet expand --lang rust "fn:area"
```

```text
fn area() {
    
}
```

Nested inside an `impl` block (a `type-body` scope), the variant fires and adds
`&self`:

```sh
codemmet expand --lang rust "impl:Point>fn:area"
```

```text
impl Point {
    fn area(&self) {
        
    }
}
```

Variants are matched top to bottom; the first whose `context` matches the
surrounding scope wins, otherwise the top-level `body` is used. A `context` may
also be `ts:<node_kind>` to match a raw tree-sitter node kind directly.

## Validating with `codemmet check`

`codemmet check` validates the override directory and reports the first
problem it finds. Run it after every edit. The rules it enforces:

| Rule | Example error |
| --- | --- |
| `meta.language` matches the pack | `meta.language is \`python\` but this pack is loaded for \`rust\`` |
| A trigger is a word | `trigger must be a word ([A-Za-z_][A-Za-z0-9_]*)` |
| A non-disabled snippet has a body | `missing body` |
| Every `${name}` is a declared param | `body references undeclared parameter \`bar\`` |
| Every declared param is referenced | `parameter \`x\` is never referenced in the body` |
| No unknown keys | `unknown field \`color\`, expected one of ...` |
| Indentation is tabs or 4-space groups | `indentation must be tabs or groups of 4 spaces` |

A file whose name is not a known language is not fatal; `check` warns and
continues:

```text
warning: /path/notalang.toml does not match any language pack name (expected one of: c.toml, cpp.toml, go.toml, javascript.toml, python.toml, rust.toml, typescript.toml)
```

A clean run reports the totals and exits 0:

```sh
codemmet check
```

```text
ok: 7 languages, 250 triggers (user snippets: /home/you/.config/codemmet/snippets)
```

## See also

- [abbreviation syntax](syntax.md) — arguments, composition, and resolution
- [language reference](languages.md) — the builtin triggers you are overriding
- [CLI reference](cli.md) — `check`, `list`, `expand`, and their flags
- [tutorials](tutorials.md) — a hands-on walkthrough
- [documentation home](README.md)
