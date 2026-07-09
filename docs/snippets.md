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

## Project-local packs

A repository can ship its own snippet packs so everyone working in it shares
the same triggers. Put them in `.codemmet/snippets/<lang>.toml` at (or above)
the project root — the same file layout as the user directory, just named
`.codemmet/snippets` inside the project.

codemmet collects **every** `.codemmet/snippets` from the file's directory up to
the filesystem root and merges them, so a monorepo can carry a repo-wide pack at
its root and a refining pack in a subproject. They merge **root-most first, so
the nearest pack wins** a per-trigger conflict while still inheriting everything
the outer packs define — a subproject pack that only redefines `log` keeps every
other trigger the root pack provides.

Project packs are the **highest-priority** tier, so the full precedence is:

```
builtin (+ inherited)
  <  user            (~/.config/codemmet/snippets)
  <  user path-mapped (personal packs matched by path — see below)
  <  project         (.codemmet/snippets, root-most → nearest)
```

A project pack overrides a user pack overrides the builtins, and any pack can
`disabled = true` a trigger from a lower tier.

How they load differs by entry point, on purpose:

- **CLI: auto-loaded.** `codemmet expand`/`wrap`/`list`/`check` discover
  `.codemmet/snippets` from the working directory automatically — like a
  formatter reading its dotfile. Pass `--no-project` to ignore it.
- **LSP: opt-in.** The editor opens whatever repository you navigate to, so the
  server loads project packs only when you set the `project_snippets`
  initialization option (see [editor setup](editors.md)). Snippet packs are
  inert text — the worst a hostile pack can do is make your abbreviations
  expand to surprising output, not run code — but the opt-in keeps a freshly
  cloned repo from silently changing your expansions.

`codemmet check` validates the project pack too and reports it alongside the
user pack.

## Personal path-scoped packs

Sometimes you want a snippet set that follows *you* rather than living in a
repo — a personal pattern library you'd rather not commit, but that should apply
whenever you work under a given directory. A `paths.toml` maps path prefixes to
snippet directories:

```toml
# ~/.config/codemmet/paths.toml  (Linux; alongside the snippets/ dir)
[[map]]
prefix = "~/work/acme"
dir    = "~/snippets/acme"

[[map]]
prefix = "~/oss"
dir    = "~/snippets/rust-oss"
```

