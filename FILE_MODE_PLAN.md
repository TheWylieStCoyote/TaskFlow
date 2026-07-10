# codemmet source files (file mode) — design

Today codemmet expands one abbreviation per invocation. This adds **file
mode**: codemmet becomes a stdin→stdout filter over a multi-line file of
abbreviations — a `.codemmet` source file renders to a code file.

```
cat main.rs.codemmet | codemmet >> main.rs
```

The flagship input (owner-approved sketch):

```
#lang rust
#repeat Msg=Quit,Move,Write

use:std::fmt

enum:Msg>variant*@Msg

impl:Msg>fn:handle
match:self>arm*@Msg
```

renders (verified against the current engine, line by line; the empty body
line is where a stripped `$0` sat — see Q4 on trimming it):

```rust
use std::fmt;

enum Msg {
    Quit,
    Move,
    Write,
}

impl Msg {
    fn handle(&self) {

    }
}
match self {
    Quit => ,
    Move => ,
    Write => ,
}
```

The differentiator is **progressive context**: the output generated so far is
the context buffer for later lines. Once the `enum Msg` block has been
emitted, a later `match:self>arm*@Msg` fills its arms from that enum via the
tree-sitter provider — the `#repeat` header line becomes unnecessary. File
mode turns the editor-only context features into a reproducible, diffable,
pipeable artifact.

## Decisions (already made — design around these)

- **Format: abbreviations + directives.** One abbreviation per line; blank
  lines pass through as blank output lines; `#`-directives (`#lang rust`,
  `#repeat Msg=Quit,Move,Write`) configure expansion.
- **Language resolution: directive > `--lang` flag > filename inference**
  (`codemmet main.rs.codemmet` strips `.codemmet` and maps the inner
  extension through the existing `lang::profile::find` ladder — id, extension,
  LSP id). Error (exit 3) when none resolves.
- **Progressive context.** After each expanded line, the accumulated output is
  the buffer; the tree-sitter provider extracts collections and scope from it
  for the next line.
- **Surface: bare `codemmet` with piped stdin is the filter**, plus a named
  subcommand alias for scripts (recommendation below: `render`).

## What is explicitly out of scope

- **Structural nesting into previous output.** A line never inserts *inside*
  an earlier block; every line appends at the end of the buffer. (Leading
  indentation is textual only — see Format spec.) Re-homing output into an
  enclosing construct is an editor/LSP job, not a filter's.
- **Multi-file output.** One `.codemmet` file → one code stream. A future
  `scaffold` command owns multi-file trees (see Integration points).
- **Per-file language switching.** One `#lang` per file; the output is one
  file in one language.
- **Streaming `--json` output** and any watch/incremental mode.

## Architecture

Keep the engine pure; keep IO and the provider in the CLI. Three pieces:

- **`codemmet-core::filefmt` (new module)** — a pure, IO-free parser from
  file text to a line stream: `Directive { name, value, line_no }`,
  `Abbrev { indent, text, line_no }`, `Blank`, `Comment`. No expansion logic.
  Living in core makes it unit-testable, fuzzable, and reusable by the wasm
  playground later.
- **`crates/codemmet/src/cli/render.rs` (new)** — the driver. Loads the
  registry (existing `Sources` tiers apply unchanged), resolves the language,
  then folds over the line stream:
  1. Maintain `buffer: String` — everything emitted so far.
  2. For an abbreviation line, ask the provider for context at the end of the
     buffer: `provider.context_at(&DocSnapshot { language, text: buffer },
     buffer.len())`. Override `base_indent` with the line's leading
     whitespace; overlay seed collections (precedence below).
  3. `engine.expand(abbr, lang, &ctx, TabstopMode::Strip)`; on success append
     `indent + body + "\n"` to the buffer (note: today's renderer does *not*
     prefix the first line with `base_indent` — the editor's cursor supplies
     it — so file mode prepends it explicitly).
  4. Blank line → append `"\n"`. Directive → update state. Comment → nothing.
  5. Only after every line succeeds, write the whole buffer to stdout.
- **Provider selection** mirrors the LSP split (`crates/codemmet/src/lsp/mod.rs`
  lines 243–247): `TreeSitterProvider` under `feature = "tree-sitter"`
  (default), `NullContextProvider` otherwise. The driver is written against
  the `ContextProvider` trait, so the minimal build compiles the same loop.

