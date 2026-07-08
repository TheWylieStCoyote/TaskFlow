# Composite (alias) snippets

A **composite** is a snippet whose definition is *itself an abbreviation* that
gets re-expanded, instead of a literal template body. It lets you author
higher-level scaffolds — a class with the common dunders, a service module,
a test file — out of the trigger vocabulary you already have, so the scaffold
stays DRY: change the underlying `class`/`def`/`init` triggers and every
composite built from them follows.

```toml
[snippets.classp]
description = "class with common dunder methods (composite)"
params = [{ name = "name", kind = "ident", default = "Name" }]
expands = "class:${name}>(init+str+repr)"

[snippets.classp.options]
eq  = "def:__eq__"
hash = "def:__hash__"
len  = "def:__len__"
ord  = "def:__lt__+def:__le__"
```

```sh
codemmet expand --lang python "classp:Point"
```

```python
class Point:
    def __init__(self):

    def __str__(self) -> str:
        return f""
    def __repr__(self) -> str:
        return f"{type(self).__name__}()"
```

The elegant property: **a composite expands to a normal abbreviation, which the
existing parser and renderer already handle** — composition (`>`/`+`/`*`),
tabstops, context variants, indentation, global tabstop renumbering, and the
size budget all flow through the ordinary pipeline. The only new work is
*resolution*: substitute the params, splice in the selected options and the
caller's freeform children, then re-parse and compose the result.

## Built-in composites

Beyond `classp`, the builtin packs ship a starter set (each built only from
triggers that already exist in its pack, so it stays DRY):

| Lang | Trigger | Expands to | What you get |
| --- | --- | --- | --- |
| Python | `classp:Name` | `class>init+str+repr` (+options `eq`/`hash`/`len`) | class with common dunders |
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

Each accepts the name inline (`newtype:Point`) or leaves a tabstop when bare
(`newtype`), takes freeform `>`-children (`ctxmgr:Timer>def:reset`), and — for
`classp` — selectable options (`classp:Point,eq`).

---

## Schema

Two new keys on a `[snippets.<trigger>]` table, both validated by the loader
(`deny_unknown_fields` still holds):

| Key | Meaning |
| --- | --- |
| `expands` | Marks a **composite**. Its value is an *abbreviation* (not a template body) that is re-expanded at render time. `${name}` references a declared parameter. |
| `options` | A table of named abbreviation **fragments** the caller selects by name. Each is merged into the expansion as a sibling of the base pieces. Only valid alongside `expands`. |

`expands` is **mutually exclusive with `body`**. A composite may **not** declare
`scope` or `variants` — the construct it expands to (e.g. `class`) already owns
its child scope and any context variants. `description` and `params` work
exactly as for a normal snippet.

A composite has no template body; internally its compiled form carries an empty
template and a `CompiledComposite { expands, options }`.

---

## Argument, option, and child rules

A composite is used like any trigger and carries **three** kinds of content:

1. a **baked-in base** — the pieces written directly in `expands`
   (`init+str+repr`);
2. a menu of **selectable named options** the caller picks by name
   (`classp:Point,eq,hash`);
3. **freeform `>`-children** supplied by the caller
   (`classp:Point>def:validate`).

### Splitting arguments into params vs. option names

> **Declared params bind first; any remaining bare args select options.**

Given `classp:Point,eq,hash` and one declared param (`name`):

- the first `1` explicit args (`Point`) bind to the declared params
  **positionally**, exactly like a normal snippet (a `str`/`ident`/`int` param,
  glued or explicit — the same `bind_args` path);
- every **remaining** explicit arg (`eq`, `hash`) is an **option name**. Each
  must exist in the `options` table, otherwise expansion fails with
  `composite \`classp\` has no option \`…\``.

Glued arguments (`classpWidget`) bind to a param by kind just as they would for
any snippet; options can only be selected with explicit `,`-args.

### `${name}` substitution — argument pass-through

A `${name}` marker in `expands` (or an option) is an **argument position** for a
sub-trigger (e.g. `class:${name}`). It is resolved by **passing the composite's
argument through** to that sub-trigger, so the underlying snippet handles it
natively — which means an unbound param becomes a real *tabstop*, not literal
text:

- a **bound** param (the caller supplied it) → the value is emitted as that
  argument: `classp:Point` → `class:Point` → renders `class Point`;
