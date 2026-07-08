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

### `${name}` substitution

Before the `expands` (and each selected option) is re-parsed, every `${name}`
marker is substituted **textually**:

- a **bound** param → its literal argument value;
- an **unbound** param → its declared `default` (empty string when it has none).

So `classp:Point` yields `class:Point>(init+str+repr)` and bare `classp` yields
`class:Name>(init+str+repr)` (the `Name` default).

**Consequence (documented tradeoff):** a composite's *own* params render as
literal text; they do **not** themselves become tabstops. Tabstops in the output
come from the *underlying* snippets (e.g. `class`'s `$1` for base classes, each
method's body cursor). Because values are spliced back into an abbreviation and
re-parsed, **composite params should be `ident`/`int` kind** — a `str` value
containing abbreviation operators (`>`, `+`, `:`, …) would be re-interpreted as
grammar and produce a parse error at render time (surfaced as
`cannot expand composite …`).

### The merge (insertion point)

The base pieces, the selected options, and the freeform children all become
**siblings at one insertion point**, in that order:

> The insertion point is the **children of the last top-level node** of the
> parsed `expands`.

For `class:${name}>(init+str+repr)` the single top-level node is `class`, so the
extras become further children of `class` — siblings of `init`/`str`/`repr`
inside the class body. When the base is a single leaf (`class:${name}` with no
`>`), the extras simply become that node's children. If the last top-level node
is itself a leaf snippet with no `${child}` slot, the extras append after its
lines (the engine's existing no-slot behaviour).

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

The builtin `classp` in the Python pack passes `check` (7 languages, 265
triggers).

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

## Open questions / future work (designed, not built)

- **Tabstop-preserving params.** Today a composite param renders as literal
  text (or its default). A future refinement could let an *unbound* param drop
  through to the underlying snippet's own tabstop (so bare `classp` gives
  `class ${1:Name}` rather than literal `class Name`) by omitting the arg from
  the generated abbreviation rather than substituting the default. This needs
  argument-level (not string-level) substitution to stay robust across
  comma-separated args.
- **Explicit `${child}` slot in `expands`.** The insertion point is currently
  implicit ("last top-level node"), which covers the common scaffold cleanly. A
  dedicated slot marker would let a composite place the extras somewhere other
  than the last node (e.g. into an inner group). The sketch: substitute the
  marker to a reserved sentinel node, then splice the extras at that node's
  position after parsing.
- **`str`-value injection safety.** Rather than documenting "use `ident`
  params", the renderer could quote/escape substituted `str` values so operator
  characters survive re-parsing verbatim.
- **Error origin.** Composite validation reports the offending trigger but a
  generic `in composite` origin rather than the source file; threading the pack
  origin onto the compiled snippet would sharpen the message.
- **LSP completion of option names.** Options are discoverable via
  `list`/docs today; a completion provider could offer them after the composite
  trigger's comma.