**Collection precedence** (mirrors the language ladder, explicit > inferred):
buffer-extracted < CLI `--repeat` < `#repeat` directive. Same-name shadowing
replaces wholesale; a `#repeat` re-declaration takes effect from its line down.

### Progressive-context cost, honestly

`TreeSitterProvider` caches one tree per language keyed by a hash of the full
document text (`crates/codemmet-tsctx/src/lib.rs`, `cache: HashMap<String,
(u64, Tree)>`). Every appended line changes the hash, so file mode pays **one
full reparse per abbreviation line** of a growing buffer — O(L) parses,
O(L²·bytes) worst case. That is fine: generated scaffolds are a few KB and
tree-sitter parses at hundreds of KB/ms, so a 500-line file stays well under
10 ms total. Collection extraction (`extract::collections`) also walks the
whole tree per call — same order, same verdict. The tsctx crate already
documents incremental `InputEdit` sync as deliberately deferred; appends are
the easy case if it ever matters. No work needed for v1.

## Format spec

A `.codemmet` file is UTF-8 text, parsed line by line:

- **Directive** — `#` immediately followed by an identifier: `#lang rust`,
  `#repeat Msg=Quit,Move,Write`. Grammar: `'#' name:[a-z][a-z0-9-]* SP
  value:rest-of-line`. Produces no output.
- **Comment** — `#` followed by whitespace or end of line (`# note`, `#`).
  Dropped from output (a `.codemmet` comment is not a target-language
  comment; `cm:…`-style snippet triggers already exist for real comments).
- **Blank line** — whitespace-only. Emits one `"\n"` (blank output line),
  and the blank is part of the context buffer.
- **Abbreviation** — anything else. Leading whitespace is captured as that
  line's `base_indent` (see Q6); the rest, right-trimmed, is the
  abbreviation.

Details:

