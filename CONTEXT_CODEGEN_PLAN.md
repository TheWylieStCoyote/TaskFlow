# Context-aware code generation (data-driven expansion) — design

Today codemmet expands templates: `arm*3` renders three identical match arms.
This adds **data-driven expansion** — the repetition count *and* the per-copy
values come from the surrounding code: in-scope list literals, the variants of
an enum, or the fields of a struct. The flagship:

```
match:Msg>arm*        (in a Rust buffer where `enum Msg { Quit, Move, Write }`)
```

```rust
match msg {
    Msg::Quit => ,
    Msg::Move => ,
    Msg::Write => ,
}
```

It reuses the existing tree-sitter seam: `ContextProvider` already receives the
whole document and the parse tree — it just classifies *scope* today; this
teaches it to also extract *data*.

## Decisions

- **Sources (all three):** in-scope list/array literals, enum variants, and
  struct/record fields.
- **Syntax (both forms):** a bare `*` repeats over the subject named by the
  enclosing construct (`match:Msg>arm*`); an explicit `*@name` repeats over a
  named source (`arm*@Msg`, `let:x*@items`). `*n` (literal count) is unchanged.
- **Languages:** all seven built-ins (Rust, Python, C, C++, JavaScript,
  TypeScript, Go) — each needs its own extraction queries.
- **Headless:** a CLI `--repeat name=v1,v2,…` hatch supplies collections
  explicitly, so the feature is scriptable, demoable, and covered by
  deterministic golden tests — not only reachable through a live editor.

## What is explicitly out of scope

- **Type inference.** `match x` where `x: Msg` requires resolving `x`'s type —
  that is rust-analyzer's job, not tree-sitter's. The design **names the
  source** (`match:Msg`, `arm*@Msg`) rather than inferring a variable's type.
- **Cross-file / cross-crate resolution.** Extraction is same-file and
  syntactic; macro-generated members are invisible.

Both limits are stated in the user docs.

## Architecture

Keep the engine pure and document-free. The provider (or the CLI) gathers
**collections** into the `ExpansionContext`; the engine only reads them.

- **`Collection`** — a named, ordered list of items:
  ```rust
  pub struct Collection { pub name: String, pub items: Vec<Item> }
  pub struct Item {
      pub value: String,                 // primary text: variant / element / field name
      pub members: BTreeMap<String, String>, // extras: e.g. {"type": "f64"} for a field
  }
  ```
- **`ExpansionContext`** gains `collections: BTreeMap<String, Collection>`
  (keyed by name). The engine resolves a data-driven repeat by looking a name
  up here. In editors the provider fills it from the buffer; on the CLI
  `--repeat` fills it; everywhere else it is empty.
- **`ContextProvider`** grows the extraction. `context_at` already walks the
  tree from the cursor; it additionally collects the in-scope list literals,
  enum definitions, and struct definitions it can see, keyed by name. (Eager
  collection keeps the engine from calling back into the provider mid-render.)

## Grammar

Extend the repeat suffix:

```
element  := node ( '*' repeat )?
repeat   := NUMBER            ; literal count (unchanged)
          | '@' WORD          ; explicit source: repeat over the named collection
          | ε                 ; bare '*': repeat over the inferred parent subject
```

- **Parent-subject inference:** an element whose first argument names a
  resolvable collection sets the *subject* for the group attached by its `>`.
  In `match:Msg>arm*`, `match:Msg` makes `Msg` the subject, so the bare `*` on
  `arm` repeats over it. The subject is the nearest such ancestor.
- **Value placeholders** in snippet bodies and arguments:
  - `${value}` — the current item's primary text.
  - `${value.KEY}` — a member (e.g. `${value.type}` for a struct field).
  - `${i}` — the 1-based index (already implemented for `*n`).
- **Missing data** (name unresolved, or no editor/CLI data): a data-driven
  repeat degrades to a **single copy** with `${value}` left as a tabstop, so the
  user still gets a usable scaffold rather than an error. (Open question below.)

