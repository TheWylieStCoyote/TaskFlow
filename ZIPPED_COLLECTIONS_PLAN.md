# Zipped collections — design (and an honest premise check)

The pitch: iterate two collections in parallel — straw syntax `*@names@types`
— so a function signature or field list can be generated from paired
name/type lists, instead of nesting producing a cross product. The pitched
flagship:

```
field*@names@types    --repeat names=x,y --repeat types=i32,f64
```

```rust
x: i32,
y: f64,
```

**This document's first job is to stress-test that premise**, because the item
model *already* pairs data within one collection — and the conclusion below is
that zip, as a grammar feature, is mostly redundant. The real feature is a
richer item model, plus a data-layer spelling of "attach a second list as a
member". The recommended flagship becomes:

```
struct:W>field*@P    --repeat P=host,port --member P.type=String,u16
```

```rust
struct W {
    host: String,
    port: u16,
}
```

— same two-lists-in, zipped-out ergonomics as the pitch, zero new abbreviation
grammar.

## Premise check: what pairing already exists

Today, one collection already carries paired data
(`crates/codemmet-core/src/context.rs`):

- `Item` is `{ value: String, members: BTreeMap<String, String> }`.
- `--repeat P=x:i32,y:f64` parses each `value:type` into an item whose `type`
  member is the type (`Collection::parse_values`; the split is `split_once`,
  so `e:std::io::Error` keeps its `::`s).
- Bodies read both axes: `${value}` and `${value.type}` (the builtin `field`
  snippet in all packs that define it: rust, c, go, python, typescript).
- Buffer extraction of struct/record fields produces exactly this shape
  (`crates/codemmet-tsctx/src/extract.rs`): field name as the value, declared
  type as the `type` member.

So the pitched use — "field list from paired name/type lists" — **works
today** whenever the pairing can be expressed in one collection:

```
$ codemmet expand --lang rust "struct:W>field*@P" --repeat P=x:i32,y:f64
struct W {
    x: i32,
    y: f64,
}
```

What zip covers that this does not, candidate by candidate:

1. **Two independently-sourced buffer collections.** E.g. an enum's variants
   zipped with an in-scope list of handler names — the enum and the list are
   separate constructs, so no single extractor pairs them. **Real gap**, but
   narrow and fragile: correctness rests on *positional* correspondence
   between two definitions that nothing keeps aligned; reorder either and the
   expansion is silently wrong. The `FillTarget` code-action seam is arguably
   the better long-term home for cross-construct generation.
2. **Reusing one axis across several expansions.** Already covered: a named
   collection can drive any number of `*@name` repeats within one
   abbreviation, and nested repeats reach the enclosing axis via `${outer}`.
   Zip adds nothing here.
3. **CLI ergonomics when the data arrives as two lists** (e.g. from a
   script). **Real gap**, but it is a *data-layer* problem: the caller has
   two columns and today must pre-zip them shell-side
   (`paste -d: names types | paste -sd,`) into `value:type` form. Nothing
   about it requires abbreviation syntax.

Meanwhile the item model itself is visibly the bottleneck (a prior review
flagged exactly this):

- Items carry exactly **one** usable member in practice (`type`); the
  `value:type` spec syntax cannot express a second one.
- Extractors leave data on the floor: TS `x?: number` optionality and
  `readonly`, Rust field visibility (`pub`), Python field defaults (`x: int
  = 0`), C enumerator values (`A = 3`), Go struct tags — and Go iota const
  groups are not extracted at all (promised in `CONTEXT_CODEGEN_PLAN.md`,
  absent from `extract.rs`).
- Rust enum variants carry **no members**: the payload shape is baked into
  the value string (`Write(..)`), so a body cannot get the bare name.

**Verdict: pivot.** Enrich the item model and give "attach a parallel list as
a named member" a data-layer spelling. Reserve — but do not spend — a grammar
spelling for in-buffer zip, pending demonstrated demand for gap 1.

## Options: zip vs member-merge vs item-model enrichment

### (a) True zip: `*@names@types`

Bodies address the second axis positionally (`${value.2}`) or by collection
name (`${value.types}`).

- Verified safe to claim: `field*@names@types` is a parse error today
  (`unexpected '@' at byte 12..13` — `repeat_spec` in
  `crates/codemmet-core/src/abbr/parser.rs` consumes one `@WORD`).
