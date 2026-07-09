# Case transforms on values — design

Data-driven expansion feeds real identifiers into templates, but it pastes
them verbatim — and source data almost never arrives in the case the target
position wants. Today:

```
$ codemmet expand --lang rust "impl:P>fn*[maxSize,minSize]"
impl P {
    fn maxSize(&self) { … }     # wrong: Rust wants fn max_size
    fn minSize(&self) { … }
}
```

This adds **case transforms**: a `|name` suffix inside `${…}` references that
re-cases the referenced text. The flagship:

```
impl:P>fn:${value|snake}*[maxSize,minSize]
```

```rust
impl P {
    fn max_size(&self) { … }
    fn min_size(&self) { … }
}
```

and the pattern it unlocks in snippet bodies — getters/setters, impossible
today because a field name cannot be re-cased:

```toml
[snippets.setter]
params = [{ name = "field" }, { name = "ty", default = "()" }]
body = "fn set_${field|snake}(&mut self, v: ${ty}) { self.${field|snake} = v; }"
```

`setter:${value},${value.type}*@Config` then stamps one correctly-cased setter
per struct field. This is the classic snippet-engine feature (TextMate/VSCode
have `/upcase`, `/pascalcase`, and regex transforms); codemmet gets the named,
non-regex subset that covers code generation.

Everything below is grounded in the current code; behaviors marked *verified*
were exercised against this tree.

## Decisions (each: options + recommendation)

### 1. Syntax: `|name` inside `${…}`, placed before `:default`

Options considered:

- **(a) Pipe suffix — recommended.** `${value|snake}`, `${field|snake}`.
  The candidates are constrained by what is already taken inside `${…}`:
  `.` selects a member (`${value.type}`, `template.rs` value/outer branch) and
  `:` introduces a default (`${value:default}`), so neither can introduce a
  transform. `|` is the TextMate-family convention for "filter" and reads
  well. Shell interaction is a non-issue: abbreviations containing `>` must
  already be quoted on a command line, and a quoted `|` is inert.
- **(b) Function style** `${snake(value)}` — no precedent in any snippet
  dialect, adds parens to the template grammar, and reads inside-out for the
  common "reference, then adjust" mental model. Rejected.
- **(c) VSCode regex transforms** `${value/([A-Z])/_$1/g}` — maximally
  powerful, but unreadable, needs a regex engine (a new dependency in a
  dependency-light core), and this repo's template parser deliberately rejects
  nested `$`-forms (`error_nested_placeholder` in `template.rs`). Named
  transforms cover the actual need (case conventions). Rejected.

**Composition order** — the token grammar is `source → .member → |transform →
:default`:

```
${value|snake}   ${value.type|snake}   ${value|snake:fallback}   ${field|snake}
```

The transform sits **before** the default, not after it. Two reasons:

- **Back-compat (verified):** `${value:max|snake}` parses *today* — the `|` is
  literal default text (`parse_placeholder` treats everything up to `}` as
  literal; verified: it renders `max|snake`). Putting the transform after the
  default would re-interpret existing packs' defaults. Before the `:`, the
  default remains a raw region and nothing that parses today changes meaning.
