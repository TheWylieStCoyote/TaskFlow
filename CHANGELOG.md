# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- **Composite (alias) snippets.** A snippet can now define `expands` instead of
  `body` — an *abbreviation* that is re-expanded at render time — so a
  higher-level scaffold is built out of the trigger vocabulary you already have
  and stays DRY. `${name}` params are passed through to the underlying
  sub-triggers (a bound arg becomes a value, an unbound one drops to that
  trigger's own tabstop), an optional `[snippets.x.options]` table offers
  caller-selectable named fragments (`classp:Point,eq`), and freeform
  `>`-children merge into the scaffold. Composites are ordinary chain nodes,
  may reference other composites (definitional cycles and >16-level nesting are
  rejected), and are validated by `codemmet check`. Builtins ship for every
  language — Python `classp`/`ctxmgr`, Rust `dstruct`/`denum`/`newtype`/
  `testmod`, Go `stringer`/`errtype`/`maincmd`, TS/JS `classc`, C
  `hstruct`/`henum`, and C++ `hclass`/`nsclass`. See
  [custom snippets → composite snippets](docs/snippets.md#composite-snippets).
- **Capture a snippet from example.** `codemmet capture --lang X --name trigger`
  (and `:CodemmetCapture` on a Neovim selection) turns a chunk of real code into
  a reusable parameterized snippet in a project pack — repeated identifiers
  become `${param}` mirrors, so expanding it reproduces the code and a different
  argument stamps out a variant. `--param`/`--literal` steer the guess; the pack
  is re-validated before writing, so it always passes `codemmet check`.
- **Path-scoped snippet packs.** Project packs now layer up the directory tree
  (a monorepo's root pack plus refining subproject packs, nearest wins), and a
  personal `paths.toml` maps path prefixes to snippet directories so a snippet
  set kept outside a repo applies under chosen paths. Precedence:
  `builtin < user < user-path-mapped < project`.
- The language server stays **read-only**: capture writes via the CLI, and a
  new read-only `codemmet.reloadSnippets` command re-reads packs so a freshly
  captured trigger works in the same session. The LSP resolves the pack stack
  per document and caches an engine per resolved directory list.

## [0.3.0] - 2026-07-07

### Added

- **Data-driven expansion (CX0 — engine + CLI).** A repeat can now be driven by
  a named collection instead of a fixed count: `arm*@Msg` repeats over `Msg`,
  and a bare `arm*` repeats over the subject named by the enclosing construct
  (`match:Msg>arm*`). Snippet bodies reference the current item with `${value}`,
  a member with `${value.key}`, and a no-data fallback with `${value:default}`
  (so `arm*3` and plain `arm` are unchanged). The CLI `--repeat NAME=v1,v2,…`
  flag supplies collections (each value may carry `value:type`), so it works and
  is testable without an editor. `value` is now a reserved parameter name.
- **Buffer-sourced collections (CX1–CX3 — live in the editor).** The
  tree-sitter provider now extracts collections from the buffer, so data-driven
  repeats fill themselves with no `--repeat` flag: `match:Msg>arm*` reads the
  variants of a same-file `enum Msg` (CX2) — a Rust tuple variant becomes
  `Write(..)` and a struct variant `Move { .. }`, so the generated arms
  compile — `field*@Point` (or a struct-scoped
  `arm*`) reads the fields of `struct Point` with each field's declared type on
  `${value.type}` (CX3), and `arm*@items` reads the elements of an in-scope
  `let items = […]` binding (CX1). Enum and struct/record definitions are
  visible file-wide; list bindings must be lexically in scope and declared
  before the cursor (nearest wins a name clash). Covered across all seven
  languages where the construct exists — Rust, C/C++, TypeScript (including
  string-union type aliases), Go, Python (`Enum` subclasses and dataclasses),
  and JavaScript (lists only). Extraction is shape-directed, not type-aware:
  you name the source rather than relying on inferring a variable's type.
- **Definition-side snippets `field` and `variant`.** Every pack that has
  records gains a `field` snippet, and every pack with an enum-like construct
  gains a `variant` snippet, so data-driven repeats render to real code both
  ways: `struct:Wrapper>field*@Point` writes field definitions and
  `enum:Color>variant*3` (or `*@Src`) writes enum members — the mirror image of
  consuming them with `arm`.
- **Data-driven collections in the browser playground (CX4).** The playground
  gains an optional "Data-driven collections" box that takes the same
  `name=values` lines as `--repeat`, so `match:cmd>arm*@Msg` and friends are
  demoable without an editor; several data-driven examples ship with it.
- **A [context-aware & data-driven expansion guide](docs/context-codegen.md)**
  covering scope variants and the three buffer sources, plus worked examples in
  the [advanced cookbook](docs/examples/advanced.md).
- **"Fill match arms" code action.** The LSP now advertises
  `textDocument/codeAction`: with the cursor in a Rust `match`, it offers "Fill
  match arms from `Name`" for each in-scope `enum`, inserting one arm per
  variant (payload variants bind wildcards so it compiles; already-covered
  variants are skipped). It's a plain-text edit, so any LSP client applies it
  with no plugin support — a discoverable path to data-driven codegen that
  needs no memorized syntax.

### Testing

- Property-based fuzzing of the abbreviation parser, the template parser, and
  the full `expand` pipeline (`proptest`), asserting they never panic on
  arbitrary or abbreviation-shaped input — runs on stable in CI.
- A [`cargo-fuzz`](fuzz/) scaffold (`parse`, `expand` targets) for
  coverage-guided fuzzing.
- The trainer's TUI event loop is now covered by tests: the loop reads from an
  injectable event source and draws to a test backend, so scripted key
  sequences exercise quit, key routing, release-filtering, reveal, and
  done-screen dismissal without a real terminal.

## [0.2.0] - 2026-07-06

### Added

- **Interactive trainer** (`codemmet train`) — a full-screen ratatui TUI, with a
  line-based fallback for non-TTY use, for learning the abbreviation syntax.
  Four modes: guided **lessons**, random **drills**, spaced-repetition
  **review**, and a free-play **sandbox**. Mixed reverse/forward exercises are
  graded by output-equivalence against the real engine, so practice can never
  drift from actual behavior. Hand-authored lesson progressions ship for all
  seven languages; a Leitner review schedule, lifetime stats, and progress
  persist under the data directory (`--reset` clears them, `--data-file`
  relocates them). See [`docs/trainer.md`](docs/trainer.md).
- **Browser playground and web trainer** — the engine and the trainer compiled
  to WebAssembly and served as static pages: an expansion playground and the
  full trainer (progress saved in `localStorage`). A new `codemmet-wasm` crate
  exposes a tiny hand-rolled C-ABI (no `wasm-bindgen` toolchain needed); the new
  `codemmet-train` library and `codemmet-core` compile for
  `wasm32-unknown-unknown`.
- **Repetition index `${i}`** — the 1-based copy number of the enclosing
  repeated node, usable in bodies and argument values (e.g.
  `test:case${i}*3` → `case1`/`case2`/`case3`).
- **Project-local snippet packs** (`.codemmet/snippets/<lang>.toml`) —
  auto-loaded by the CLI (searching upward from the working directory, disable
  with `--no-project`) and opt-in for the LSP. Highest priority:
  project > user > builtin.
- **Live ghost-line preview** in Neovim — a read-only `codemmet.preview` LSP
  command renders the expansion as dimmed virtual lines while you type
  (`:CodemmetPreviewToggle`, or `preview = true`).
- **`:CodemmetNote`** — capture context-tagged dogfooding notes from Neovim.

### Changed

- `--version` now includes the git commit hash (`-dirty` when built with
  uncommitted changes); `-V` remains the short form.
- Wrap injects the selection at the construct's child slot rather than the final
  cursor.
- Documentation expanded: a verified example per trigger linked from the
  language reference, an advanced-examples cookbook with step-by-step
  breakdowns, the trainer guide, and a README demo GIF.

### Fixed

- Live preview no longer flickers while typing an incomplete abbreviation.

### CI / tooling

- Pinned the Rust toolchain via [`rust-toolchain.toml`](rust-toolchain.toml) so
  a new stable release can't silently break the build.
- CI builds `codemmet-wasm` for `wasm32-unknown-unknown`, and the browser
  playground auto-deploys to GitHub Pages on relevant changes.

## [0.1.0]

Initial release.

### Added

- A single binary that is both a CLI (`expand`, `wrap`, `list`, `check`) and an
  LSP server (`lsp`), plus a Neovim plugin in the same repository.
- The abbreviation mini-grammar: terse triggers with glued or explicit
  arguments and the `>` (child), `+` (sibling), `*` (repeat), `^` (climb-up),
  and `()` (group) composition operators, expanding to code with LSP tabstops.
- tree-sitter context awareness (e.g. `fn` inside an `impl` block gains
  `&self`) and wrap-with-abbreviation.
- Built-in snippet packs for seven languages — Rust, Python, C, C++,
  JavaScript, TypeScript, and Go — with user overrides from the platform config
  directory.
- Strict quality tooling: rustfmt, clippy (pedantic + nursery), cargo-deny, a
  minimal (tree-sitter-free) build check, and comprehensive user documentation
  under `docs/`.
