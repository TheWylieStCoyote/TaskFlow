# Project- and path-scoped custom snippets

This document unifies codemmet's story for **custom snippets that follow the
code they belong to**: snippet sets chosen by the path of the file you are
editing, and workflows for *creating* those snippets from code you already
have. It is both a design (the full picture the user asked for) and a record of
what this branch actually prototypes.

> **Status legend** — 🟢 shipped before this work · 🔵 **built on this branch**
> · ⚪ designed here, not built.

## 1. Where we start

codemmet already resolves three snippet tiers, merged per-trigger with the
later tier winning and `disabled = true` removing a trigger
(`crates/codemmet-core/src/lang/registry.rs`, `Registry::with_dirs`):

```
builtin (+ inherited)  <  user (~/.config/codemmet/snippets)  <  project (.codemmet/snippets)
```

🟢 **Project packs** live in `.codemmet/snippets/<lang>.toml`, discovered by
walking up from the working directory (`find_project_snippets_dir`) — CLI
auto-loads them, the LSP opts in via `project_snippets`. Hand-authored project
snippets therefore work today. This branch builds the *layer above*: choosing
sources by path more richly, and generating snippets instead of hand-writing
them.

## 2. Path-scoped resolution model

### 2.1 The three axes

A snippet source can be attached to a path in three ways. The first exists; the
other two are the design.

1. 🟢 **Committed project pack** — `.codemmet/snippets/` inside the repo.
   Travels with the repository; shared by everyone who clones it.
2. ⚪ **Layered project packs up the tree** — every `.codemmet/snippets` from
   the file's directory up to the filesystem root, not just the nearest.
   Serves monorepos where a subproject refines the repo-wide pack.
3. ⚪ **User path-prefix mapping** — a personal config that maps path prefixes
   to snippet directories, so a snippet set you keep *outside* a repo applies
   whenever you edit files under a chosen path. Lets a personal pattern library
   ride along with projects without being committed.

### 2.2 Precedence

The extended precedence, lowest to highest:

```
builtin (+inherited)
  < user global        (~/.config/codemmet/snippets)
  < user path-mapped   (dirs matched by prefix, shallow prefix → deep prefix)
  < project (up the tree, root-most → nearest)      ← nearest wins ties
```

Rationale for the ordering:

- **Project beats user** (already true): a committed pack is the team's
  intent and should win over a personal one, and it can `disabled = true` a
  personal trigger that conflicts.
- **User path-mapped beats user global**: a mapping is a deliberate, specific
  opt-in ("in *these* paths, also use *this* set"), so it should refine the
  always-on global set.
- **Nearest wins within a tier**: for the layered project packs, a subproject's
  `.codemmet/snippets` is more specific than the monorepo root's, so it is
  compiled *last* and overrides. This is a **merge up the tree, nearest-wins**
  model — not "nearest replaces everything". A subproject that only redefines
  `log` still inherits every trigger the root pack defines. I chose merge over
  nearest-only-replace because it matches how the tiers already compose
  (per-trigger merge) and because a monorepo's root pack is usually the shared
  baseline you want everywhere.

### 2.3 How it extends today's loader

`Registry::with_dirs(user, project)` already takes an ordered source list and
compiles it with `SnippetSet::compile` (later wins). The extension is purely
"gather more directories, in the right order" — the compile step is unchanged:

```rust
// Sketch. Replaces the two Option<&Path> args with an ordered list.
pub fn with_layered_dirs(sources: &[SnippetDir]) -> Result<Self, ConfigError> { … }

struct SnippetDir { origin_label: String, path: PathBuf }

// The CLI / LSP build the list, lowest priority first:
//   1. user global dir
//   2. user path-mapped dirs whose prefix matches the file, shallow → deep
//   3. every .codemmet/snippets from the fs root down to the file's dir
fn resolve_sources(file: &Path, user_global: Option<&Path>, mapping: &PathMapping) -> Vec<SnippetDir>
```

- **Layered project discovery** generalizes `find_project_snippets_dir` from
  "first hit walking up" to "collect all hits, return root-most → nearest".