When the file you're editing lives under a `prefix`, that `dir` joins the merge
as the **user path-mapped** tier — above your global user pack but below a
committed project pack (so a repo's own conventions still win inside it).
Prefixes match by path component (`~/work/acme` matches `~/work/acme/lib/x.rs`
but not `~/work/acme-fork/…`); when several match, the deeper prefix wins. A
missing `paths.toml` simply means no mappings.

## Capturing a snippet from code

You don't have to hand-write the TOML. Point codemmet at a piece of real code
and it writes the snippet for you, generalizing the repeated identifiers into
parameters:

```sh
codemmet capture --lang rust --name mkwidget --param Widget - <<'EOF'
fn make_widget() -> Widget {
    Widget::default()
}
EOF
# → captured `mkwidget` (1 parameter: Widget) into ./.codemmet/snippets/rust.toml

codemmet expand --lang rust mkwidget         # reproduces the code
codemmet expand --lang rust mkwidgetGadget   # the Widget parameter → Gadget
```

It writes into the project pack in scope (or the user directory with `--user`),
re-validating the whole file before saving so the result always passes
`codemmet check`. In Neovim, select code and run `:CodemmetCapture` (see
[editor setup](editors.md)). Full flags are in the
[CLI reference](cli.md#capture).

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
ok: 7 languages, 282 triggers (user snippets: /home/you/.config/codemmet/snippets)
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
| `body` | `[snippets.<trigger>]` | Required unless `disabled` or `expands`. The template. |
| `scope` | `[snippets.<trigger>]` | Optional. The scope `${child}` content lives in. |
| `variants` | `[[snippets.<trigger>.variants]]` | Optional context-conditional bodies. |
| `disabled` | `[snippets.<trigger>]` | In a user pack, removes the builtin of this name. |
| `expands` | `[snippets.<trigger>]` | Makes this a [composite](#composite-snippets): an abbreviation re-expanded at render time (mutually exclusive with `body`). |
| `options` | `[snippets.<trigger>.options]` | Named selectable fragments for a composite; the caller picks them by name. |

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

### Case transforms on parameters

`${name|snake}` (also `camel`, `pascal`, `upper`, `kebab`, `lower`) re-cases
the parameter's value before inserting it, so one argument can appear in
several conventions:

```toml
[snippets.setter]
description = "setter method"
params = [{ name = "field" }, { name = "ty", default = "()" }]
body = '''
fn set_${field|snake}(&mut self, v: ${ty}) {
	self.${field|snake} = v;
}
'''
```

`setter:maxSize` writes `fn set_max_size(…) { self.max_size = v; }`, and
`setter:${value},${value.type}*@Config` stamps one correctly-cased setter per
struct field. The same suffix works on data-driven `${value…}` / `${outer…}`
references (order `source → .member → |transform → :default`, so a `|` inside
a `:default` stays literal) — see the [data-driven
guide](context-codegen.md#case-transforms).

Transforms apply to **known text only**: a bound argument, a resolved item, or
a member. When the parameter is *unbound*, the reference renders exactly as
without the transform — a tabstop (mirrored across occurrences) whose
placeholder is the declared `default`, verbatim. The engine cannot re-case
text the user has yet to type, so `struct ${name|pascal}` + `fn
new_${name|snake}` gives no case linkage while typing into the tabstop; supply
the argument (or drive the snippet from a collection) to get the re-casing.
One transform per reference; an unknown or chained name fails `check`.

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

## Composite snippets

A normal snippet has a literal `body`. A **composite** has an `expands` field
instead: an *abbreviation* that is re-expanded at render time. It lets you build
a higher-level scaffold — a class with the common dunders, a derived struct, an
include-guarded header — out of the trigger vocabulary you already have, so the
scaffold stays DRY: improve the underlying `class`/`struct`/`guard` triggers and
every composite built on them follows.

```toml
[meta]
language = "rust"

[snippets.dstruct]
description = "#[derive]d struct"
params = [{ name = "name", kind = "ident", default = "Name" }]
expands = "derive+struct:${name}>field"
```

```sh
codemmet expand --lang rust "dstruct:Point"
```

```text
#[derive(Debug, Clone)]
struct Point {
    name: Type,
}
```

Because a composite expands to an ordinary abbreviation, everything the parser
and renderer already do — composition (`>`/`+`/`*`), tabstops, context
variants, indentation, global tabstop renumbering — just works. The only new
piece is *resolution*: substitute the parameters, then re-parse and compose.

### `expands` and `body` are mutually exclusive

A composite declares `expands` and **not** `body`. It also may **not** declare
`scope` or `variants` — the construct it expands to (say `struct`) already owns
its child scope and any context variants. `description` and `params` work
exactly as for a normal snippet.

### `${name}` is passed through, not substituted as text

A `${name}` in `expands` marks an **argument position** for a sub-trigger
(`struct:${name}`). It is resolved by passing the composite's argument *through*
to that sub-trigger, so the underlying snippet handles it natively:

- a **bound** param (the caller supplied it) emits the value as that argument —
  `dstruct:Point` → `struct:Point` → `struct Point`;
- an **unbound** param passes through as a *tabstop* carrying the composite
  param's own `default` as placeholder — bare `dstruct` → `struct ${1:Name}` in
  keep mode, a tabstop you can tab into.

When one unbound param flows into **several** sub-triggers, they share a single
tabstop and **mirror** each other, so the editor keeps them in sync. Rust
`newtype` = `(struct:${name}>field)+impl:${name}` is the motivating case — bare,
its `name` renders once with a placeholder (`struct ${1:Name}`) and once as a
bare mirror (`impl $1`).

Values are spliced into the sub-trigger's arguments *after* parsing, so operator
characters (`>`, `+`, `:`, `,`) in a value pass through verbatim — a composite
param may be any kind (`ident`, `int`, or `str`). And because a passed-through
tabstop keeps its position filled, an unbound argument may sit before a bound
one (`fori:${var},${end}` with only `end` supplied renders `var` as a tabstop
ahead of it).

### Options and freeform children

A composite carries up to three kinds of content, merged as siblings at one
insertion point — by default the **children of the last top-level node** of
`expands` — in this order:

1. the **base** pieces written directly in `expands`;
2. **selectable options** the caller picks by name from an `options` table;
3. **freeform `>`-children** the caller supplies.

To place them somewhere else, write `${child}` directly in `expands` (at most
once): the extras go there instead of at the last node — for example
`expands = "(impl:${name}>${child})+pr"` drops them into the `impl` body rather
than after the trailing `pr`.

The `[snippets.<trigger>.options]` table is a menu of named abbreviation
fragments. Any bare argument left after the declared params bind is treated as
an option name:

```toml
[snippets.classp]
description = "class with common dunder methods"
params = [{ name = "name", kind = "ident", default = "Name" }]
expands = "class:${name}>(init+str+repr)"

[snippets.classp.options]
eq   = "def:__eq__"
hash = "def:__hash__"
len  = "def:__len__"
```

```sh
# base only; base + selected options; base + option + freeform child
codemmet expand --lang python "classp:Point"
codemmet expand --lang python "classp:Point,eq,hash"
codemmet expand --lang python "classp:Point,eq>def:validate"
```

An unknown option name is an error (`composite \`classp\` has no option \`…\``).

### Full-node semantics and safety

A composite is a normal node in a chain: `classp:P>def:x` merges `def:x` into
the class body, `classp:P+pr` adds a sibling, `ifmain>classp:P` nests the whole
expansion one level deeper, and `classp:P*2` stamps two independent copies.
A composite may reference other composites; **definitional cycles are rejected
at compile time** and a runtime depth cap (16 levels) bounds pathological
caller nesting. A **data-driven** repeat (`classp*@Msg`) is *not* supported on a
composite — it has no single collection subject.

### Built-in composites

Every language pack ships a starter set, each built only from triggers that
already exist in its pack:

| Lang | Trigger | Expands to | What you get |
| --- | --- | --- | --- |
| Python | `classp:Name` | `class>init+str+repr` (+options `eq`/`hash`/`len`/`ord`) | class with common dunders |
| Python | `ctxmgr:Name` | `class>init+__enter__+__exit__` | context-manager class |
| Rust | `dstruct:Name` | `derive+struct>field` | `#[derive]`d struct |
| Rust | `denum:Name` | `derive+enum>variant` | `#[derive]`d enum |
| Rust | `newtype:Name` | `(struct>field)+impl` | struct paired with an impl block |
| Rust | `testmod` | `tmod>test` | test module with a first test |
| Go | `stringer:Name` | `(struct>field)+meth:String` | struct + a `String()` method |
| Go | `errtype:Name` | `(struct>field)+meth:Error` | error struct + an `Error()` method |
| Go | `maincmd` | `pkg:main+main` | `package main` + `func main` |
| TS/JS | `classc:Name` | `class>ctor` | class with a constructor |
| C | `hstruct:Name` | `guard>struct` | include-guarded struct for a header |
| C | `henum:Name` | `guard>enum>variant` | include-guarded enum for a header |
| C++ | `hclass:Name` | `guard>class` | include-guarded class for a header |
| C++ | `nsclass:Name` | `ns>class` | class inside a namespace |

`codemmet check` validates composites like any other snippet, and additionally
rejects an `expands`/option fragment that doesn't parse, a reference to a
trigger that doesn't resolve, and definitional cycles.

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
ok: 7 languages, 281 triggers (user snippets: /home/you/.config/codemmet/snippets)
```

## See also

- [abbreviation syntax](syntax.md) — arguments, composition, and resolution
- [language reference](languages.md) — the builtin triggers you are overriding
- [CLI reference](cli.md) — `check`, `list`, `expand`, and their flags
- [tutorials](tutorials.md) — a hands-on walkthrough
- [documentation home](README.md)
