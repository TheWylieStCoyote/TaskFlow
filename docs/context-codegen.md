# Context-aware and data-driven expansion

Most of codemmet is pure text: a trigger and its arguments expand the same way
everywhere. But in an editor, codemmet also reads the code *around* the cursor
with tree-sitter, and can use it two ways:

- **Context-aware variants** — the same trigger expands differently depending on
  the enclosing scope (a `fn` inside an `impl` gains `&self`).
- **Data-driven repeats** — a repeat is driven by a *collection* pulled from the
  buffer (a `match` fills one arm per enum variant), instead of a fixed count.

Both need the tree-sitter provider, so they apply on the **editor / language
server path**. Plain CLI expansion has no buffer to read; where it matters, the
CLI offers explicit flags (`--repeat`) so the behavior is still reachable and
testable. This is the same split as the rest of codemmet's context features —
see the [`NullContextProvider`](../PLAN.md) note.

## Context-aware variants

A snippet can declare scope-specific variants. The classic case: a function
gains a receiver parameter inside a type body.

In an editor, typing `fn` inside an `impl` block expands to `fn name(&self) {…}`;
at the top level it expands to `fn name() {…}`. The same happens for composed
abbreviations, because a snippet's declared scope flows to its children:

```sh
codemmet expand --lang rust "impl:Point>fn:area"
```

```rust
impl Point {
    fn area(&self) {

    }
}
```

Python's `def` behaves the same inside a `class`:

```sh
codemmet expand --lang python "class:Widget>def:run"
```

```python
class Widget:
    def run(self):

```

The normalized scopes codemmet recognizes are `type-body`, `fn-body`, `loop`,
`conditional`, `match-body`, and `try-body`; snippet authors match on them (or
on a raw tree-sitter node kind) to add variants. See
[custom snippets](snippets.md) for how to declare them.

## Data-driven repeats

A repeat written `*@name` draws from a **collection** — an ordered list of
**items** — and renders one copy per item, each filled from that item. The
snippet body references the current item:

| In a body | Renders |
| --- | --- |
| `${value}` | the item's primary value (an enum variant, a field name, a list element) |
| `${value.type}` (or any `${value.key}`) | a named member of the item |
| `${value:default}` | the value, or `default` (as a tabstop) when there is no data |
| `${outer}` (also `.key` / `:default`) | the **enclosing** repeat's item, for a nested repeat (see [Nesting](#nesting-a-cross-product)) |

### Naming each copy from the item

A snippet named from a **parameter** — like `fn ${name}(…)` — is named by the
collection automatically. When the body doesn't itself use `${value}`, the item
**auto-binds to the trigger's first unfilled parameter**, so the simple case
needs nothing special:

```sh
codemmet expand --lang rust "impl:Point>fn*@Meth" --repeat Meth=new,len,area
```
```rust
impl Point {
    fn new(&self) {  }
    fn len(&self) {  }
    fn area(&self) {  }
}
```

For finer control you can place the item explicitly. Writing `${value}` in an
argument is the same as the auto-bind above:

```sh
codemmet expand --lang rust 'impl:Point>fn:${value}*@Meth' --repeat Meth=new,len,area
```

and `${value.member}` picks a member of the item while `${value:default}`
supplies a fallback — each substituted per copy. Use the explicit form when a
trigger has several arguments and you need the item in a specific one, or to
pass a member rather than the primary value.

> **Single-quote an explicit `${value}`.** It looks like a shell variable, so a
> double-quoted `"…${value}…"` lets the shell blank it before codemmet sees it.
> Single quotes (or escaping `\${value}`) keep it intact. (The bare `fn*@Meth`
> form above has no `${value}`, so it needs no special quoting.)

An explicit argument wins over the auto-bind (it fills the slot), a body that
already uses `${value}` (arm/field/variant) is never auto-bound, and with no
data a bare `${value}` argument falls back to that parameter's own tabstop.

Three built-in snippets use these in their bodies, so the common cases work out
of the box:

| Trigger | Body | Fills from |
| --- | --- | --- |
| `arm` | `${value:pattern} => $0,` | a `match`/`switch` over a collection |
| `field` | a field definition, with `${value.type}` | a `struct`/record/interface |
| `variant` | an enum member | an `enum` definition |

### Where the collection comes from

**In an editor**, codemmet extracts collections from the buffer with
tree-sitter. Three kinds are recognized:

- **Enum variants**, keyed by the enum name. Typing `match:msg>arm*@Msg` (or
  the shorthand `match:Msg>arm*`, which infers the subject) reads the variants
  of a same-file `enum Msg` and writes one arm each.
- **Struct / record fields**, keyed by the type name, each carrying its declared
  `type`. `struct:Wrapper>field*@Point` copies `Point`'s fields, and
  `${value.type}` renders each field's type.
- **In-scope list bindings**, keyed by the variable name. Given
  `let items = [a, b, c];`, an `arm*@items` repeats over `a`, `b`, `c`.

Enum and struct/record definitions are visible anywhere in the file (a
definition may sit below its use). A list binding must be **lexically in scope**
and appear **before the cursor**; when two in-scope bindings share a name, the
nearest one wins.

**On the CLI** there is no buffer, so you supply collections explicitly with
`--repeat name=item1,item2,…` (repeatable). Each item may carry a member after a
colon (`value:type`):

```sh
codemmet expand --lang rust "match:shape>arm*@Shape" \
    --repeat Shape=Circle,Square,Triangle
```

```rust
match shape {
    Circle => ,
    Square => ,
    Triangle => ,
}
```

```sh
codemmet expand --lang rust "struct:Wrapper>field*@Point" \
    --repeat Point=x:i32,y:f64
```

