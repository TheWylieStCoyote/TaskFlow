# Snippet variables (context-resolved values) — design

Today a snippet body has two kinds of dynamic content: parameters the caller
supplies (`${name}`) and reserved context markers the engine resolves (`${i}`
for the repeat index, `${value}`/`${outer}` for data-driven items). This adds
**variables** — the same mechanism extended to editor and environment context:
the document's filename, the current year, and user/project-defined values like
an author or license name. The flagship, a file header:

```toml
[snippets.hdr]
description = "file header comment"
body = """
// ${TM_FILENAME} — $1
// Copyright ${CURRENT_YEAR} ${author}. Licensed under ${license}.
$0
"""
```

expanded in `src/render.rs` in 2026, with `author`/`license` set once in
config:

```rust
// render.rs — 
// Copyright 2026 Wylie Standage-Beier. Licensed under MIT.
```

Deterministic boilerplate — headers, module preambles, test scaffolds that
name their file — written once, correct in every file, in every language.

## The fork: who resolves a variable

**Approach A: pass LSP/TextMate variable syntax through to the editor.** Emit
`${TM_FILENAME}`/`${CURRENT_YEAR}` verbatim in keep-mode output and let the
client's snippet engine resolve it. VS Code resolves the full TextMate set —
but Neovim's `vim.snippet` resolves only a small subset, and unknown variables
come out empty or literal, so this is unreliable exactly where codemmet's own
plugin lives. It also does nothing for strip mode, the CLI, the playground, or
the trainer, and it cannot resolve user-defined variables at all (no editor
knows `${author}`).

**Approach B: codemmet resolves them, via the context seam.** The host fills a
`variables` map on `ExpansionContext` — exactly how `collections` already
works — and the renderer substitutes recognized `${VAR}` references as literal
text. The pure core cannot read the filesystem or clock itself (the same
reason it cannot in the WASM build), so the host supplies values: the LSP
server derives the document family from the URI and the time family from the
clock, the CLI takes `--var` and derives the document family from a new
`--from` flag, and user/project values load from config.

**Recommendation: Approach B, exclusively.** Deterministic, identical across
every editor, works in strip mode and headless, and testable — the golden
suite can assert a header byte-for-byte with an injected clock. No variable
syntax ever reaches the editor's snippet engine; by the time a body leaves the
server it contains only tabstops and literal text, sidestepping `vim.snippet`'s
spotty variable support entirely.

## Resolution model

`${X}` in a template body resolves in this order:

1. **Declared parameter.** `X` appears in the snippet's `params` list — the
   existing behavior (bound argument as text, otherwise a defaulted tabstop),
   entirely unchanged. A declared param always shadows a variable of the same
   name, so today's packs keep today's meaning.
2. **Known variable.** `X` is a builtin variable name (the reserved set below)
   or a user/project-defined variable. Renders as literal text from
   `ExpansionContext.variables` in both keep and strip modes — a variable is
   a value, never a tabstop.
3. **Neither** — the existing "undeclared parameter" error.

The reserved builtin names are uppercase (TextMate convention) and
user-defined names are ordinary lowercase words, so the two namespaces read
differently at a glance, and neither can collide with the existing reserved
template names (`i`, `value`, `outer`, `child`), which keep their current
meanings and win their spellings outright.

### The reserved builtin set

Document family, derived from the document's path (URI in the LSP, `--from`
on the CLI):

| Name | Example (`/repo/src/render.rs`) |
| --- | --- |
| `TM_FILENAME` | `render.rs` |
| `TM_FILENAME_BASE` | `render` |
| `TM_DIRECTORY` | `/repo/src` |
| `TM_FILEPATH` | `/repo/src/render.rs` |
| `RELATIVE_FILEPATH` | `src/render.rs` (needs a workspace root) |
| `WORKSPACE_NAME` | `repo` |

