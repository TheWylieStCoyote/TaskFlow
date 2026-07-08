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

### `${name}` substitution — argument pass-through

A `${name}` marker in `expands` (or an option) is an **argument position** for a
sub-trigger (e.g. `class:${name}`). It is resolved by **passing the composite's
argument through** to that sub-trigger, so the underlying snippet handles it
natively — which means an unbound param becomes a real *tabstop*, not literal
text:

- a **bound** param (the caller supplied it) → the value is emitted as that
  argument: `classp:Point` → `class:Point` → renders `class Point`;
- an **unbound** param → the argument is **dropped**, so the sub-trigger's own
  parameter default surfaces: bare `classp` → `class` (no arg) → renders
  `class ${1:Name}` in keep mode — a tabstop you can tab into. The declared
  default is **not** substituted as text.

```sh
codemmet expand --lang python --tabstops keep "classp"        # class ${1:Name}$2: …
codemmet expand --lang python --tabstops keep "classp:Point"  # class Point$1: …
```

The same rule applies to every `${name}` and to option fragments; a name used in
several sub-triggers passes through to each. Tabstop renumbering stays global
across the whole merged tree, so the passed-through tabstops interleave
correctly with the base pieces, options, and freeform children.

**Mechanism.** Substitution can't drop an argument at the string level
(`class:` does not parse), so markers are resolved on the *parsed* tree: each
`${name}` is first replaced by a parse-safe placeholder, the fragment is parsed,
then each placeholder that is a **whole argument** is either replaced by the
bound value or removed. Because values are spliced back through the abbreviation
grammar, **composite params should be `ident`/`int` kind** — a `str` value
containing abbreviation operators (`>`, `+`, `:`, …) would be re-interpreted as
grammar and produce a render-time parse error (`cannot expand composite …`).

**Non-argument markers.** A `${name}` that is *not* a whole argument (embedded
in a longer arg, or glued into a word) has no argument slot to drop, so it falls
back to value-or-default **text** substitution. Composite params are meant for
argument positions; the text fallback keeps odd shapes working rather than
failing.

**Positional gap (documented limitation).** The abbreviation grammar has no
empty positional slot, so an **unbound argument that sits *before* a later
argument** in the same sub-trigger's list cannot be expressed. Example:
`fori:${var},${end}` with `var` unbound but `end` bound (e.g. via a glued int).
This is **rejected with a clear error** rather than silently mis-binding:

```text
unbound parameter `var` leaves a positional gap before a later argument;
declare optional params last, or pass `var`
```

The clean shapes — a single param, or several params where the unbound ones are
**trailing** (`fori:${var},${end}` with `end` unbound drops cleanly to its
tabstop) — all work. The rule: put optional (droppable) params last.

Because the pass-through resolves the composite param to the sub-trigger's
argument, a composite param's own `default` is only observed for the
non-argument text fallback; in an argument position the *sub-trigger's* default
governs the unbound case.

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

- **Interior positional gaps.** An unbound argument *before* a bound one in the
  same sub-trigger arg list is rejected (see "Positional gap" above) rather than
  handled. Handling it would need either named arguments or an empty-slot syntax
  in the abbreviation grammar; the trailing-optional convention avoids it in
  practice.
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