```rust
struct Wrapper {
    x: i32,
    y: f64,
}
```

The [browser playground](https://thewyliestcoyote.github.io/codemmit/) has a
**Data-driven collections** box that takes the same `name=values` lines, so you
can try all of this without an editor.

**Inline in the abbreviation.** When the values are short and one-off, skip the
flag entirely and write them as a literal `*[a,b,c]` (each item optionally
`value:type`, parsed exactly like a `--repeat` value):

```sh
codemmet expand --lang rust "impl:Point>fn*[new,len,area]"
```

```rust
impl Point {
    fn new(&self) {  }
    fn len(&self) {  }
    fn area(&self) {  }
}
```

This works the same in the editor — no buffer collection or flag needed — which
makes it the quickest way to stamp out a handful of named members on the spot.

### Nesting: a cross product

Data-driven repeats **nest** — the inner one runs once per outer copy, so two
of them form a cross product. This writes each method for each type:

```sh
codemmet expand --lang rust "impl*[Point,Line,Circle]>fn*[new,len,area]"
# or with named collections:
codemmet expand --lang rust "impl*@Types>fn*@Methods" \
    --repeat Types=Point,Line,Circle --repeat Methods=new,len,area
```

```rust
impl Point { fn new(&self) {} fn len(&self) {} fn area(&self) {} }
impl Line   { … }
impl Circle { … }
```

Each level auto-binds *its own* item (the outer names the `impl`, the inner names
the `fn`), and `${value}` inside a level refers to that level's item.

To reach the **enclosing** item from inside the inner body or an inner argument,
use `${outer}` — it names the nearest surrounding data-driven repeat's item (and
takes the same `.member` / `:default` suffixes as `${value}`). So a body that
pairs the two,

```sh
codemmet expand --lang rust "impl*[Point,Line]>const*[X,Y]"
# where `const` is `const ${outer}_${value}: …` → Point_X, Point_Y, Line_X, Line_Y
```

writes the cross product with both levels' items in each copy. With no enclosing
repeat, a bare `${outer}` falls back to its own tabstop, exactly like `${value}`.

### Defining, then consuming

The `variant`/`field` snippets and the `arm` snippet are mirror images, so the
same collection feature scaffolds a definition and later consumes it:

```sh
# define — three variants, filled from a collection (or *3 for blank tabstops)
codemmet expand --lang rust "enum:Color>variant*@V" --repeat V=Red,Green,Blue
```

```rust
enum Color {
    Red,
    Green,
    Blue,
}
```

In an editor, once `Color` exists in the buffer, `match:c>arm*@Color` (or
`match:Color>arm*`) reads those variants straight back out — no `--repeat`.

### The no-data fallback

A data-driven repeat with no matching collection degrades to a **single copy**
whose `${value:default}` spans become tabstops, so it is always safe to type. A
bare `arm*` with no enum in scope yields one `pattern => ,` arm; `field*` yields
one `name: Type,`. This keeps `arm*3` (a fixed count) and a plain `arm`
identical to before data-driven repeats existed.

### What it does *not* do

Extraction is **shape-directed, not type-aware**. codemmet reads what the syntax
literally says — an enum's written variants, a struct's written fields, a list
literal's written elements — and you name the source (`@Msg`). It does **not**
infer a variable's type to find its definition, resolve imports, or read other
files; that is a type-checker's job (rust-analyzer, tsc), not codemmet's.

Rust payload variants are handled minimally: a tuple variant extracts as
`Write(..)` and a struct variant as `Move { .. }`, so a generated `match` arm
compiles without your having to name each field. The bindings are wildcards —
replace them if you need the payload.

## Language coverage

| Source | Rust | C / C++ | TypeScript | Go | Python | JavaScript |
| --- | :-: | :-: | :-: | :-: | :-: | :-: |
| Enum variants | ✓ | ✓ | ✓ (incl. string-union `type`) | — | ✓ (`Enum` subclass) | — |
| Struct / record fields | ✓ | ✓ | ✓ (interface) | ✓ | ✓ (dataclass) | — |
| In-scope lists | ✓ (incl. `vec!`) | ✓ | ✓ | ✓ (`:=` / `var`) | ✓ | ✓ |

Go has no `enum` keyword (its enums are `const`/`iota` blocks) and JavaScript
has neither enums nor typed fields, which is why those cells are blank.

## Code actions

Data-driven repeats are powerful but easy to miss — you have to know `arm*@`
exists. So the same machinery also drives a **code action**, which any editor
surfaces without you memorizing anything.

Put the cursor inside a `match` over a value and trigger your editor's code
actions (in Neovim, `vim.lsp.buf.code_action()`); for each `enum` in the file
you get **"Fill match arms from `Name`"**:

```rust
enum Msg { Quit, Write(String), Move { x: i32 } }

fn handle(m: Msg) {
    match m {}   // cursor here → "Fill match arms from Msg"
}
```

applying it →

```rust
    match m {
        Quit => todo!(),
        Write(..) => todo!(),
        Move { .. } => todo!(),
    }
```

Payload variants bind wildcards so the result compiles; arms already present are
skipped, so it also completes a partial `match`. The edit is plain text (no
tabstops), so it works in any LSP client — no codemmet plugin needed. This is
Rust-only for now.

## See also

- [Abbreviation syntax](syntax.md) — the `*@name` grammar and the `${value}`
  spans in one place.
- [Custom snippets](snippets.md) — declaring context variants and using
  `${value}` / `${value.type}` in your own snippet bodies.
- [Advanced examples](examples/advanced.md) — worked, end-to-end abbreviations.
- [Language reference](languages.md) — every trigger per language.
