# codemmet — Emmet for Programming Languages

## Context

Emmet lets web developers expand terse abbreviations (`div>ul>li*3`) into HTML/CSS. **codemmet** brings the same idea to general programming languages: type `fn:main>fori10>pr` in a Rust buffer and get a `main` function containing a for-loop containing a `println!`, with tabstops to jump through.

Design decisions:

- **Syntax:** Hybrid — terse keyword triggers with glued/explicit args (`fori10`, `fn:main`) plus Emmet composition operators `>` (child), `+` (sibling), `*` (repeat), `()` (group), `^` (climb-up, deferred).
- **Tree-sitter:** phased — ship pure text expansion first, then add context-aware expansion (`fn` in `impl` → `&self`; `def` in class → `self`), wrap-with-abbreviation, and tree-based indentation.
- **Integration:** LSP server first-class (works in Neovim, VS Code, Helix, Zed…), plus a CLI, plus a Neovim Lua plugin **in the same repo**.
- **Languages:** Rust, Python, C, C++, JavaScript, TypeScript, Go. Rust + Python first (covers brace and indentation paradigms).
- **Snippets:** built-in packs compiled into the binary + user overrides from `~/.config/codemmet/snippets/<lang>.toml`.
- **Tabstops:** LSP snippet syntax (`${1:placeholder}`, `$0`); CLI can strip them.
- **Quality bar:** strict clippy (pedantic/nursery) + rustfmt, GitHub Actions CI, cargo-deny, lefthook pre-commit/pre-push hooks, stylua for Lua.

## Workspace Layout

Convert to a cargo workspace; one installed binary `codemmet` with subcommands (`expand`, `wrap`, `list`, `check`, `lsp`) like rust-analyzer/emmet-language-server.

```
codemmet/
├── Cargo.toml                  # workspace root: workspace.package/dependencies/lints
├── PLAN.md                     # this plan, committed to the repo
├── crates/
│   ├── codemmet-core/          # lib: abbr parser, snippet IR, renderer, TOML loader,
│   │   │                       #   language profiles, ContextProvider trait
│   │   ├── src/{lib.rs, abbr/, snippet/, lang/, config/, context.rs}
│   │   └── snippets/           # builtin packs: rust.toml, python.toml, c.toml, cpp.toml,
│   │                           #   javascript.toml, typescript.toml, go.toml (include_str!)
│   ├── codemmet-tsctx/         # lib (M3+): tree-sitter ContextProvider; per-lang features
│   └── codemmet/               # bin: clap CLI + lsp-server main loop
│       └── src/{main.rs, cli/, lsp/}
├── lua/codemmet/{init.lua, lsp.lua, wrap.lua}   # Neovim plugin
├── plugin/codemmet.lua
├── .github/workflows/ci.yml
├── deny.toml  rustfmt.toml  stylua.toml  lefthook.yml  justfile  README.md
```

Dependency rules: **codemmet-core has no tree-sitter, no tokio, no clap** (only serde, toml, thiserror, dirs). codemmet-tsctx depends on core, never the reverse. The bin depends on tsctx behind a default-on `tree-sitter` cargo feature so a minimal build exists.

Key deps: `clap 4.5`, `lsp-server 0.7` + `lsp-types 0.97` (rust-analyzer stack — synchronous, no tokio; `tower-lsp` is unmaintained and overkill for pure CPU work; `Connection::memory()` enables in-process integration tests), `serde 1`, `toml 0.9`, `thiserror 2`, `anyhow 1`, `insta 1` (dev), `tree-sitter 0.26` + `tree-sitter-<lang>` grammar crates (features per language).

## Abbreviation Mini-Grammar

Tokens: `WORD` (`[A-Za-z_][A-Za-z0-9_]*`), `NUMBER`, operators `> + * : , ( ) ^`, quoted args (`"…"`/`'…'`), raw args (backslash escapes operators). Whitespace outside quotes terminates the abbreviation (used to extract it left of the cursor).