- an **unbound** param → a **shared tabstop** is passed through, so the
  sub-trigger renders a tabstop carrying the *composite* param's own default:
  bare `classp` → `class` → `class ${1:Name}` in keep mode, a tabstop you can
  tab into.

```sh
codemmet expand --lang python --tabstops keep "classp"        # class ${1:Name}$2: …
codemmet expand --lang python --tabstops keep "classp:Point"  # class Point$1: …
```

**Mirroring.** A single unbound param used in **several** sub-triggers passes
the *same* tabstop identity through to each, so the editor keeps them all in
sync. Rust `newtype` = `(struct:${name}>field)+impl:${name}` is the motivating
case — bare, its `name` renders once with a placeholder and once as a bare
mirror:

```sh
codemmet expand --lang rust --tabstops keep "newtype"
# struct ${1:Name} {           ← placeholder
#     ${2:name}: ${3:Type},
# }
# impl $1 {                    ← mirror of the same tabstop
#     $0
# }
```

The same identity is shared across the base pieces, selected options, freeform
children, **and** nested composites. Tabstop renumbering stays global across the
whole merged tree, so the passed-through tabstops interleave correctly.

**Mechanism.** Substitution can't drop or blank an argument at the string level
(`class:` does not parse), so markers are resolved on the *parsed* tree: each
`${name}` is first replaced by a parse-safe placeholder, the fragment is parsed,
then each placeholder that is a **whole argument** is replaced by the bound value
or, when unbound, by a *shared-stop sentinel* that renders as the mirrored
tabstop. Because a value is spliced into the sub-trigger's argument list **after**
parsing, it is never re-interpreted as grammar: a `str` value containing operator
characters (`>`, `+`, `:`, `,`) passes through verbatim, so composite params may
be any kind (`ident`, `int`, or `str`).

**Non-argument markers.** A `${name}` that is *not* a whole argument (embedded
in a longer arg, or glued into a word) has no argument slot, so it falls back to
value-or-default **text** substitution — an unbound embedded use is the plain
default text, not a tabstop. Composite params are meant for argument positions;
the text fallback keeps odd shapes working rather than failing.

**Interior arguments.** Because the shared-stop sentinel keeps the position
filled, an unbound argument that sits *before* a later one — `fori:${var},${end}`
with `var` unbound but `end` bound via a glued int — is no longer a positional
gap: `var` simply renders as a tabstop ahead of the bound `end`.