- But it forks the item model: a copy now has N current items, so **every
  item-consuming mechanism becomes ambiguous** — which axis does `${outer}`
  expose to nested repeats? which axis auto-binds to the first free
  parameter (`compose_node`'s `auto_bind`)? what does `${value}` mean?
  Positional members (`${value.2}`) read badly and collide with nothing
  today only because they are unreadable.
- Spends `@`-in-repeat-tail from the operator budget for a niche case.

### (b) Member-merge in the grammar: `*@names:type=@types`

A second collection attaches as a **named member** of the first, reusing
`${value.type}` machinery wholesale. One item per copy, so `${outer}`,
auto-bind, and `${value}` all stay unambiguous.

- Verified safe to claim: `field*@names:type=@types` is a parse error today
  (`unexpected ':'` after the repeat spec), and the LSP candidate extractor
  (`crates/codemmet/src/lsp/extract.rs`) carries it — candidates split at
  unquoted whitespace, `@`/`=` are not operators, and quotes only open after
  `:`/`,`, which this form never triggers mid-token.
- This is the *right shape* for zip. But as grammar it still only pays off
  for gap 1 (two buffer collections), and it burdens `:` — already the
  busiest character on the surface — with a second meaning inside the repeat
  spec.

### (c) Pivot: enrich the item model; zip becomes a data-layer merge

Three legs:

1. **Multi-member items on the CLI, spelled columnar**: a `--member
   NAME.key=v1,v2,…` flag attaches a parallel value list as member `key` of
   collection `NAME`. This *is* the zip — two lists in, paired items out —
   and it dodges the separator minefield entirely (an inline per-item syntax
   like `x:i32;vis=pub` collides with real types: `[u8; 4]` contains `;`,
   `HashMap<K, V>` contains `,` — the latter already breaks `--repeat`
   today, verified). `value:type` keeps working unchanged.
2. **Richer buffer extraction**: fill the members that are currently dropped
   (`optional`, `vis`, `default`, enum `name`/`value`), per language — see
   the cross-language table below.
3. **Small engine parity fixes** uncovered while probing (arg-position
   `${value.member:default}`, member-absent rendering) — listed under
   Architecture.

**Recommendation: (c),** with (b)'s spelling documented as the reserved
future syntax for in-buffer zip and anti-regression tests keeping it a parse
error until then. Reasoning: (c) covers gaps 2 and 3 outright, upgrades every
existing consumer (`field`, `variant`, future snippets) for free, spends zero
operator characters, and leaves only the narrow, fragile gap 1 on the table —
which (b) can still claim later without any migration, because one-item-per-
copy semantics are identical whether the members came from `--member`, an
extractor, or a future `*@a:k=@b`.

## What is explicitly out of scope

- **Grammar-level zip in this milestone.** `*@names@types` is rejected
  outright; `*@names:type=@types` is reserved (kept a parse error, covered by
  tests) and revisited only if in-buffer cross-construct pairing shows real
  demand.
- **Cross-construct extraction heuristics** (guessing that `handlers[i]`
  corresponds to variant `i`). Positional coupling between separate
  definitions is the user's assertion to make, never the extractor's.
- **Case/format transforms on member values** (`${value.name:lower}`-style).
  Members are stored display-ready instead (see vocabulary).
- **Quoting/escaping overhaul of `--repeat` value lists.** Commas inside
  types (`HashMap<K, V>`) are broken today and stay a documented limitation;
  `--member` sidesteps it for new data but does not fix the old spelling.

## Architecture

All data-layer; the abbreviation grammar and `Repeat` enum are untouched.

- **`Item` is already sufficient** — `members` is a free-form
  `BTreeMap<String, String>`. No core type changes.
- **CLI (`crates/codemmet/src/cli/expand.rs`)** gains `--member
  NAME.key=v1,v2,…` (repeatable). Parsing: split the spec at the first `=`,
  split the left side at the first `.` into collection name and member key,
  then reuse the value-list split. Resolution happens after all `--repeat`
  flags are parsed (so flag order does not matter); referencing an unknown
  collection is an error. Empty slots (`--member P.type=i32,,f64`) mean
  "member absent on that item".
- **Spec parsing lives next to `Collection::parse_spec`** in
  `crates/codemmet-core/src/context.rs` (e.g. `Collection::attach_member`),
  so the web playground and any future editor surface get it for free, and
  the length-mismatch policy is enforced in exactly one place.
- **Extraction (`crates/codemmet-tsctx/src/extract.rs`)** grows per-language
  member capture (table below). No signature changes — extractors already
  return `Item`s.
- **Engine parity fixes** (small, in `crates/codemmet-core/src/snippet/render.rs`):
  - *Arg-position `${value.member:default}` is silently broken today*:
    `item_token` treats everything after the `.` as the member key, so
    `fn:${value.handler:cb}*@P` looks up a member literally named
    `"handler:cb"` and renders empty. Body position parses the combined form
    correctly (`template.rs`). Fix `item_token` to split the member at `:`.
  - *Member absent on a present item renders the default as literal text*
    (verified: `--repeat P=x:i32,y` renders `y: Type,` — plain text), while a
    missing *item* renders a tabstop. Proposal: member-absent-with-default
    also renders a **tabstop** carrying the default, so partially-zipped data
    leaves holes the user can tab through. This changes existing goldens;
    called out as an open question.

## Grammar & back-compat

Nothing is added to the abbreviation surface. Verified anti-regression facts
(to be pinned by parser tests):

- `a*@names@types` → parse error (`unexpected '@'`).
- `a*@names:type=@types` → parse error (`unexpected ':'`).
- `OPERATOR_CHARS` is unchanged; the LSP extractor's whitespace/quote logic
  is untouched.
- Operator budget: this design spends **zero** characters. If (b) is later
  adopted, it spends `:`+`=` *inside* the repeat spec only — both already
  operators/plain chars the extractor carries — rather than a new character.

`--member`'s `.` and `=` live entirely in CLI flag values, outside the
grammar.

## Mismatch policy

Applies to `--member` lists vs. their collection's length (and to any future
zip form). Options:

- **Shortest wins** — silently drops items; hides off-by-one bugs in the
  producing script. Rejected.
- **Pad with fallback** — shorter member list ⇒ trailing items lack the
  member ⇒ default/tabstop. Graceful, but a length mismatch on the CLI is
  almost always a bug in explicit, cheap-to-fix data.
- **Error on mismatch** (recommended) — `--member P.type=…` must supply
  exactly as many slots as `P` has items; **empty slots are the deliberate
  escape hatch** for sparse members (`--member P.vis=pub,,pub`). Fail fast
  where the data is explicit; degrade gracefully only where data is implicit
  (buffer extraction, the existing no-data single-copy fallback — both
  unchanged).

## Member vocabulary

Free-form keys remain allowed (the map is free-form; `${value.key}` already
resolves any key). The following names are **blessed** — extractors emit
them, builtin snippet bodies may rely on them, docs teach them:

| key | meaning | example values |
| --- | --- | --- |
| `type` | declared type (existing) | `i32`, `string` |
| `optional` | optionality marker, display-ready | `?` (TS), empty/absent |
| `vis` | visibility/qualifier prefix, display-ready | `pub `, `pub(crate) ` |
| `default` | initializer text | `0`, `"x"` |
| `name` | bare name when `value` is a richer form | `Write` for value `Write(..)` |
| `value` | an enum member's assigned value | `3` (C), `"n"` (TS) |

**Display-ready convention** (recommended): members that render as optional
prefixes/suffixes carry their own spacing/punctuation (`vis` = `"pub "`,
`optional` = `"?"`), because a body like `${value.vis}${value:name}` has no
conditional-space mechanism and an absent member already renders as nothing.
Precedent: `rust_variant_pattern` bakes `(..)` into the value. Consequence:
`--member` value lists must not trim slots aggressively (leading trim only,
or none) — flagged as an open question.

`--repeat`'s `value:type` syntax is **unchanged** — it stays the one-member
sugar; additional members come columnar via `--member`. Inline `*[…]`
literals likewise stay single-member (`*[x:i32,y:f64]`); a multi-member
inline syntax has no safe separator (see (c) above) and inline literals are
for quick stamps, not rich data.

## Interactions

- **Nesting + `${outer}`**: unchanged — every copy still has exactly one
  item (with more members), so "the outer item" stays well-defined. This is
  the decisive structural argument for member-merge/enrichment over true
  zip, where `${outer}` would need an axis qualifier.
- **Auto-bind**: unchanged — the item's `value` auto-binds to the first free
  parameter exactly as today (`compose_node`); members never auto-bind. True
  zip would have had to invent a rule for which axis binds.
- **Inline literals**: no zip of inline literals exists by construction (no
  syntax); `*[…]` semantics untouched.
- **No-data degradation**: unchanged — a missing collection still degrades
  to a single copy with `${value…}` tabstops; `--member` on a missing
  collection is a CLI error (explicit data).
- **Composites**: data-driven repeats remain rejected on composites
  (`expand_composite` guard in `render.rs`) — inherited unchanged, since
  nothing here touches `Repeat`.

## Cross-language extraction notes

What each language's extraction yields today, and the members IM1 adds:

| language | today | IM1 adds | still on the floor |
| --- | --- | --- | --- |
| Rust | enum variants as match-ready patterns (`Write(..)`, **no members**); struct fields `value`+`type` | variants: `name`; fields: `vis` (from `visibility_modifier`, display-ready) | attributes, generic bounds, payload field types |
| C | enumerator names; struct/union fields `value`+`type` (pointer/array declarators handled) | enumerators: `value` (`A = 3` → `3`) | bitfields, anonymous members |
| C++ | same pass as C (`c_defs`) | same as C | class members, access sections (`public:`), default member initializers |
| TypeScript | enum members; `type T = "a" \| "b"` literal unions; interface fields `value`+`type` | fields: `optional` (`?`), `readonly`; enum members: `value` (initializer) | classes, method signatures, index signatures |
| Go | struct fields `value`+`type` (multi-name `X, Y int` handled); **no enums** | iota const groups as enum collections (closing a `CONTEXT_CODEGEN_PLAN.md` promise); fields: `tag` | embedded fields, exportedness-as-`vis` (debatable: casing already encodes it) |
| Python | `Enum` class members (assigned names); annotated class attrs `value`+`type` | fields: `default` (`x: int = 0` → `0`); enum members: `value` | properties, `__init__`-only attributes |
| JavaScript | list bindings only (no type definitions to mine) | — | everything; JS stays list-only by design |

In-scope list bindings (all languages) stay member-less element text.

## Verification

- **Goldens via the CLI** (deterministic, the CX0 pattern):
  - the flagship: `--repeat P=host,port --member P.type=String,u16`;
  - sparse members via empty slots; member-absent → tabstop-with-default
    (if adopted) or current literal-default (if not);
  - mismatch errors: too-few/too-many `--member` slots, unknown collection,
    malformed spec (`--member P=…` without a key, empty key);
  - multi-member items flowing through nesting, `${outer.key}`, and
    arg-position `${value.key}`.
- **Parser anti-regression tests**: `a*@x@y` and `a*@x:k=@y` remain parse
  errors with stable messages — the reserved spelling stays free.
- **Engine tests**: `item_token` accepts `${value.member:default}` in arg
  position (the parity fix), with a regression test on the old
  silently-empty behavior.
- **Extraction fixture tests** per language for each new member (the M3
  cursor-marker pattern): TS `?`, Rust `pub`, Python defaults, C enumerator
  values, Go iota.
- **Property/fuzz**: `--member` spec parsing and member-heavy items
  (adversarial keys/values, huge maps) never panic and respect budgets.

## Milestones

- **IM0 — spec + engine (no tree-sitter).** `Collection::attach_member` +
  the `--member` flag with the strict mismatch policy; the `item_token`
  arg-position parity fix; decide and land the member-absent rendering
  policy. Golden-tested through the CLI. *First usable slice — this alone
  delivers the pitched two-lists flagship.*
- **IM1 — extraction enrichment**, per language (Rust `vis`/`name` and TS
  `optional` first — highest value, simplest queries; Go iota is the largest
  single item). Fixture tests per member.
- **IM2 — snippet + docs uptake.** Update builtin `field`/`variant` bodies
  where a blessed member improves output (TS `${value.optional}`, Rust
  `${value.vis}`); document the vocabulary and `--member` in
  `docs/context-codegen.md`; playground input for members.
- **IM3 — reassess in-buffer zip.** With enriched items dogfooded, decide
  whether gap 1 (two buffer collections) justifies the reserved
  `*@names:type=@types` grammar — or whether `FillTarget` code actions cover
  it better. No code in this plan.

## Risks and open questions

- **Member-absent rendering** (the biggest behavioral call): keep today's
  literal-default text, or switch to tabstop-with-default? The switch is
  better for partially-zipped data but changes existing goldens and strip-
  mode output. Leaning: switch, with a CHANGELOG note.
- **Trimming vs display-ready members**: `--member` slots need to preserve
  meaningful trailing space for `vis`-style members, but untrimmed slots
  surprise casual callers. Options: trim leading only; or bless `vis` as the
  one key extractors emit with trailing space while CLI callers write bodies
  accordingly. Needs an owner decision before IM0 freezes the spec.
- **`--member` vs extending `--repeat`** (`--repeat P=@names:type=@types`):
  one flag fewer, and it visually previews the reserved grammar form — but
  it overloads `--repeat`'s value position with reference semantics (`@`
  prefix) and reopens separator questions. Leaning: separate `--member`.
- **Blessed-key drift**: extractors across 7 languages must agree on member
  semantics (`optional` as marker text vs boolean-ish `"true"`). The
  display-ready convention answers this, but each IM1 extractor PR should
  cite the table above.
- **Reserved-syntax rot**: keeping `*@a:k=@b` "free" costs two parser tests
  forever; cheap, but the reservation should be re-justified (or dropped) at
  IM3 rather than lingering.