- **The path mapping** is a new small config file, e.g.
  `~/.config/codemmet/paths.toml`:

  ```toml
  # Personal pattern sets applied by path prefix. Prefixes match the file's
  # absolute path; longer (deeper) prefixes are higher priority.
  [[map]]
  prefix = "~/work/acme"
  dir    = "~/snippets/acme"

  [[map]]
  prefix = "~/oss"
  dir    = "~/snippets/rust-oss"
  ```

  The LSP resolves against the *document* path; the CLI against the working
  directory (or a future `--file` hint). Because it is user config, it is
  trusted the same way `~/.config/codemmet/snippets` is — no new trust
  boundary. Committed project packs remain opt-in in the LSP.

### 2.4 One subtlety: caching

Today the registry is built once per process (CLI) or per LSP session. Path
mapping and layered discovery make the resolved source set **depend on the file
being edited**, so a single cached `Registry` is no longer correct in the LSP
when documents from different subtrees are open at once. The design is to key a
small registry cache by *resolved source-set fingerprint* (the ordered list of
dirs + their mtimes). This branch does **not** implement that — see §5.

## 3. Creation workflows

### 3.1 Hand-authored TOML — 🟢 exists

Write `.codemmet/snippets/<lang>.toml`, run `codemmet check`. Fully documented
in `docs/snippets.md`.

### 3.2 Capture-from-example — 🔵 built (the flagship of this branch)

Turn a chunk of real code into a reusable, parameterized snippet. This is the
"I have a pattern; let me make more of it" loop, end to end (CLI + Neovim).

**Core** (`crates/codemmet-core/src/snippet/capture.rs`, no tree-sitter — works
in `--no-default-features`). `capture(lang, code, options) -> Captured`:

1. **Dedent + retab** the code so it satisfies the template indentation rules
   and adapts to any target indent unit.
2. **Tokenize** into identifiers vs. everything else.
3. **Generalize**: an identifier becomes a `${param}` when it **occurs more
   than once** and is not a language keyword. Repeated occurrences become a
   mirror. Everything else stays literal. Each parameter's `default` is the
   original identifier, so **expanding with no arguments reproduces the
   original** (the round-trip invariant, tested).
4. **Escape** literal `$` and `\` so the body round-trips.
5. Escape hatches: `--param <ident>` forces an identifier to a parameter even
   if it occurs once; `--literal <ident>` keeps one literal even if it repeats.

Why "repeated identifier" as the default signal? With a single example there is
no diff to learn the varying parts from, and a repeated identifier (a type name
echoed across a signature, a variable used several times) is empirically "the
subject" of a pattern, and mirrors keep the copies in sync. Matching is
**whole-token**, so `foo` and `handle_foo` are distinct — predictable, at the
cost of missing substring variation (a documented limitation; hand-edit or
`--param` covers it).

**Persistence** (`crates/codemmet-core/src/config/writer.rs`).
`upsert_snippet(dir, lang, entry, force)` inserts/replaces one
`[snippets.<name>]` table in `<dir>/<lang>.toml`, creating the file (with a
`[meta]` header) if absent. It is minimally invasive — other snippets,
comments, and formatting survive; only the target trigger's tables (including
its `[[…variants]]`) are replaced. **The result is re-validated through the
loader before it touches disk**, so a written pack always passes
`codemmet check`. An existing trigger conflicts (exit 3 / JSON-RPC error)
unless `force`.

**Target directory** (`resolve_capture_dir` in `registry.rs`): the existing
`.codemmet/snippets` at or above the file wins; otherwise a fresh one under the
nearest `.git` root, else beside the file. The CLI's `--user` writes to the
user dir instead.

**CLI** (`crates/codemmet/src/cli/capture.rs`):

```sh
codemmet capture --lang rust --name mkwidget --param Widget - <<'EOF'
fn make_widget() -> Widget {
    Widget::default()
}
EOF
# → captured `mkwidget` (1 parameter(s): Widget) into …/.codemmet/snippets/rust.toml