- **Encoding:** UTF-8 required; invalid input is an environment error, exit 3
  (same as today's stdin read failure in `expand`).
- **CRLF:** tolerated on input — `\r` stripped at end of line (the precedent
  is `read_abbr`'s `trim_end_matches(['\r', '\n'])`). Output is always LF.
- **Trailing whitespace** on any line is trimmed before classification.
- **Line continuations: none in v1.** Abbreviations are dense by design — the
  `>`-chain composes on one line, and a continuation escape (`\`) would
  complicate line-number diagnostics for near-zero demand. Revisit only if
  real files hit unreadable widths.
- **Extension convention:** `NAME.EXT.codemmet` (e.g. `main.rs.codemmet`,
  `handlers.go.codemmet`); the inner `EXT` drives filename inference. A plain
  `NAME.codemmet` is fine when `#lang` or `--lang` supplies the language.

## Design questions

### Q1 — directive grammar

**Spelling.** `#lang <id>` where `<id>` is anything `lang::profile::find`
accepts (id, extension, LSP id — same as `--lang`). `#repeat NAME=v1,v2,…`
is parsed by the *same* `Collection::parse_spec` as the `--repeat` flag
(`crates/codemmet-core/src/context.rs`) — identical spelling, including
`value:type` members and trailing-comma tolerance. One spec per `#repeat`
line; repeat the directive for more collections.

**Namespace rule (this is the design):** *a directive is the long CLI flag
name with the same value grammar.* `#lang` ↔ `--lang`, `#repeat` ↔
`--repeat`. When the in-flight zip-collections work lands its `--member`
flag, `#member <same spec syntax>` slots in with zero namespace design — as
would a hypothetical `#indent`. (Integration point only; this design targets
master and does not depend on that branch.)

**Comment leader vs hard error — options:**

- (a) `#` is always a comment unless the word is a known directive. Rejected:
  `#repaet Msg=…` silently vanishes and the user debugs wrong output.
- (b) **`#` + identifier must be a known directive (unknown → hard error,
  exit 1 with line number); `#` + whitespace/EOL is a comment. Recommended.**
  Typos fail loudly; freeform comments stay available with one keystroke of
  discipline (`# like this`). This also keeps the namespace open — adding a
  directive later can't change the meaning of an existing file's comments,
  because `#word` comments were never legal.

**Position semantics — options:**

- (a) File-wide regardless of position (two-pass). Rejected: it breaks the
  streaming mental model ("the file renders top to bottom") that progressive
  context establishes, and makes `#repeat` re-declaration meaningless.
- (b) **From-their-line-down. Recommended.** `#repeat` takes effect for
  subsequent lines and a same-name re-declaration shadows from that point.
  `#lang` gets one extra rule: it must appear **before the first abbreviation
  line**, and at most once — a mid-file or duplicate `#lang` is an error
  (exit 1, with line number). One file, one language; the restriction is
  explicit rather than accidental.

### Q2 — progressive-context mechanics

**The buffer and the cursor.** Before expanding line N, the context document
is exactly the output emitted for lines 1..N-1 (including blank lines), and
the virtual cursor sits at the **end of the buffer** — as if the user were
typing the abbreviation on a fresh line at the bottom of the file. That is
the same seam the LSP uses (`context_at(doc, offset)`), so scope
classification and collection extraction come for free with zero provider
changes.

**Top level only — options:**

- (a) Nesting into previous output (a line targets the body of the previous
  block, e.g. via indentation depth). Rejected for v1: it requires locating
  an *insertion point inside* already-emitted text, splicing, and re-indent —
  a structural editor operation, not a filter append. It also destroys the
  append-only buffer invariant that makes progressive context cheap and easy
  to reason about.
- (b) **Every line expands at the end of the buffer (top level). Recommended.**
  Each snippet emits a complete, closed construct, so in practice the scope
  at end-of-buffer is `Scope::Root` and scope-aware variants behave as they
  do at a file's top level. Composite abbreviations (`impl:Msg>fn:handle`)
  already express one level of nesting *within* a line; that is the intended
  way to put a thing inside a thing.

**Scope, precisely defined:** line N's scope is whatever the provider
classifies at `offset = buffer.len()` — no special-casing. Because every
builtin snippet closes its delimiters, this is `Root` today. If a future
snippet emitted an unclosed construct, later lines *would* see its scope;
that is a feature (the definition stays "parse the buffer, ask at the end"),
not something file mode papers over. Collections are extracted per the
existing rules (`crates/codemmet-tsctx/src/extract.rs`): enums and structs
are visible file-wide; list bindings only when lexically in scope before the
cursor — at end-of-buffer that means all top-level bindings emitted so far.

**Without tree-sitter** (`--no-default-features` or `tree-sitter` off), the
driver runs on `NullContextProvider`:

- `#repeat` / `--repeat` collections still work fully — they are engine-level
  (`ExpansionContext.collections`), no provider involved. The flagship sketch
  renders identically because its header supplies `Msg`.
- Buffer-derived collections are simply absent, so a data-driven repeat whose
  name resolves nowhere degrades per the existing no-data rule: **one copy
  with the placeholder left in** (verified: `arm*@Msg` with no data →
  `pattern => ,`, exit 0). No new failure mode.
- Scope-aware variants always see `Root`; `base_indent` still comes from the
  abbreviation line's leading whitespace.

This is stated in the docs as: *file mode never requires tree-sitter;
tree-sitter makes the `#repeat` lines optional.*

### Q3 — output discipline and failure atomicity

`codemmet >> main.rs` means a mid-file failure must not leave half of the
render appended to a real source file. **Recommendation: render the entire
file to memory and write stdout only on total success.** The buffer already
exists — progressive context *requires* accumulating all output in memory —
so atomicity is free: on any error, nothing has been written and the shell
redirection appends zero bytes. Streaming line-by-line would gain nothing
(output is small) and would turn every abbreviation typo into a corrupted
target file.

**Fail-fast vs collect-all — options:**

- (a) Collect all errors, report at the end. Rejected for v1: progressive
  context makes later lines *depend* on earlier output. After line 7 fails,
  line 12's buffer is wrong, so its "error" (or silent misexpansion) is
  cascade noise, not signal.
- (b) **Fail fast at the first error. Recommended.** Report it with the
  source name and 1-based `.codemmet` line number on stderr, write nothing
  to stdout, exit non-zero. A `--keep-going` diagnostic mode can come later
  if dry-run linting of `.codemmet` files becomes a workflow.

**Diagnostics format:** `error: main.rs.codemmet:7: cannot expand
\`match:self>arm*\`: unknown trigger \`arm\`` — source is the positional
file path, or `<stdin>` when piped.

**Exit codes** reuse the existing convention exactly
(`crates/codemmet/src/cli/mod.rs`): 0 success; 1 parse error — now covering
both abbreviation parse errors *and* file-format errors (malformed/unknown
directive, misplaced `#lang`); 2 unknown trigger / render failure; 3 config
or environment (unknown language, unreadable file, invalid UTF-8, no
language resolvable). The first failing line determines the code.

### Q4 — tabstops

**Decision (v1, superseded — see the revision below): file mode always
strips; `--tabstops keep` is rejected (exit 3).** Two independent reasons:

- The concatenation of per-line snippets is not one valid snippet: every line
  restarts numbering at `$1` and may carry its own `$0`, so "keep" output
  would need whole-file renumbering and `$0` unification — real design work
  for a consumer that does not exist yet.
- Progressive context parses the accumulated output with tree-sitter; the
  buffer must be real code. Keeping markers in the emitted text while
  stripping them in the buffer means dual rendering per line — again,
  possible, but not for free.

`$0` in strip mode already just vanishes (the engine reports the cursor
position separately, which file mode discards — there is no meaningful single
cursor for a whole file). One cosmetic consequence to fix in v1: a stripped
body can leave a whitespace-only line where `$0` sat alone (verified:
`impl:Msg>fn:handle` emits an 8-space blank body line). File mode should
right-trim each emitted line — the output is committed code, and
`.editorconfig`-clean output matters for `>>` appends. If a keep-mode
consumer materializes (an editor plugin ingesting whole files as snippets),
revisit with a renumbering pass; the flag surface is reserved for it.

#### Q4 revision — `--tabstops keep` is now supported (owner-approved)

The keep-mode consumer materialized: the Neovim plugin's `<A-E>` buffer
render re-enters `vim.snippet` navigation by expanding the whole render as
one snippet. Both v1 objections are resolved by implemented machinery:

- **Whole-file renumbering** lives in `codemmet-core::snippet::merge`
  (`SnippetDocBuilder`). It applies the engine's own single-snippet
  `renumber` rules across the concatenated per-line expansions, treating
  each line exactly like a sibling group inside one abbreviation (`if+el`):
  - Tabstop identities are **line-local**; fresh global numbers are assigned
    by first occurrence in document order, so mirrors keep their linkage
    within their line and lines never alias each other's stops.
  - Every `$0` candidate except the **last in document order** demotes to a
    plain numbered stop at its position. This follows the engine's sibling
    precedent (`if+el` → `if $1 {…$2…} else {…$0…}`; `arm*3` → `$2`, `$4`,
    `$0`) rather than the vanish rule, which applies only to a parent whose
    children take over its cursor. Demotion is also the better UX: each
    construct keeps a jump position (`use std::fmt;$1`, `}$2`), so
    tab-tab-tab walks the whole render. Since the engine appends a `$0` to
    any snippet lacking one, every line has a candidate and the survivor is
    always the last abbreviation line's.
  - Escaping survives concatenation byte-for-byte: only `$n` / `${n}` /
    `${n:placeholder}` / `$0` tokens are rewritten; `\$`, `\\`, `\}` pass
    through verbatim, and an unrecognizable `$` form stays literal text.
  - Keep-mode lines are **not** right-trimmed (unlike strip mode): a `$0`
    alone on an indented line is a real cursor position, not trailing
    whitespace.
- **Dual rendering per line** is exactly what the driver does: each
  abbreviation expands twice from the same context — stripped for the
  progressive-context buffer (which must stay real, parseable code) and
  kept for the output document. Expansion is deterministic, so the two
  bodies always agree; the cost (one extra expansion per line) is noise.

`strip` remains the default; `keep` is opt-in via `--tabstops keep` on
`render`. Failure atomicity is unchanged — a mid-file error still emits
zero bytes in either mode.

### Q5 — CLI wiring

**Bare invocation.** In `run()` before clap parsing: if there are no CLI
arguments at all **and** stdin is not a terminal (`std::io::IsTerminal`, the
`train` crate already uses it), run file mode on stdin. Interactive bare
`codemmet` is untouched: clap prints help to stderr and exits 2, exactly as
today. Any flag or argument present falls through to normal clap parsing —
the sugar is only for the pure-pipe spelling.

**Subcommand alias — options:** `build`, `render`, `file`, `gen`.
**Recommendation: `render`.** It names precisely what happens — a pure
input→output transformation with no artifact bookkeeping — and matches the
codebase's own vocabulary (`snippet/render.rs`, `RenderError`, "render" is
used throughout the docs). `build` over-promises (dependency graphs, outputs
on disk, incremental rebuilds — cargo connotations this filter will never
honor); `gen` is generic; `expand` is taken and semantically singular.

```
codemmet render main.rs.codemmet >> main.rs
codemmet render < main.rs.codemmet          # stdin, needs #lang or --lang
codemmet render --lang go - < handlers.txt
```

**Positional file arg.** Two spellings, one rule:

- `codemmet render [FILE]` — `FILE` optional, `-`/omitted reads stdin (the
  `expand` precedent).
- `codemmet FILE.codemmet` — top-level positional via clap's
  `args_conflicts_with_subcommands`, **accepted only when the argument ends
  in `.codemmet`** and otherwise rejected with a hint to use `render`. The
  extension gate keeps a typo'd subcommand (`codemmet expnad`) an error
  instead of a file lookup, and sidesteps shadowing (a file literally named
  `list` is still reachable via `codemmet render list`).

The positional spelling enables filename inference (decision: directive >
`--lang` > inner extension).

**Flags carried over:**

- `--config`, `--paths`, `--no-project` — already global; the registry tiers
  apply unchanged, so project packs work in file mode.
- `--lang` — the middle rung of the resolution ladder.
- `--indent` — same meaning; resolved once per file via the existing
  `resolve_indent_unit`.
- `--repeat` — CLI-level seed collections, below `#repeat` in precedence
  (Q1); useful for parameterizing one `.codemmet` template from a script:
  `codemmet render tmpl.rs.codemmet --repeat Msg=A,B`.
- `--tabstops` — `strip` is the default; `keep` emits the whole file as one
  renumbered snippet document (Q4 revision; originally rejected in v1).
- **Not carried:** `--json` (no meaningful single cursor; a JSON stream is
  out of scope), `--base-indent` (subsumed by per-line leading indentation,
  Q6).

### Q6 — format details

Most are fixed in the Format spec above. The one real fork:

**Leading indentation on an abbreviation line — options:**

- (a) Error in v1 ("reserved for future semantics"). Conservative; keeps both
  future interpretations (textual indent vs structural nesting) open, and
  immediately corrects a user who indents expecting nesting.
- (b) **Significant as that line's `base_indent` — textual only.
  Recommended.** The engine already threads `base_indent` through every
  continuation line (it is the `--base-indent` flag's machinery, zero new
  rendering code), and it is genuinely useful: a `.codemmet` fragment can
  render method bodies pre-indented for pasting into an existing `impl` or
  class. The docs must say in one line: *indentation indents the text; it
  does not nest it into the previous block* (nesting is out of scope, Q2).
  The risk of that misreading is real but cheap to correct — the misindented
  output makes the misunderstanding visible immediately, and option (a)
  forecloses a useful feature to prevent a recoverable surprise.

Tabs and spaces are both accepted and passed through verbatim as the base
indent (matching `--base-indent`, which accepts any prefix string).

### Q7 — integration points (noted, not built)

- **Import-requires (in-flight PR).** Once `Expansion.requires` reports the
  imports a rendered snippet needs, file mode is its best consumer: collect
  `requires` across all lines, dedupe, and prepend the corresponding `use:`/
  `import` renders once at the top of the output — something the single-shot
  `expand` CLI can never do well. Design hook: the driver already owns the
  whole buffer, so a final prepend pass is trivial. Not in v1; when the PR
  lands, add a `#requires off` escape hatch and goldens.
- **Transforms CLI (feat/case-transforms).** Transforms are per-abbreviation
  syntax inside the engine, so they work in file mode automatically the day
  they land — no file-mode work. If a transforms-related flag appears, the
  Q1 namespace rule gives it a directive for free.
- **`codemmet scaffold` (multi-file, future).** File mode is the natural
  substrate: a scaffold is a manifest mapping output paths to `.codemmet`
  sources (or inline blocks), and `scaffold` becomes a loop over `render`
  with a shared seed-collection environment. Designing file mode as a pure
  function `(text, lang, seeds) → code` (which the core `filefmt` split
  ensures) keeps that composition mechanical.

### Q8 — testing

- **Golden fixtures via the CLI**, following `crates/codemmet/tests/cli.rs`
  house style: a `tests/file_mode/` fixture dir of `NAME.codemmet` →
  `NAME.expected.EXT` pairs, run through the binary with `assert_eq!` on
  exact stdout. Cover: the flagship sketch, blank-line preservation, comment
  dropping, `#repeat` shadowing and from-line-down scope, leading-indent
  base-indent, CRLF input, multi-collection files.
- **Error-path goldens:** unknown directive, misplaced/duplicate `#lang`,
  abbreviation parse error, unknown trigger — each asserting exit code
  (1/1/1/2), the `FILE:LINE:` prefix on stderr, and **empty stdout**
  (the atomicity contract is a tested invariant, not prose).
- **Progressive-context fixtures:** `enum:Msg>variant*[Quit,Move]` on line 1,
  `match:m>arm*@Msg` later, *no* `#repeat` — arms filled from the emitted
  enum. Plus a struct→field case and a shadowing case (`#repeat Msg=X`
  beating the buffer enum, per Q1 precedence).
- **Degradation tests:** the same progressive fixture under
  `--no-default-features --features train`-less build asserting the
  single-copy placeholder fallback and exit 0; a `#repeat`-driven fixture
  asserting identical output with and without tree-sitter. CI grows a
  `cargo test -p codemmet --no-default-features` lane if it lacks one.
- **Fuzz:** a `fuzz/fuzz_targets/filefmt.rs` target over the pure
  `codemmet-core::filefmt` parser (the new untrusted-input surface),
  mirroring the existing `parse.rs` target: arbitrary bytes never panic.
  A second-stage target can drive the full render loop with a small
  language-pinned corpus.
- **CLI-wiring tests:** bare-pipe detection (piped stdin, no args →
  rendered output), interactive-bare unchanged (help, exit 2), positional
  `.codemmet` gate, `render -` stdin spelling, language-ladder precedence
  (all three rungs and the exit-3 miss).

## Verification

- `just ci` (fmt-check + clippy + full test suite) green, including the new
  golden and error-path suites.
- The flagship pipeline end-to-end:
  `cat demo.rs.codemmet | codemmet >> /tmp/out.rs` produces compiling-shaped
  Rust and appends nothing on an injected error (checked by byte count).
- The progressive flagship without its `#repeat` line renders identical
  output on a default build; the minimal build renders the placeholder
  fallback and exits 0.
- `cargo fuzz run filefmt` clean for a smoke budget.

## Milestones

Sliced so a usable filter ships before progressive context — which works
because `#repeat` already carries the flagship: the owner's sketch renders
byte-identically in FM0.

- **FM0 — format & filter (no tree-sitter dependency).** The
  `codemmet-core::filefmt` parser (directives, comments, blanks, indent
  capture); `#lang`/`#repeat` with the Q1 rules; the language-resolution
  ladder incl. filename inference; the render loop on `NullContextProvider`
  with seed collections; atomic buffered output; `FILE:LINE:` diagnostics
  and exit codes; `render` subcommand, top-level `.codemmet` positional,
  bare-pipe detection. Golden + error-path + wiring tests. **First
  shippable, demoable slice — the flagship sketch works.**
- **FM1 — progressive context.** Swap in `TreeSitterProvider` on default
  builds (the LSP's cfg split); end-of-buffer `context_at` per line;
  collection-precedence overlay; progressive fixtures (enum→match,
  struct→fields, shadowing); degradation tests pinning FM0 behavior on
  minimal builds. The `#repeat` header becomes optional.
- **FM2 — polish.** `docs/cli.md` section + a `docs/examples` entry; the
  `filefmt` fuzz target; whitespace-only-line trimming audit; CHANGELOG;
  a `PLAN.md` note; evaluate `filefmt` reuse in the wasm playground.

## Open questions for the owner

- **Leading indentation** (Q6): base-indent semantics (recommended) or
  reserved-error in v1? This is the one format decision that is hard to
  reverse once files exist.
- **Subcommand name** (Q5): `render` (recommended) vs `build` — pure
  naming, but it lands in scripts immediately.
- ~~**`--tabstops keep` rejection** (Q4): fine to hard-error in v1, or is
  there a near-term editor consumer that needs keep-mode whole-file
  snippets now (which would pull renumbering work into FM0/FM1)?~~
  **Resolved:** the Neovim plugin's snippet re-entry is that consumer;
  keep mode is now supported (see the Q4 revision).
- **Comment policy** (Q1): confirm `#`+space comment / unknown-`#word`
  hard error split.
- **Whitespace-only body lines** (Q4): confirm right-trimming emitted
  lines is desired (it changes `expand` parity for a few snippets whose
  `$0` sits alone on an indented line).