Time family, derived from the host clock: `CURRENT_YEAR`, `CURRENT_MONTH`,
`CURRENT_DAY`, `CURRENT_DATE` (ISO `YYYY-MM-DD`).

The whole set is *reserved* (validated by `codemmet check`) from day one, but
populated in stages — the MVP fills the filename family and `CURRENT_YEAR`
(see milestones). TextMate spellings are recommended over codemmet-native ones
(`${FILENAME}`) because pack authors porting VS Code snippets recognize them
instantly and there is no competing house convention to protect; an owner call
at review (open question).

### Unpopulated builtins

A body can reference `${TM_FILENAME}` in a context that has no file — an
untitled buffer, a bare `codemmet expand` without `--from`, the playground.
Three options:

- **(a) Error.** Honest but hostile: the snippet becomes unusable headless.
- **(b) Empty string.** Silent data loss; a header with a blank year looks
  like a bug.
- **(c) Fall back to a tabstop with the variable name as placeholder**
  (`${3:TM_FILENAME}`), so the user lands on it and types the value.

**Recommendation: (c)** — it is exactly the missing-data fallback `${value}`
already uses, so the engine stays consistent: known-but-absent context
degrades to a scaffold, only *unknown names* are errors. An unpopulated
**user** variable behaves the same way (`${author}` with no config → a
tabstop placeholder `author`).

## The `[variables]` config table

Where user/project values live. Options:

- **(a) Inside snippet packs** (`[variables]` in `<lang>.toml`). Rejected:
  `author` and `license` are language-independent, and packs are per-language
  (`SnippetFile` is `deny_unknown_fields` today — packs stay snippet-only).
- **(b) A dedicated file per tier**: `<config>/codemmet/variables.toml` (user)
  and `.codemmet/variables.toml` (project), each holding one `[variables]`
  table. Merged user < project, same direction as snippet packs; project
  variables follow the existing `project_snippets`-style trust gate in the
  editor (a project can set `${author}`, so untrusted repos stay opt-in —
  reuse the same switch rather than adding a second one).

**Recommendation: (b).**

```toml
# ~/.config/codemmet/variables.toml
[variables]
author = "Wylie Standage-Beier"
license = "MIT"
```

Validation rules, enforced when the file loads and by `codemmet check`:
names must be words (`[A-Za-z_][A-Za-z0-9_]*`), must not shadow the reserved
template names (`i`, `value`, `outer`, `child`), and must not redefine a
builtin variable name — the builtins are host-derived facts, and silently
pinning `CURRENT_YEAR = "2019"` in a project file is a footgun, so it is
rejected exactly like a param named `i` is today (open question below).
Values are plain strings; no interpolation inside values (a variable cannot
reference another variable).

## What is explicitly out of scope

- **Editor-state variables** (`TM_SELECTED_TEXT`, `CLIPBOARD`,
  `TM_LINE_NUMBER`). Deliberately deferred, not rejected: the mechanism
  supports them (the Neovim plugin would pass them in command arguments, as it
  already passes `formatting_options`), but the MVP ships without them. Wrap
  already covers the selection use case.
- **Approach A passthrough.** No mode that emits unresolved variable syntax
  for the client; one resolution story.