## Data extraction (per source × language)

The bulk of the work: 3 sources × 7 languages, driven by per-language
tree-sitter query tables (mirroring the existing scope-classification tables).

- **In-scope list literal** — the nearest binding `name = [ … ]` (or `vec![…]`,
  `[]T{…}`) visible above the cursor; items = element texts. Simplest and fully
  syntactic; the recommended first extractor.
- **Enum variants** — `enum Name { … }` (Rust/C/C++/TS), `type T = A | B` (TS
  unions), `class E(Enum)` (Python), `const (… iota)` (Go). Items = member
  names; `members` may carry the variant's payload shape.
- **Struct/record fields** — `struct`/`class`/`interface`/dataclass fields;
  items = field names with `members["type"]` where available.

Each extractor is best-effort and same-file; unknown constructs yield no
collection (→ the missing-data fallback).

## CLI hatch

```
codemmet expand --lang rust "match:Msg>arm*" --repeat Msg=Quit,Move,Write
codemmet expand --lang rust "arm*@items"     --repeat items=Foo,Bar
```

- `--repeat NAME=v1,v2,…` (repeatable) supplies simple collections.
- Structured members use `NAME=field:type;field:type` (e.g.
  `--repeat Point=x:f64;y:f64`), parsed into `Item.value` + `members`.

## Milestones

Staged so a usable slice ships before all 21 extractors exist.

- **CX0 — grammar + engine + CLI (no tree-sitter).** Parse `*`, `*@name`;
  add `Collection`/`Item` and `collections` to `ExpansionContext`; compose a
  data-driven repeat over a supplied collection with `${value}`/`${value.KEY}`;
  the `--repeat` flag; the missing-data fallback. Golden-tested entirely
  through the CLI. **This is the first usable, demoable slice.**
- **CX1 — in-scope list extraction**, all seven languages, wired through the
  LSP provider; fixture-file + cursor tests.
- **CX2 — enum variants**, all seven; the `match:Enum>arm*` flagship, including
  qualified value form (`Enum::Variant`) and a minimal treatment of payload
  variants.
- **CX3 — struct/record fields**, all seven; `${value.type}`; ship example
  snippets that use it (a builder, a `Debug`/serialize scaffold).
- **CX4 — docs + polish.** A `docs/context-codegen.md` guide, examples linked
  from the cookbook, an optional collections input in the web playground, and a
  `PLAN.md` note.

## Testing

- **Golden snapshots via the CLI `--repeat` hatch** — deterministic, no editor,
  covering the grammar, `${value}`, members, nesting, and the fallback.
- **Extraction tests** — fixture files with a cursor marker → expected
  collection, one set per language per source (the M3 scope-test pattern).
- **Property/fuzz** — data-driven repeats with adversarial collection data
  (empty, huge, odd characters) never panic and respect the size budget.

## Risks and open questions

- **Extraction breadth.** 21 extractors is the real cost; each language's
  syntax for the three sources differs. Mitigated by shipping CX0+CX1 first and
  adding sources incrementally.
- **Missing-data fallback.** Single-copy-with-tabstop vs. zero copies vs. a
  soft error — CX0 picks one; revisit after dogfooding. (Leaning:
  single-copy-with-tabstop.)
- **Value form for enums.** Bare `Quit` vs. qualified `Msg::Quit` vs. a binding
  pattern `Msg::Move(..)` for payload variants — needs a per-language default.
- **Ambiguity / shadowing.** Two collections with the same name → nearest
  in-scope wins; document the rule.
- **Grammar interactions.** Bare `*` (inferred) vs. `*n` vs. `*@name`, and how
  `^` climb-up composes with a data-driven repeat — covered by parser tests.
- **Editor-only for real buffers.** Extraction needs the tree-sitter provider;
  the bare CLI relies on `--repeat`. Consistent with the existing
  `NullContextProvider` split.
- **Nested collections** (a list of structs; per-item sub-repeats) — deferred.