In an argument position the composite param's own `default` is the tabstop
placeholder (rather than the sub-trigger's default), which is what makes a
mirrored param show one consistent placeholder across every occurrence.

### The merge (insertion point)

The base pieces, the selected options, and the freeform children all become
**siblings at one insertion point**, in that order. By default:

> The insertion point is the **children of the last top-level node** of the
> parsed `expands`.

For `class:${name}>(init+str+repr)` the single top-level node is `class`, so the
extras become further children of `class` — siblings of `init`/`str`/`repr`
inside the class body. When the base is a single leaf (`class:${name}` with no
`>`), the extras simply become that node's children. If the last top-level node
is itself a leaf snippet with no `${child}` slot, the extras append after its
lines (the engine's existing no-slot behaviour).

**Explicit `${child}` slot.** A composite may instead write `${child}` directly
in its `expands` to place the extras somewhere other than the last node — for
example into an inner group, or into the first of several top-level nodes:

```toml
# extras land in the impl body, not after the trailing `pr`
expands = "(impl:${name}>${child})+pr"
```

`${child}` may appear **at most once** and only in `expands` (not in an option
fragment); `check` enforces both. With no extras to insert, the marker simply
vanishes.

```sh
codemmet expand --lang python "classp:Point,eq>def:validate"
```

```python
class Point:
    def __init__(self): ...
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...
    def __eq__(self):          # ← the `eq` option
    def validate(self):        # ← the freeform `>` child
```

---

## Full-node semantics

A composite behaves like any trigger — usable standalone **and** as a node in a
`>`/`+`/`*` chain:

| Form | Behaviour |
| --- | --- |
| `classp:Point` | standalone expansion |
| `classp:Point>def:x` | `def:x` is a freeform child, merged into the class body (see above) |
| `classp:Point+pr` | `+` sibling: the class, then a sibling `print()` at the same level |
| `ifmain>classp:Point` | composite as a **child**: its whole expansion nests one level deeper inside `if __name__ == "__main__":` |
| `classp:Point*2` | literal repeat: two independent copies, tabstops renumbered globally and independently per copy |

Guarded / awkward cases:

- **Data-driven repeat on a composite** (`classp*@Msg`, bare `classp*`) is
  **rejected** — a composite has no single collection subject and the semantics
  would be ambiguous. Error: `a data-driven repeat … is not supported on a
  composite`.
- **Composite as a child of a leaf** (`pr>classp:Point`): the leaf has no
  `${child}` slot, so the composite's lines append after it as siblings — the
  same rule the engine already applies to any child of a slot-less snippet.
- **Composite under `*n`**: each of the `n` copies is expanded independently, so
  tabstops never collide.

---

## Recursion, cycles, and depth

A composite may reference other composites (`svc` → `classp` → `class`). Two
guards keep this safe:

1. **Definitional cycles are rejected at compile time.**
   `SnippetSet::validate_composites` runs once over the fully merged set (it has
   to be whole-set, because a composite may reference a trigger a *later* pack in
   the merge adds). It builds the composite→composite reference graph and runs a
   DFS (a visited/on-stack set) to report the first cycle:
   `composite cycle: \`b\` expands through \`a\``. Because the graph is proven
   acyclic here, render-time expansion of any loaded pack always terminates.

2. **A runtime depth cap** (`MAX_COMPOSITE_DEPTH = 16`) is a defence-in-depth
   backstop, and it also bounds *caller-authored* nesting
   (`classp>classp>classp>…`, which is finite but could be pathological). It
   errors with `composite expansion nested too deep (more than 16 levels)`.

---

## Validation (`codemmet check`)

Composite validation is part of pack compilation, so `codemmet check` surfaces
every failure and exits non-zero. A composite is rejected when it:

- also declares `body`, or declares `scope`/`variants`;
- references an undeclared `${name}`, or declares a param it never references;
- has an `expands`/option fragment that **does not parse** as an abbreviation;
- references a trigger that does not resolve in the merged set
  (`references unknown trigger \`nosuchtrigger\``);
- forms a definitional cycle with other composites.

The builtin composites all pass `check` (7 languages, 281 triggers).

---

## Worked examples

```sh
# base only
codemmet expand --lang python "classp:Point"

# with selected options (eq + hash methods)
codemmet expand --lang python "classp:Point,eq,hash"

# with a freeform child merged into the class body
codemmet expand --lang python "classp:Point>def:validate"

# options + freeform child together (order: base, options, children)
codemmet expand --lang python "classp:Point,ord>prop:size"

# a nested composite (user pack): svc → classp → class
#   [snippets.svc]
#   params  = [{ name = "name", kind = "ident", default = "Svc" }]
#   expands = "classp:${name}>def:handle"
codemmet --config ./pack expand --lang python "svc:Server"
```

Error paths:

```text
$ codemmet expand --lang python "classp:Point,nope"
error: cannot expand `classp:Point,nope`: composite `classp` has no option `nope` (options: eq, hash, len, ord)   # exit 2

$ codemmet check          # pack with `a`→`b`→`a`
error: invalid snippet `b` in composite: composite cycle: `b` expands through `a`   # exit 3
```

---

## Resolved refinements

- **Multi-use param mirroring.** An unbound param used in several sub-triggers
  now shares one tabstop identity (see "Mirroring" above), so `newtype` and
  friends tab through one linked name instead of retyping it.
- **Explicit `${child}` slot in `expands`.** A composite may place `${child}` in
  its `expands` string to control where options and freeform children land,
  instead of the implicit "last top-level node" default.
- **`str`-value injection safety.** Values are spliced into the sub-trigger
  argument list after parsing, so operator characters survive verbatim; any
  param kind is safe.
- **Interior arguments.** An unbound argument before a bound one is handled (it
  renders as a tabstop) rather than rejected as a positional gap.
- **LSP completion of option names.** The server offers a composite's option
  names after the trigger's comma.

## Open questions / future work (designed, not built)

- **Error origin.** Composite validation reports the offending trigger but a
  generic `in composite` origin rather than the source file; threading the pack
  origin onto the compiled snippet would sharpen the message.