```
abbr     := group
group    := chain ( '+' chain )*         ; siblings
chain    := element ( '>' group )?       ; '>' attaches following whole group as children
element  := atom ( '*' NUMBER )?
atom     := node | '(' group ')'
node     := WORD args?
args     := ':' arg ( ',' arg )*         ; positional binding to declared params
```

Precedence tightest-first: args → `*` → `>` → `+`. This reproduces Emmet semantics (`a>b+c`: b,c children of a; `a+b>c`: c child of b). `^` reserved from day one, implemented in M4 as a parser stack-pop like Emmet does.

**Trigger resolution / glued args:** lexer yields WORD; the resolver finds the longest registered trigger that is a prefix (`fori10` → trigger `fori`, remainder `10`). A numeric remainder binds to the first `kind = "int"` param, identifier remainder to first `kind = "ident"` param. Explicit `:`-args bind positionally. Unknown trigger → error with edit-distance "did you mean" suggestion.

Parser: hand-written recursive descent (~200 lines) producing `AbbrNode { trigger, args, repeat, children }`, with byte-offset error spans.

## Core Data Model (AST → IR → text)

**Key design decision:** template bodies parse into lines with _relative indent depth_ (`IrLine { depth, spans }`); braces are just text and the renderer owns indentation — one pipeline handles both brace and indentation languages.

Spans: `Text`, `Tabstop {index, placeholder}`, `Mirror`, `ChildSlot` (`${child}`), `FinalCursor` (`$0`).

Rendering `expand(abbr, lang, ctx)`:

1. Parse abbreviation → resolve each trigger to a snippet definition (context variant selection is trivial until M3).
2. Args supplied → plain text; unsupplied params → tabstop with default as placeholder.
3. Compose depth-first: children render at `ChildSlot` with +1 depth per nesting level; a parent with children drops its own `$0`; a leaf's `ChildSlot` degrades to a cursor position; siblings concatenate; `*n` renders n sibling copies with independent tabstops.
4. Global tabstop renumbering in document order; exactly one `$0` survives (innermost/last).
5. Materialize with `indent_unit` (from editor context, `--indent`, or language default) and `base_indent`. Escape `$`/`}`/`\` per LSP snippet grammar; `--tabstops strip` renders from the IR (no regex over snippet syntax).

Language profile: `{ id, lsp_language_ids, extensions, default_indent, snippet_sources }`. Registry parses builtin TOMLs once, merges user overrides per-trigger (user wins; `disabled = true` removes a builtin). TypeScript inherits JavaScript; C++ inherits C.

## TOML Snippet Schema

Body syntax = LSP snippet syntax **plus** `${child}` (composition target, at most one) and `${param_name}` (named params). Literal-string bodies with tab-indentation (loader also accepts 4-space groups; rejects mixed). `serde(deny_unknown_fields)`; `codemmet check` validates user files.

```toml
[meta]
language = "rust"

[snippets.fori]
description = "for loop over integer range"
params = [
  { name = "var", kind = "ident", default = "i" },
  { name = "end", kind = "int",   default = "10" },
]
body = '''
for ${var} in 0..${end} {
	${child}
}
'''