- **Shell/command variables.** Never. A snippet pack must stay data, not code
  — the existing security stance (a project pack "cannot run code, but it can
  mislead") depends on it.
- **Variable transforms** (`${TM_FILENAME/\..+$//}` regex syntax). TextMate
  supports it; codemmet does not. The `TM_FILENAME_BASE`-style precomputed
  variants cover the real cases without embedding a regex engine in bodies.
- **Cross-file/workspace facts** beyond the document family (git branch,
  crate name from `Cargo.toml`). Plausible growth; not designed for now.

## Architecture

The seam split mirrors `collections` exactly: the core resolves, hosts supply.

- **`ExpansionContext`** (`codemmet-core/src/context.rs`) gains
  `variables: BTreeMap<String, String>` next to `collections`; empty
  everywhere by default.
- **Renderer** (`codemmet-core/src/snippet/render.rs`): `Span::Param { name }`
  resolution gains step 2 — after the declared-param path misses, look the
  name up in `ctx.variables` and emit literal text; unknown names keep the
  current error, and known-but-absent builtins take the tabstop fallback. No
  new span kind: a variable reference *is* an undeclared `${name}`, and only
  render/validation time can classify it (user variable names are not known
  when a template parses).
- **Loader/validation** (`codemmet-core/src/config/loader.rs`): the
  compile-time "body references undeclared parameter" check loosens to accept
  names in the known-variable set. `SnippetSet::compile` takes the merged
  variable names (builtin set + loaded `[variables]` keys) alongside its
  sources; `codemmet check` thereby validates the namespace, and its error
  message for a genuinely unknown name grows a hint ("not a parameter,
  builtin variable, or `[variables]` entry"). Composite fragments get the
  same treatment: `${name}` markers in `expands`/options that name a known
  variable substitute the variable's value at render time (before re-parse),
  and validation substitutes a safe dummy exactly as it does for params.
- **Config** (`codemmet-core/src/config/`): a small `variables.rs` module —
  parse one `[variables]` file, merge tiers, validate names. Loaded by the
  CLI (`Sources`), the LSP server (init options gain `variables_file`
  overriding the default path, mirroring `paths_file`), and `check`.
- **LSP server** (`crates/codemmet/src/lsp/`): when building the context for
  `expand`/`wrap`/`preview`/completion (and `codemmet.list` bodies), populate
  `ctx.variables` — document family from `file_uri_to_path(uri)`,
  `WORKSPACE_NAME`/`RELATIVE_FILEPATH` from the initialize `rootUri`, time
  family from the clock, plus the user names from the config tiers. The two
  never overlap: builtins are host-derived facts the config cannot redefine
  (see validation). The Neovim plugin needs **zero changes**: resolution is
  server-side.
- **Clock.** `std` has no calendar; options are a date crate (`jiff`/`time`)
  or a ~15-line UTC civil-date conversion from `SystemTime`. **Recommendation:
  the small internal helper**, UTC, behind an injectable timestamp so tests
  are deterministic — a dependency is not worth four fields, and cargo-deny
  stays quiet. Local-vs-UTC only matters within hours of New Year (open
  question; UTC is the defensible default).
- **WASM/trainer.** Nothing to do: `variables` defaults empty, builtins fall
  back to tabstops, and the trainer's graded exercises simply avoid
  variable-using triggers. The playground can grow a variables input box
  beside the collections box as polish.

## CLI hatch

Every editor feature keeps a headless twin:

```
codemmet expand --lang rust hdr --var author="Wylie S" --var license=MIT \
                --from src/render.rs
```

- `--var NAME=value` (repeatable) sets user variables for one expansion,
  layered over the loaded `[variables]` tiers (explicit beats config). Shared
  by `expand` and `wrap`.
- `--from PATH` derives the document family from a path without reading the
  file — `TM_FILENAME`, `TM_FILENAME_BASE`, `TM_DIRECTORY`, `TM_FILEPATH`
  (and `RELATIVE_FILEPATH`/`WORKSPACE_NAME` when a project root is findable
  above it). The path does not need to exist, so golden tests are hermetic.
- Time family fills from the real clock; a hidden `--now TIMESTAMP` (or env
  var) pins it for golden tests — **recommendation: an undocumented
  `CODEMMET_NOW` env var**, so user-facing flags stay clean while the test
  suite stays deterministic.
- `codemmet check` picks up the `[variables]` tiers it can see and validates
  every pack reference against params + builtins + defined variables, and
  validates the `variables.toml` files themselves (bad names, redefined
  builtins).

## Verification

- **Golden CLI tests** — the core suite: `hdr`-style fixtures through
  `codemmet expand --var … --from … ` with `CODEMMET_NOW` pinned, asserting
  exact output in keep and strip modes; precedence cases (param shadows
  variable, `--var` beats config, project beats user); fallback cases (no
  `--from` → filename tabstop).
- **Unit tests** in `codemmet-core`: renderer precedence and fallback, loader
  namespace validation (unknown name still errors, builtin reference passes,
  `[variables]` key passes), `variables.toml` validation errors, the UTC date
  helper against known timestamps (epoch edges, leap years).
- **LSP integration tests** (`crates/codemmet/tests/lsp.rs`): open a
  `file:///…/render.rs` document with a variable-using override pack, expand,
  assert `TM_FILENAME` resolved in the returned snippet and that no `${…}`
  variable syntax survives to the client; an untitled/non-file URI takes the
  tabstop fallback.
- **Fuzz**: bodies with arbitrary `${…}` names and adversarial
  `variables` maps never panic; resolved output contains no unresolved
  markers for known names.

## Milestones

- **V0 — core resolution + CLI.** `ExpansionContext.variables`, renderer
  precedence + tabstop fallback, loader accepts the builtin reserved set,
  `--var`/`--from`/`CODEMMET_NOW`, the date helper, golden tests. **The first
  usable, demoable slice** — headers work headless.
- **V1 — `[variables]` config tiers.** `variables.toml` user + project
  loading and validation, `codemmet check` coverage, `--var` layering. The
  killer `${author}`/`${license}` use case lands here.
- **V2 — LSP population.** Document/workspace family from the URI and root,
  time family from the clock, `variables_file` init option, integration
  tests. Neovim (and every other client) gets headers for free, no plugin
  changes.
- **V3 — docs + polish.** A variables section in `docs/snippets.md` + a
  cookbook header example, builtin `hdr` snippets per language, the
  playground variables box, PLAN.md note. Editor-state variables
  (selection/clipboard/line) remain listed growth, unblocked by the
  mechanism.

## Risks and open questions

- **Builtin name spelling.** TextMate (`TM_FILENAME`, `CURRENT_YEAR`) vs
  native (`FILENAME`, `YEAR`). Leaning TextMate for portability and
  recognition; the uppercase convention is the real invariant. **Owner call
  at review.**
- **Redefining builtins in `[variables]`.** Rejected above (footgun), but
  there is a legitimate reading — pinning `CURRENT_YEAR` in a fixture-heavy
  repo. `CODEMMET_NOW` covers the test case; revisit only if demand shows up.
- **Unpopulated-builtin fallback.** Tabstop-with-name is recommended; if
  dogfooding shows headers silently gaining `TM_FILENAME` placeholders where
  users expected an error, tighten to a render error behind the same
  decision the `${value}` fallback gets.
- **UTC vs local time.** UTC recommended (deterministic, no tz database);
  a `CURRENT_YEAR` off by one for a few hours at New Year in western
  timezones is the accepted cost. Revisit if a date-heavy variable family
  (full timestamps) ever lands.
- **Loader plumbing.** `SnippetSet::compile` gaining a known-variable-names
  argument touches every call site (registry, tests, fixtures); mechanical
  but wide. An alternative — validating variable references in a separate
  post-compile pass like `validate_composites` — keeps signatures stable and
  is worth choosing at implementation time.
- **Completion/browser interplay.** Variable-using snippets render their
  resolved values in completion docs, ghost previews, and the new
  `codemmet.list` hints automatically (all go through the engine) — verify a
  stale-clock cache cannot arise (engines are cached per directory list;
  variables live in the per-request context, so they cannot go stale — assert
  this in a test).

Per the owner's notes (`variables.md`): the MVP is user/project-defined
variables plus the document filename and current year, resolved by codemmet
(Approach B), never passed through to the editor. This plan goes back for
owner review before any implementation.