- **Semantics:** the transform is a property of the *value source* ("the item,
  snake-cased"); the default is fallback display text the author writes
  directly in whatever case they want. Keeping them separated keeps one rule
  (below): transforms apply to data, never to author-written literals.

**Rule: transforms apply only to caller/data-supplied text.** The rendered
item value, member value, or bound argument is transformed. An author-written
`:default` (and a param's declared `default`) renders verbatim — the author
already controls its spelling, and this keeps the unbound-param story (§4)
simple: what you see in the placeholder is exactly what the author wrote.

### 2. Transform set: six names, one shared tokenizer

Recommended v1 set (small, closed, all total functions — any input produces
*some* reasonable output):

| name     | result for `maxSize` | definition |
|----------|----------------------|------------|
| `snake`  | `max_size`           | tokens lowercased, joined with `_` |
| `kebab`  | `max-size`           | tokens lowercased, joined with `-` |
| `pascal` | `MaxSize`            | each token capitalized (first char upper, rest lower), joined |
| `camel`  | `maxSize`            | `pascal` with the first token all-lowercase |
| `upper`  | `MAX_SIZE`           | tokens uppercased, joined with `_` (SCREAMING_SNAKE) |
| `lower`  | `maxsize`            | plain lowercase fold of the whole string, no re-joining |

`upper` meaning SCREAMING_SNAKE (not a dumb fold) is the deliberate choice:
that is what code generation wants for constants; a plain uppercase fold has
no code-generation use. `lower` *is* the dumb fold (useful for `Red` → `red`
enum-variant-to-literal cases). If the owner prefers symmetric folds, the
alternative is `scream` for SCREAMING_SNAKE and `upper`/`lower` as folds — but
six names that each map to one convention is the smaller vocabulary.

**Tokenization** (shared by all except `lower`), defined precisely so tests
can pin it:

1. Strip and remember a run of **leading underscores**; re-attach it verbatim
   to the result (`_maxSize` → `_max_size`, `|pascal` → `_MaxSize` — Rust's
   `_unused` convention survives).
2. Split at every `_`, `-`, and whitespace character (separator dropped).
3. Within a run of letters/digits, split before an **upper→anything-lower**
   boundary and after an **acronym run**: a lowercase/digit followed by an
   uppercase letter starts a new token (`maxSize` → `max`,`Size`; `utf8String`
   → `utf8`,`String`), and an uppercase run followed by uppercase+lowercase
   splits before the last capital (`HTTPServer` → `HTTP`,`Server`;
   `XMLHttpRequest` → `XML`,`Http`,`Request`).
4. Digits never start a split on their own (`vec3` is one token; `Vec3D` →
   `Vec3`,`D` by rule 3).
5. Empty tokens (doubled separators) are dropped.

Case mapping is **ASCII-only** in v1; non-ASCII characters pass through
unchanged and count as lowercase letters for boundary detection. Identifiers
are overwhelmingly ASCII, and this keeps the module dependency-free.

### 3. What can carry a transform: item values and named params

- **`${value…}` / `${outer…}`** (`Span::ItemValue`) — yes; this is the
  motivating case and the text is always known at render time.
- **Named params** (`Span::Param`) — yes. Params are the mechanism behind
  getters/setters: `set_${field|snake}` with `setter:maxSize` must work, and a
  bound param is known text (`instantiate_param`'s `Binding::Bound` arm in
  `render.rs`). The unbound case degrades (§4).
- **`${i}`** — no. It is a number; every case transform is the identity on
  digits, so allowing `${i|snake}` is pure noise. Template parse error.
- **Numbered tabstops** (`${1|snake}`) — no. A tabstop is *always* future
  user typing; there is never known text to transform (§4). Template parse
  error, which also keeps the numbered-form grammar untouched.

### 4. The hard problem: unbound and unknown text

A transform of known text is trivial. But an **unbound param** renders as a
tabstop the user types into later (`instantiate_param`, the `Free` arm) — the
engine emits text once and cannot transform future keystrokes. Options:

- **(a) Known-text only — recommended.** A transform applies when the text is
  known (data-driven item, member, bound argument). An *unbound* transformed
  param renders exactly as it does today: a tabstop (mirrored across
  occurrences, same `StopId`), placeholder = the declared default, verbatim.
  The transform is simply inert. Documented loudly in `docs/snippets.md`.
- **(b) Emit LSP regex-transform syntax on mirrors** (`${1/…/…/}` with
  `/downcase`-style format modifiers) where the client might apply it.
  Rejected for v1 on grounds verified in this tree and checked upstream:
  - The engine has one output per expansion; LSP defines **no client
    capability** advertising transform support, so the server cannot branch.
  - VSCode applies transforms, but **Neovim's `vim.snippet` parses and then
    strips them** (the tabstop survives, the transform vanishes) — and this
    repo's own editor integration calls `vim.snippet.expand()`
    (`lua/codemmet/lsp.lua:151`). Our flagship client would silently drop the
    feature. LuaSnip applies them only when the optional `jsregexp` library is
    installed. (Checked against the Neovim snippet-grammar source and the
    Neovim snippet-API roadmap issue #25696, July 2026; client behavior may
    have shifted since — worth a re-check at implementation time.)
  - Named transforms would have to be compiled into regex/replacement
    programs; camel→snake requires replacement-side case modifiers that are
    themselves VSCode-dialect. Fragile on top of unsupported.
- **(c) Reject transforms on params that can be unbound** — every param *can*
  be unbound (any caller may omit the argument), so this is equivalent to
  banning transforms on params entirely, killing the getter/setter case.
  Rejected.

**Is the feature still worth it under (a)?** Yes, decisively:

- The data-driven path — the motivating gap — is *always* known text. Items
  from `*[…]`, `*@name`, buffer extraction, and `--repeat` all resolve before
  rendering. `fn:${value|snake}*@Color` and the setter flagship work in full.
- Bound params cover the interactive generation workflow (`setter:maxSize`),
  and composites bind params through to sub-triggers, so `${name|pascal}` in a
  *body* works whenever the composite caller supplied the name.
- The degraded case (`struct ${name|pascal}` + `fn new_${name|snake}`, no
  argument: user types into a mirror with no case linkage) is exactly today's
  behavior — the feature never makes anything worse, and (b) can be layered
  on later behind a config flag without any template-syntax change.

One subtle sub-case: a composite's unbound shared param arrives at a
sub-snippet as a **stop sentinel** (`parse_stop_sentinel` in `render.rs`) —
that is future typing too, so a transform on a param whose binding is a
sentinel is likewise inert (same rule, same reason).

### 5. Argument position: yes

`fn:${value|snake}*@Color` must work — argument position is arguably the
*primary* spelling, because it re-cases at the use site without touching the
pack. `item_token()` in `render.rs` already parses whole-token `${value…}` /
`${outer…}` arguments; it grows the `|transform` suffix and
`resolve_item_args` applies the transform to the resolved text (items are
always known there, so no unbound story exists).

Two verified facts make this low-risk:

- The abbreviation parser already accepts `|` in raw arguments (`ends_raw_arg`
  in `abbr/parser.rs` stops only at `> + * , ( ) ^` and whitespace), and the
  LSP candidate extractor splits at whitespace with quote-awareness
  (`lsp/extract.rs`) — `|` flows through both untouched. No grammar change.
- Today `fn:${value|snake}*[maxSize]` parses but renders the **literal text**
  `fn ${value|snake}()` (verified): `item_token` returns `None` for the `|`
  suffix, so the arg binds as literal garbage. The feature turns silent
  nonsense into meaning; an *unknown* transform name in argument position
  should become a `RenderError` listing the valid names rather than staying
  literal.

Transforms do **not** apply to plain literal args (`fn:maxSize|snake` stays
literal text) — only inside a whole `${…}` token, mirroring how `${value}`
args already work.

### 6. Composability: no chaining in v1

`${value|snake|upper}` is rejected (template parse error, "one transform per
reference"). Reasons: each v1 transform is already a total normalization from
*any* input spelling to one convention, so no chain reaches a target a single
name cannot; chains invite order-dependence puzzles (`|upper|snake` vs
`|snake|upper` differ) for zero expressive gain; and rejecting now is
forward-compatible — accepting chains later is a pure grammar extension.

### 7. Interactions

- **Data-driven repeats and `${outer}`.** `${outer|snake}` works identically
  (`ItemValue { outer: true, … }` shares the render path). The shipped
  `mtest` snippet (`fn ${outer:group}_${value:case}()`, see
  `docs/context-codegen.md`) becomes robust to CamelCase collections as
  `fn ${outer|snake}_${value|snake}()`.
- **Auto-bind.** When a body doesn't consume `${value}`, the item auto-binds
  to the first free param (`compose_node` in `render.rs`) — there is no
  `${…}` token in the *abbreviation* to hang a `|snake` on, so the auto-bound
  value is **never transformed**. That is a feature, not a gap: auto-bind is
  the zero-syntax convenience path; wanting a transform is precisely the
  moment to write the explicit form (`fn:${value|snake}*@Meth`). The
  alternative — a transform on the repeat itself (`fn*@Meth|snake`) — was
  considered and rejected: it muddies the abbreviation grammar (`|` would need
  to end raw args) and cannot say *which* param it targets.
- **Composites.** `expands` strings substitute `${name}` markers by exact
  name (`substitute_markers`); `${name|pascal}` inside an `expands`/option
  fragment is **out of scope v1** (it would not match a marker and should be
  rejected by composite validation rather than passed through as text). The
  sub-snippets' *bodies* can carry transforms, and a composite's bound params
  reach them as known text, so the capability is available where it matters.
- **Capture.** Should `codemmet capture` *guess* a transform when one example
  contains both `maxSize` and `max_size`? Out of scope — capture's heuristic
  is deliberately whole-token (`snippet/capture.rs` doc comment), and
  cross-case linking is substring-level inference. Noted as a future
  direction; stated in the capture docs.
- **Trainer and playground.** Pure engine feature: the trainer grades through
  `Engine::expand` and the playground is the same engine compiled to wasm —
  both pick it up with no work beyond a wasm rebuild and (optionally) a late
  trainer lesson.
- **`codemmet check`.** An unknown transform name in a body is a
  `TemplateError` at pack compile time, so `check` fails with the file, the
  trigger, and the valid-name list — same channel as today's template errors
  (verified: `check` already surfaces `${value|snake}` as a parse error).

### 8. Back-compat (all verified against this tree)

| today | behavior | after this design |
|---|---|---|
| `${value\|snake}` in a body | **parse error** ("expected '}' or ':' in ${value}/${outer}") | becomes the feature |
| `${name\|snake}` in a body | **parse error** ("expected '}' after parameter name") | becomes the feature |
| `${value:max\|snake}` in a body | parses; `\|snake` is **literal default text** | unchanged (transform sits before `:`) |
| `${1:a\|b}` placeholder | literal | unchanged (no transforms on tabstops) |
| `fn:${value\|snake}` argument | parses; renders **literal** `${value\|snake}` | gains meaning (silent-garbage → transform); unknown names error |

No currently-*valid* pack changes meaning; the only behavioral change to
currently-*accepted* input is the argument-position case, where the old
output was unusable literal text.

## What is explicitly out of scope

- **Regex transforms** and VSCode format-string replacement — named
  transforms only.
- **Chained transforms** (`|snake|upper`) — v1 rejects; forward-compatible.
- **Editor-side transforms on unbound params** (option 4b) — revisit behind a
  config flag if client support matures; requires no syntax change.
- **Transforms on `${i}`, numbered tabstops, auto-bound values, and composite
  `expands` fragments.**
- **Capture guessing transforms** from mixed-case examples.
- **Unicode-aware case mapping** — ASCII folds; non-ASCII passes through.

## Architecture

One new pure module plus threaded metadata; no new dependencies.

- **`crates/codemmet-core/src/snippet/transform.rs`** *(new)*:
  ```rust
  pub enum CaseTransform { Snake, Camel, Pascal, Upper, Kebab, Lower }
  impl CaseTransform {
      pub fn parse(name: &str) -> Option<Self>;   // + `names()` for errors
      pub fn apply(&self, text: &str) -> String;
  }
  fn split_words(text: &str) -> (prefix, impl Iterator<Item = &str>)
  ```
  The tokenizer is private to the module; `apply` is the only entry point.
- **IR (`snippet/ir.rs`)**: `Span::ItemValue` gains
  `transform: Option<CaseTransform>`; `Span::Param` becomes
  `{ name: String, transform: Option<CaseTransform> }`.
- **Template parser (`snippet/template.rs`)**: in the `${value…}`/`${outer…}`
  branch, accept `|NAME` after the optional member and before `:`/`}`; in the
  param branch, accept `|NAME` before `}`. Unknown name, a transform on
  `i`/`child`/a digit form, or a second `|` → `TemplateError`.
- **Renderer (`snippet/render.rs`)**:
  - `instantiate` (ItemValue arm): apply to the item/member text; the
    no-item fallback tabstop keeps its verbatim default.
  - `instantiate_param`: apply to `Binding::Bound` values *after* the
    stop-sentinel check (a sentinel is future typing) and after `${i}`
    substitution; `Free` renders as today.
  - `item_token` / `resolve_item_args`: parse and apply the `|transform`
    suffix on whole-token arguments; unknown names surface as
    `RenderError::ArgBinding`-style errors with the valid-name list.
- **Everything else** (LSP, CLI, trainer, playground, wrap) flows through
  unchanged — the feature is entirely inside template parse + render.

## Grammar / schema

Template body reference forms (extending the doc comment in `template.rs`):

```
value_ref := '${' ('value' | 'outer') ('.' KEY)? ('|' TRANSFORM)? (':' DEFAULT)? '}'
param_ref := '${' NAME ('|' TRANSFORM)? '}'
TRANSFORM := 'snake' | 'camel' | 'pascal' | 'upper' | 'kebab' | 'lower'
```

Abbreviation argument tokens accept the same `value_ref` shape (whole-token
only, as today). No TOML schema change: transforms live in body/argument
text, not in `params` declarations.

## Verification

- **Tokenizer/transform unit tests** (`transform.rs`), one table per
  transform over the edge-case inputs: `maxSize`, `HTTPServer`,
  `XMLHttpRequest`, `utf8String`, `Vec3D`, `_leading`, `__double`,
  `already_snake`, `kebab-in`, `SCREAMING_IN`, `x`, `""`, `héllo` (non-ASCII
  passthrough). Plus idempotence properties: `snake(snake(x)) == snake(x)`
  for every transform, and round-trips like `snake(pascal(x)) == snake(x)`.
- **Template parse tests**: every accepted form; rejections (unknown name,
  chained, `${i|snake}`, `${1|snake}`); back-compat pins for `${value:a|b}`
  and `${1:a|b}` staying literal.
- **Golden CLI snapshots** (the existing `golden` harness): the flagship
  `impl:P>fn:${value|snake}*[maxSize,minSize]`; the setter body over
  `--repeat Config=maxSize:usize,minSize:usize`; `${outer|snake}_${value|snake}`
  nesting; member transform `${value.type|snake}`; unbound-param degrade in
  both `--tabstops keep` and `strip`; unknown-transform error text.
- **`check` test**: a pack with `${value|bogus}` fails naming the trigger,
  file, and valid names.
- **Fuzz**: extend the template/render fuzz targets so arbitrary collection
  values hit `apply` — must never panic, must respect the size budget.

## Milestones (if approved)

- **CT0 — transform module.** `transform.rs` with tokenizer + six transforms,
  fully unit-tested. No visible behavior change.
- **CT1 — template syntax.** IR fields, template-parser support for value/
  outer/param transforms, renderer application, `check` errors, golden tests.
  *First usable slice: the setter body works.*
- **CT2 — argument position.** `item_token`/`resolve_item_args` support +
  error path; the flagship abbreviation works end to end.
- **CT3 — docs + polish.** `docs/syntax.md` + `docs/snippets.md` +
  `docs/context-codegen.md` sections (including the loud unbound-param
  caveat), cookbook getter/setter examples, playground wasm rebuild,
  `PLAN.md` note; optionally re-case-hardened builtin snippets (`mtest`).

## Risks and open questions

- **Open (syntax):** transform-before-default (`${value|snake:fallback}`,
  recommended for back-compat) vs after — and confirm defaults render
  verbatim (never transformed). This is the one decision that is expensive to
  reverse once packs exist.
- **Open (vocabulary):** bless the six names, in particular `upper` =
  SCREAMING_SNAKE and `lower` = plain fold — or rename SCREAMING to `scream`
  and keep `upper`/`lower` as symmetric dumb folds.
- **Open (unbound params):** accept the known-text-only degrade (4a) as the
  permanent contract, or explicitly plan a later opt-in for editor-side
  regex transforms (4b) — affects how loudly the docs promise "no case
  linkage while typing".
- **Tokenizer edge rules** (digits, acronym runs, leading underscores) are
  defined above but deserve a skim at review — they are the part users will
  memory-map, and changing them later silently changes rendered output.
- **Argument-position meaning change**: `fn:${value|snake}` stops rendering
  literal text. Judged safe (the old output was unusable), but it is
  technically observable.