codemmet expand --lang rust mkwidget          # reproduces the original
codemmet expand --lang rust mkwidgetGadget    # glued arg → Gadget instead
```

**Neovim** (`lua/codemmet/capture.lua`, `:CodemmetCapture [name]`): select
code, run the command, get prompted for a trigger name. It calls the
`codemmet.capture` LSP command, which writes the project pack **and reloads the
registry**, so the new trigger is expandable in the same session — no restart.
Mirrors `wrap.lua` and the `codemmet.wrap` command exactly (same range plumbing,
same client `exec`).

**Why an LSP command (a write) and not a read-only return?** `expand`/`wrap`
return edits and never touch disk. `capture` must persist. It stays safe
because it writes only (a) the file the user explicitly asked to write, (b)
inert snippet text — the same trust argument the docs already make for packs.
Capturing implicitly adopts the project pack dir for the session (sets
`project_dir` and reloads), which is reasonable since the user just chose to
write there.

### 3.3 Implement-an-interface / generate-from-a-type — ⚪ related, NOT built

The user also wants "given an interface/type in the project, generate an
implementation". This is **codegen from a concrete declaration**, not
generalization from an example, so it is a separate feature — but it fits the
same seams:

- It is the discoverable sibling of the existing **fill-match-arms** code action
  (`crates/codemmet-tsctx/src/codegen.rs`) and the data-driven `field*@Type` /
  `arm*@Enum` extraction (`extract.rs`). Both already read types/enums from the
  buffer with tree-sitter.
- Shape: a tree-sitter recognizer finds the target (a Rust `trait`, a Go
  `interface`, a TS `interface`), enumerates its required members, and emits a
  stub `impl`/`class` with one method per member — as a code action (plain-text
  edit) or as a synthesized snippet.
- It belongs in `codemmet-tsctx` (per-language node knowledge), gated behind the
  `tree-sitter` feature, returning `FillTarget`s like `match_arm_fills` does.
- It is **not** a snippet-pack writer — it generates code inline, once, rather
  than registering a reusable trigger. (Capture is the bridge if you *do* want
  it reusable: generate the impl, then `capture` it.)

Deliberately left for a follow-up: it needs a member-enumeration pass per
language and careful signature reproduction, which is its own body of work.

## 4. What this branch built vs. designed

| Piece | State |
| --- | --- |
| `capture()` generalization + round-trip invariant | 🔵 built, unit-tested |
| `upsert_snippet()` pack writer (validate-before-write, comment-preserving) | 🔵 built, unit-tested |
| `resolve_capture_dir()` target resolution | 🔵 built, unit-tested |
| `codemmet capture` CLI subcommand | 🔵 built, integration-tested |
| `codemmet.capture` LSP command + live registry reload | 🔵 built, integration-tested |
| `:CodemmetCapture` + `capture.lua` | 🔵 built, nvim smoke scenario |
| Layered project packs up the tree | ⚪ designed (§2.2–2.3) |
| User path-prefix mapping (`paths.toml`) | ⚪ designed (§2.3) |
| File-scoped registry caching for the above | ⚪ designed (§2.4) |
| Implement-an-interface codegen | ⚪ sketched (§3.3) |

The prototype targets the **existing single `.codemmet/snippets` location** for
writing, which is the honest, well-scoped outcome the task allowed: the capture
loop is complete and real, and the multi-location resolution is fully designed
above rather than half-built.

## 5. Open questions for the user

1. **Merge vs. nearest-only for layered project packs.** §2.2 proposes
   *merge up the tree, nearest wins ties*. Do you ever want a subproject to
   *fully replace* the ancestor set instead of refining it (e.g. a vendored
   subdir with unrelated conventions)? If so we'd need an opt-out marker in the
   subproject pack.
2. **Path-mapping precedence vs. committed packs.** The design puts committed
   project packs *above* user path-mapped ones. If you keep a personal set that
   you want to win even inside a repo (your muscle-memory triggers), we'd need a
   per-mapping "priority: high" flag. Which is the common case for you?
3. **Capture generalization strength.** The default only generalizes *repeated*
   identifiers. For single-occurrence "the name" cases you must pass `--param`.
   Would you prefer a smarter default — e.g. "also generalize the first
   identifier after `fn`/`struct`/`class`" via tree-sitter — accepting more
   magic and per-language code? (Tree-sitter capture would also let us skip
   identifiers inside strings/comments, which the text tokenizer does not.)
4. **Type-aware capture.** Should capture optionally generalize *types* into a
   second class of parameter (e.g. `${T}` for `Widget`, `${U}` for a field
   type), or is name-level generalization enough for now?
5. **LSP capture writing to disk.** Comfortable with the `codemmet.capture`
   command writing files (and adopting the project pack for the session), or
   should the editor path instead shell out to `codemmet capture` so the server
   stays strictly read-only? Both are viable; this branch chose the in-server
   command for the live-reload UX.
6. **Where should a mapping-less capture land?** `resolve_capture_dir` prefers
   an existing pack, else the `.git` root, else beside the file. Is "`.git`
   root" the right default project anchor, or would you rather it always prompt
   for the location in the editor?