# M3: context-conditional variants (first match wins; top-level body is fallback)
[[snippets.fn.variants]]
context = "type-body"        # normalized scope, or raw node kind as "ts:impl_item"
body = '''
fn ${name}(&self$1) {
	${child}
}
'''
```

`kind` (`ident|int|str`, default `str`) drives glued-arg binding. Repeated `${name}` → tabstop + mirrors. MVP packs: ~25–40 triggers per language (fn/def, fori/fore, if/el/elif, while, match/switch, struct/class, impl, let, ret, pr, try, test, main, imports).

## LSP Server

Single-threaded `for msg in connection.receiver` loop over `lsp-server`. State: `Engine`, doc store (Full sync in M2 → Incremental in M3 for tree-sitter `InputEdit`s), `Box<dyn ContextProvider>`, config from `initializationOptions`.

Capabilities: `textDocumentSync: Incremental` (M2 shipped Full; M3 moved to Incremental), `completionProvider { resolveProvider: false, triggerCharacters: [">", "+", ":", ","] }`, `executeCommandProvider { commands: ["codemmet.expand", "codemmet.wrap"] }`. **Shipped note:** `resolveProvider` is `false` — the expansion is already computed cheaply, so documentation is attached to each item eagerly rather than via a `completionItem/resolve` round-trip.

**Completion flow:** scan left from cursor over the abbreviation charset (quote-aware — a quote only opens a quoted region after `:`/`,`, so host-language apostrophes like Rust lifetimes don't swallow the line; stop at whitespace, ~200-char cap). If the candidate parses and all triggers resolve, offer ONE snippet item: `insertTextFormat: Snippet`, **server-computed `textEdit`** replacing the abbreviation range (never trust client word boundaries — the emmet-ls lesson), `filterText` = abbreviation, documentation attached eagerly. **Shipped note:** the snippet is rendered with `base_indent` zeroed — LSP snippet clients re-indent continuation lines from the insertion position, so baking the base indent in would double it. Also offer trigger-prefix discovery items mid-composition. Operator gating (`completion = "operators-only"`) uses the canonical operator set exported from `codemmet-core::abbr` (`OPERATOR_CHARS`), so climb-up (`^`) and grouping count.

**Commands** (because `workspace/applyEdit` can't carry tabstops): `codemmet.expand {uri, position}` and `codemmet.wrap {uri, range, abbreviation}` **return** `{ range, snippet, isSnippet }`; the client deletes the range and calls its snippet expander. **Shipped note:** wrap is not string-spliced — the core exposes `render_with_selection` / `Engine::expand_wrap`, which dedents the selection and splices it into the composed IR at the final-cursor position, so escaping, mirrors, indentation, and tabstop renumbering all flow through the normal pipeline (this replaced an earlier string-search-for-`$0` approach that diverged on mirrors). Tree-sitter snaps the selection to statement boundaries first; the returned range starts after the first line's leading whitespace so the construct keeps its indentation. Editor passes `shiftwidth`/`expandtab` in command args.

## Tree-sitter Seam (defined in M1, never reworked)

```rust
// codemmet-core/src/context.rs — no tree-sitter here
pub trait ContextProvider {
    fn context_at(&mut self, doc: &DocSnapshot, offset: usize) -> ExpansionContext;
    fn validate_wrap(&mut self, doc: &DocSnapshot, range: Range<usize>) -> WrapSelection;
}
pub enum Scope { Root, TypeBody, FnBody, Loop, Conditional, MatchBody, TryBody, Unknown }
```

`Engine::expand()` takes `&ExpansionContext` from day one (M1 uses `NullContextProvider`: scopes=[Root], indent from line prefix). **codemmet-tsctx (M3):** compiled-in grammar crates behind features; per-URI parser/tree cache (full reparse first, incremental later); `context_at` walks ancestors from `named_descendant_for_byte_range`, maps raw kinds → normalized `Scope` via per-language tables; string/comment suppression for completion; tree-derived indent depth. `validate_wrap` (M4): snap selection to smallest covering statement run, reject cross-scope selections.

## CLI

```
codemmet expand --lang rust "fn:main>fori10>pr"
echo 'fori10>pr' | codemmet expand --lang go -
codemmet wrap --lang rust --abbr "fori10" < selection.txt
codemmet list --lang rust [--json]
codemmet check                # validate user snippet TOMLs
codemmet lsp [--stdio]
```

Flags: `--lang` (required, accepts extensions), `--indent`, `--base-indent`, `--tabstops keep|strip` (default strip), `--json`. **Shipped note:** `--json` emits `{text, cursor, is_snippet}` (not the `tabstops` array originally sketched) — in strip mode `cursor` is a `{line, character}` byte position and `is_snippet` is false; in keep mode `text` carries the LSP snippet syntax and `cursor` is null. `check` also warns on stderr about `*.toml` files in the override directory whose names don't match a language pack. Exit codes: 0 ok, 1 parse error, 2 unknown trigger (with suggestion) / render error, 3 config or unknown-language error. Diagnostics on stderr only.

## Neovim Plugin (same repo, nvim ≥ 0.10)

`require("codemmet").setup({ cmd, filetypes, keymaps = { expand = "<A-e>", wrap = "<leader>cw" }, init_options })` — FileType autocmd + `vim.lsp.start` (completion then works with nvim-cmp/blink.cmp/built-in automatically). `expand()`: `client:exec_cmd("codemmet.expand")` → `nvim_buf_set_text` to delete range → `vim.snippet.expand(result.snippet)`. `wrap()`: visual selection + `vim.ui.input` prompt → same apply path. `:CodemmetExpand`/`:CodemmetWrap` user commands. Lua checked with stylua.

## Quality Tooling

- Root `Cargo.toml`: `resolver = "3"`, `[workspace.lints.rust] unsafe_code = "deny"`, `[workspace.lints.clippy] pedantic/nursery = warn (priority -1)`, `unwrap_used = warn`, allow `module_name_repetitions`, `must_use_candidate`, `missing_errors_doc`. Each crate: `[lints] workspace = true`. CI runs clippy with `-D warnings`.
- `deny.toml`: advisories `yanked = deny`; license allowlist (MIT, Apache-2.0 [+LLVM-exception], BSD-2/3, ISC, Unicode-3.0, Zlib); `unknown-registry/git = deny`.
- CI (`ci.yml`, push/PR, `dtolnay/rust-toolchain@stable` + `Swatinem/rust-cache@v2`): fmt (+stylua), clippy `-D warnings --all-features`, test `--all-features`, release build **plus a `--no-default-features` bin build** to keep the tree-sitter-free path honest, `cargo-deny-action@v2`.
- `justfile` (`just fmt/lint/test/snap/ci`) + `lefthook.yml`: pre-commit = fmt checks only (fast); pre-push = clippy + tests.

## Milestones

- **M0 — Scaffolding:** commit `PLAN.md`; workspace conversion; three empty crates compiling; lints, rustfmt.toml, deny.toml, CI, justfile, lefthook, README skeleton. Done when CI is green.
- **M1 — Core engine + CLI (Rust & Python):** abbr lexer/parser; TOML loader + validation; body parser → IR; composer/renderer with tabstop renumbering; `NullContextProvider`; rust.toml + python.toml packs; `expand`/`list`/`check` subcommands.
- **M2 — LSP + Neovim plugin:** lsp-server loop, doc store, completion, `codemmet.expand`, naive `codemmet.wrap`; Lua plugin with keymaps.
- **M3 — Tree-sitter context:** codemmet-tsctx; contextual variants (`fn` in impl, `def` in class); string/comment suppression; tree-based indent; incremental sync.
- **M4 — Wrap + remaining languages + polish:** tree-sitter wrap snapping; `^` operator; C/C++/JS/TS/Go packs (TS←JS, C++←C inheritance); release binaries (plain GitHub Actions workflow on `v*` tags); user docs (shipped under `docs/`, with a per-language examples directory). `completionItem/resolve` was dropped in favour of eager documentation (see the LSP section).

**Post-M4 hardening** (review-driven, all shipped): a global composed-line budget plus a parse-time node cap that reject expansion-size blow-ups; snippets rendered without baked `base_indent` for LSP clients; grammar-aware quote handling in candidate extraction; a shared operator set; profile-default indentation for flat buffers; transitive profile inheritance; and `codemmet check` warnings for misnamed packs.

## Verification

- **M1:** parser unit tests (every operator, precedence `a>b+c` vs `a+b>c`, grouping, quoting/escaping, error spans); loader rejection tests; **insta snapshot suite** over `(lang, abbreviation)` pairs in both `--tabstops keep` and `strip` modes — this is the living spec. Manual: `cargo run -- expand --lang rust "fn:main>fori10>pr"`.
- **M2:** in-process LSP integration tests via `lsp_server::Connection::memory()` (initialize → didOpen → completion/executeCommand, snapshot the JSON). Neovim smoke test: `nvim --headless -l tests/smoke.lua` in CI (Linux, best-effort). Manual: open a `.rs` file in nvim with the plugin, type `fori10`, accept completion, jump tabstops.
- **M3:** fixture files with cursor markers → expected `Scope` stacks; same abbreviation snapshotted at two contexts (inside/outside impl).
- **M4:** wrap golden tests (file + range + abbr → snapshot); cross-language consistency test that every pack defines the documented core trigger set.

## Key Risks

1. **Completion UX for composed abbreviations** (clients treat `>`/`:` as non-word chars): mitigated by server-computed textEdit, filterText, operator triggerCharacters, and the explicit expand keymap as the always-works path.
2. **Trigger noise vs real identifiers** (`fn`, `if`): completion is offer-only; add `sortText` de-ranking and a `completion = "always"|"operators-only"|"off"` config.
3. **tree-sitter grammar crate version churn** vs core 0.26 ABI: pin per-grammar versions in M3, isolated in codemmet-tsctx.
4. **tree-sitter-cpp compile time/size:** per-language cargo features.
5. **Tabstop renumbering corner cases** (mirrors across `*`, nested placeholders — reject nesting in v1): defended by the snapshot suite.

## Status and deferred work

All milestones (M0–M4) plus the post-M4 hardening above are complete: seven
languages, ~250 built-in triggers, a CLI, an LSP server, a Neovim plugin, and
full user documentation under `docs/`.

Repetition index variables shipped: `${i}` renders the 1-based copy number of
the enclosing repeated node, both in snippet bodies and in argument values
(e.g. `test:case${i}*3` → `case1`/`case2`/`case3`). It is a `RepeatIndex` span
in the IR; `i` is a reserved parameter name. Group-repeat (`(...)*n`) is
distributed at parse time, so the index applies to `node*n`, not to a group.

Project-local snippet packs shipped: `.codemmet/snippets/<lang>.toml` discovered
by walking up from the working directory (CLI, auto-loaded, `--no-project` to
disable) or the workspace root (LSP, opt-in via the `project_snippets`
initialization option). They are the highest-priority tier —
project > user > builtin — via `Registry::with_dirs`. The LSP defaults to off
because an opened repository is not necessarily trusted.

Live ghost-line preview shipped: a read-only `codemmet.preview` command returns
the clean (strip-mode) expansion of the abbreviation before the cursor, and the
Neovim plugin (opt-in `preview = true`, or `:CodemmetPreviewToggle`) renders it
as dimmed virtual lines below the current line while typing — no buffer edit and
no completion menu.

Interactive trainer shipped: `codemmet train` (behind the default-on `train`
cargo feature) is a full-screen ratatui trainer — with a line-based fallback for
non-TTY use — for learning the abbreviation syntax. It teaches the composition
operators then the trigger vocabulary via mixed reverse/forward exercises,
graded by output-equivalence against the real engine, across four modes
(guided lessons, drills, spaced-repetition review, and a free-play sandbox).
Progress, lifetime stats, and a Leitner review schedule persist under the data
dir (`--reset` clears them). A pure `Trainer` state machine holds all logic; the
front ends only render and forward key events. See `TRAINER_PLAN.md` (design)
and `docs/trainer.md` (user guide).

Deferred features (not built): a packaged VS Code extension (codemmet works
there via generic LSP today), and a cross-language `base.toml` pack
(per-language packs read better; likely dropped rather than built).

Deferred performance work (intentionally left at "M3 scale is fine"): the LSP
server re-parses the whole buffer per edit rather than using tree-sitter
`InputEdit` incremental sync, keeps its tree cache keyed per language rather
than per document, and clones the document text into a `DocSnapshot` per
request. These are contained changes to revisit only if large-buffer latency
becomes a real complaint.

Operational, not yet done: push to a GitHub remote (CI and the release
workflow have never run), cut a `v0.1.0` tag to exercise the release binaries,
and publish to crates.io (manifests already carry the metadata).
