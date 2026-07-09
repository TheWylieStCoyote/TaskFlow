# Import requirements (`requires`) — design

Today a snippet that uses a non-prelude symbol must either fully qualify it
(`hmap` renders `std::collections::HashMap::new()`) or silently assume the
import exists. This adds **import requirements**: a snippet declares what it
needs, and expanding it in an editor also inserts the missing
import/use/include at the right place in the file — *if* it is not already
there. The flagship:

```toml
[snippets.hmap]
description = "HashMap binding"
requires = ["std::collections::HashMap"]
body = '''
let mut ${1:map} = HashMap::new();$0
'''
```

Expanding `hmap` in a Rust buffer that already has some `use` lines:

```rust
use std::fs;
use std::collections::HashMap;   // ← inserted, after the last existing use

fn main() {
    let mut map = HashMap::new();
}
```

Expanding it again — or in a file where a `use std::collections::{HashMap,
HashSet};` already provides the name — inserts nothing. The import edit is a
**plain text edit** (no tabstops), so any LSP client can apply it without
cooperating with the codemmet plugin.

## Design decisions (options + recommendation)

### 1. Schema: raw import line vs structured symbol

- **Option A — raw per-language line.** `requires = ["use
  std::collections::HashMap;"]`. Zero formatting code; the author writes
  exactly what lands in the file. But dedup degenerates to exact-line match
  (`use std::collections::{HashMap, HashSet};` would not count as "already
  present"), Go's grouped `import ( … )` block cannot be expressed as a
  standalone insertable line, and formatting drift (`use x ;`) defeats even
  the exact match.
- **Option B — structured symbol, formatted per language.** `requires =
  ["std::collections::HashMap"]`; a small per-language formatter in
  `codemmet-core` turns the spec into the import text, and the same parse
  gives dedup a *symbol* to look for rather than a line. Costs a per-language
  mini-grammar for the spec string.

**Recommendation: Option B.** Dedup needs the symbol anyway, and Go grouping
is impossible under Option A. The per-language spec forms are small and match
what authors already know:

| language | spec form                         | rendered import                        |
| -------- | --------------------------------- | -------------------------------------- |
| rust     | `std::collections::HashMap`       | `use std::collections::HashMap;`       |
| go       | `fmt`, `net/http`                 | `"fmt"` inside the grouped block (or a new `import "fmt"`) |
| python   | `dataclasses:dataclass`           | `from dataclasses import dataclass`    |
| python   | `json`                            | `import json`                          |
| c / cpp  | `<stdio.h>`, `"local.h"`          | `#include <stdio.h>` / `#include "local.h"` (brackets/quotes are **required**, they disambiguate system vs local) |
| js / ts  | `module:name` (deferred, see §5)  | `import { name } from "module";`       |

A raw-line escape hatch (`raw:` prefix) is deferred until a user pack needs
something the specs cannot express.

**Where it lives: per-snippet AND per-variant.** `SnippetDef` (in
`crates/codemmet-core/src/config/schema.rs`, `deny_unknown_fields`) gains
`requires: Vec<String>` (default empty). `VariantDef` gains `requires:
Option<Vec<String>>` — `Option` so "absent" and "declared empty" are
distinct. When a variant is selected *and* declares `requires`, its list
**replaces** the snippet-level one; absent means inherit. This mirrors how a
variant's `body` fully replaces the default body (first match wins), and an
empty list lets a variant opt *out* (e.g. a `type-body` variant that renders
a fully-qualified form). The alternative — additive union — was rejected
because a scope variant that needs a *different* import would still drag the
base import in.

**Composites: aggregate automatically, plus their own.** A composite's
expansion is an abbreviation re-expanded at render time
(`loader.rs::CompiledComposite`), so if the renderer collects the requires of
every snippet it actually renders, composite aggregation falls out for free —
`classp:Point,eq` picks up whatever `class`/`init`/`__eq__` declare, and
stays correct when those change (the same DRY argument
`COMPOSITE_SNIPPETS.md` makes for bodies). A composite may also declare its
own `requires`, merged in (deduplicated by spec). Recommended; the
alternative (composites declare everything explicitly) re-introduces exactly
the drift composites exist to prevent.

### 2. Delivery mechanism

- **Option A — `additionalTextEdits` on the completion item.** The standard
  LSP auto-import mechanism: the client applies them atomically with the main
  `textEdit` on accept, in every LSP client, with no position-shift races
  (both edits are against the same document version). Covers only the
  completion path.
- **Option B — extend the command return shape.** `codemmet.expand` /
  `codemmet.wrap` return `{range, snippet, isSnippet}` today
  (`crates/codemmet/src/lsp/commands.rs`); add `extraEdits: [TextEdit]` and
  teach the Neovim plugin (`lua/codemmet/lsp.lua::M.expand` →
  `apply_snippet`) to apply them. Non-standard: every client integration must
  cooperate.
- **Option C — both.**

**Recommendation: Option C, completion first.** Completion is where most
expansions happen and `additionalTextEdits` is free correctness
(`lsp_types::CompletionItem` already has the field; `completion.rs` just
never sets it). The command path is how the Neovim `<Plug>` mapping and
`:CodemmetWrap` work, so it needs the richer shape eventually — but it can
ship a milestone later. Client apply order for the command path: **main edit
first, then extra edits** — the import insertion point is at/above the top of
the file, so applying the expansion (below it) does not move it; the plugin
must still shift an extra edit's line by the main edit's line delta in the
rare case the expansion happens *above* the import block (cursor at the very
top of the file).

**Ghost preview: no import shown.** `codemmet.preview` renders strip-mode
virtual lines *below the current line* (`lua/codemmet/preview.lua`); an
import drawn there would appear in the wrong place (it belongs at the top of
the file), and the preview's job is the local expansion. A future refinement
— a one-line `+ use std::collections::HashMap` annotation in the preview
result so the plugin can hint it — is listed under open questions, not v1.

### 3. Dedup detection

- **Option A — exact match on the formatted line.** Trivial, but misses brace
  groups, `from x import a, b`, and grouped Go imports — inserting duplicates
  in exactly the files that are already well-organized.
- **Option B — full symbol resolution.** Handle aliases (`as`), globs,
  re-exports. That is rust-analyzer's job; same argument as
  `CONTEXT_CODEGEN_PLAN.md`'s "no type inference".
- **Option C — tree-sitter import statements + conservative token match.**
  The provider lists the file's import *statements* (per-language node kinds:
  `use_declaration`, `import_statement` / `import_from_statement`,
  `preproc_include`, `import_declaration` / `import_spec`); the required
  symbol counts as present when its dedup key appears **at token boundaries
  inside an import statement's text** — Rust: the final path segment
  (`HashMap` matches inside `use std::collections::{HashMap, HashSet};`);
  Go/C: the exact import path / header name; Python: the module for `import
  m`, the name for `from m import n`.

**Recommendation: Option C**, and the safe direction is explicit: the two
failure modes are not symmetric. A **false negative** ("not present" though
it is) inserts a harmless duplicate the user sees immediately and any
formatter/linter flags. A **false positive** ("present" though it is not)
silently ships broken code. So the matcher only claims "present" on the
confident token match above, and known blind spots resolve toward
*inserting*: a glob (`use std::collections::*;`) does not match `HashMap`, so
we insert a redundant-but-valid `use`. The one accepted false positive:
`use other::HashMap;` or `use x::HashMap as Map;` matches the token — in the
first case the *name* is genuinely bound (inserting ours would conflict, so
skipping is right); the alias case is rare enough to document as a limitation
rather than engineer around.

### 4. Insertion point per language

Computed from the tree in **codemmet-tsctx** (it needs the parse), per
language:

| language | with existing imports | with zero imports |
| -------- | --------------------- | ----------------- |
| rust     | after the last top-level `use_declaration` | after inner attributes (`#![…]`) and inner doc comments (`//!`), else offset 0 |
| go       | inside the last grouped `import ( … )` block, before `)`; else a new `import "…"` after the last `import_declaration` | after the `package_clause`, with a separating blank line |
| python   | after the last top-level `import_statement` / `import_from_statement` (this automatically lands after `from __future__ import …`, which are ordinary import nodes and always first) | after the module docstring (a leading expression-statement string), else offset 0 |
| c / cpp  | after the last top-level `preproc_include` (including ones nested in a header-guard `preproc_ifdef`) | after `#pragma once`, or after the guard's `#define` line inside `#ifndef`/`#define`, else offset 0 |
| js / ts  | after the last `import_statement` | after a shebang / leading `"use strict";`, else offset 0 |

All variants insert a whole line (`<import>\n`) at a line-start byte offset,
so the edit is a pure insertion that never rewrites existing text.

**How it reaches the LSP — two options:**

- **Option A — piggyback on `ExpansionContext`** (a new `imports:
  Option<ImportScan>` field next to `collections`), filled eagerly by
  `context_at` the way collections are.
- **Option B — a new default-implemented `ContextProvider` method**,
  `fn import_scan(&mut self, doc: &DocSnapshot) -> Option<ImportScan>`
  (default `None`), following the `fill_targets` precedent in
  `crates/codemmet-core/src/context.rs`.

**Recommendation: Option B.** The engine never reads import data — dedup and
edit construction are LSP-layer concerns — so putting it in
`ExpansionContext` would bloat a struct whose consumer is the renderer.
`fill_targets` already established the pattern of provider capabilities the
engine ignores, and the second provider call is effectively free: the
tree cache in `TreeSitterProvider::tree_for` is keyed by a text hash, so the
completion request's `context_at` parse is reused. The scan itself is
cursor-independent (file-level), which also argues against the
cursor-anchored `context_at`.

```rust
/// What the provider knows about a file's imports (codemmet-core/src/context.rs).
pub struct ImportScan {
    /// Text of each import statement, for dedup token-matching.
    pub statements: Vec<String>,
    /// Byte offset (a line start) where a new import line is inserted.
    pub insertion_offset: usize,
    /// Go only: byte offset inside an existing grouped block, if one exists.
    pub grouped_insertion_offset: Option<usize>,
}
```

### 5. Languages / rollout

**Recommendation: v1 = Rust, Go, Python, C, C++; JS/TS deferred to v2.**
Rust and C/C++ have the highest builtin value (see the survey below) and the
simplest insertion; Go users *expect* auto-import (even though goimports
would also fix it on save, the in-editor experience should not depend on a
save hook). The JS/TS spec grammar is the hardest (named vs default vs
namespace imports, quote style) and **zero current builtins need it** —
`impt` in `typescript.toml` *is* an import snippet. The schema accepts
`requires` for every language from day one (user packs may declare them);
`codemmet check` warns when the language has no formatter yet (§7).

**Builtin candidates per language** (surveyed from
`crates/codemmet-core/snippets/*.toml`):

- **rust**: `hmap` → `std::collections::HashMap` (and its body drops the
  `std::collections::` qualification — see open questions). Natural new
  builtins once the mechanism exists: `hset`, `btmap`, `arc`, `rc`, `refcell`.
- **c**: `pr`, `prf` → `<stdio.h>`; `malloc`, `free` → `<stdlib.h>`;
  `assert`, `aeq` → `<assert.h>`. The strongest existing set.
- **cpp**: inherits all of C's, plus `cout` → `<iostream>`; `vec` →
  `<vector>`; `map` → `<unordered_map>` (+ `<string>` for its default key
  type); `uptr`, `sptr` → `<memory>`.
- **go**: `pr`, `prf`, `prd` → `fmt`; `err` → `errors`; `test`, `aeq` →
  `testing` (usually already imported in a `_test.go`, which is exactly what
  dedup is for).
- **python**: thin among *existing* builtins (they are std-syntax; `dc` is a
  dict comprehension, not a dataclass). The mechanism unlocks new builtins: a
  `dcls` dataclass scaffold requiring `dataclasses:dataclass`, and user packs
  (the likeliest heavy users overall).
- **js/ts**: none today; deferred.

### 6. Failure modes

- **No tree-sitter** (`NullContextProvider`, a `codemmet-tsctx` build without
  the language's `lang-*` feature, or a parse failure): `import_scan` returns
  `None` and requires are **silently skipped** — the expansion itself is
  unchanged and correct, just unaccompanied. This matches the existing
  degradation contract (scope variants and collections also vanish without a
  grammar). No error, no partial guess: a text-heuristic insertion without a
  tree risks Python docstrings and C header guards.
- **Expanding the same trigger twice.** After the first accept the buffer
  *has* the import (the client applied it; unsaved is irrelevant — the LSP
  server tracks buffer text via incremental `didChange`, and
  `TreeSitterProvider`'s cache re-keys on the new text hash). The second
  completion request scans the updated document and dedup catches it. Two
  *pending* completion popups cannot race: `additionalTextEdits` travel with
  the item and are applied atomically on accept against the version they were
  computed for; a stale item is re-requested by the client.
- **Insertion point collides with the expansion.** In a near-empty file the
  computed insertion offset can land on the same line the abbreviation is on.
  LSP forbids `additionalTextEdits` overlapping the main edit; a zero-width
  insertion at the main edit's start is technically legal but client behavior
  varies. v1 rule: when the insertion line equals the main edit's start line,
  **drop the import edit** (safe direction again — the user gets the
  expansion and adds the import; no client ever sees an overlapping edit).
- **Header files.** `.h`/`.hpp` map to the c/cpp profiles, so includes insert
  inside the header guard per the table above; nothing special beyond that.
- **Cursor inside a string/comment** is already gated before any of this runs
  (`completion.rs` returns `None`).

### 7. CLI behavior

`codemmet expand` has no file to edit, so:

- **Plain-text mode: ignore requires.** stdout stays pipeable code, exactly
  as today.
- **`--json` mode: add a field.** The existing object
  (`cli/expand.rs::emit`) gains
  `"requires": [{"spec": "std::collections::HashMap", "line": "use std::collections::HashMap;"}]`
  — both the raw spec and the formatted line, so scripts and the playground
  can render either. (This is also why the per-language *formatter* lives in
  `codemmet-core`, not the tree-sitter crate: the CLI and WASM builds need it
  without a parse.)
- **`codemmet check`**: spec validation happens at pack-compile time in
  `loader.rs::compile_snippet` (same place body/param rules live), so `check`
  gets it for free — a malformed spec (`requires = ["<stdio.h"]`, an empty
  string, a C spec without brackets/quotes) is a `ConfigError::Invalid` with
  the trigger and origin. Additionally `check` **warns** (stderr, non-fatal,
  like the misnamed-pack warning in `cli/check.rs`) when a pack declares
  `requires` for a language whose insertion/formatting support is not
  implemented, so authors are not surprised by silence.

## What is explicitly out of scope

- **Symbol/type resolution.** No alias tracking, no re-export chasing, no
  "which crate provides this" — the spec names exactly one import to write,
  and dedup is a syntactic token match (§3). rust-analyzer-grade auto-import
  is a non-goal.
- **Import ordering/grouping style.** The new line goes after the last
  existing import; it is not sorted into std/external/local groups. Users
  with `goimports`/`isort`/`rustfmt` get that on save.
- **Removing imports** when an expansion is undone.
- **JS/TS formatting** (v2, once a builtin or user pack demands it).
- **Multi-file effects** (e.g. adding a Cargo dependency). Same-file only.

## Architecture

Everything follows the seam that already exists; the engine stays pure and
document-free.

1. **Schema/loader** (`codemmet-core/src/config/`): `SnippetDef.requires:
   Vec<String>`, `VariantDef.requires: Option<Vec<String>>`. `compile_snippet`
   parses each spec through the per-language formatter (new
   `codemmet-core/src/lang/imports.rs`: `parse_spec(lang, s) -> Result<ImportSpec>`,
   `ImportSpec::render_line()`, `ImportSpec::dedup_key()`), storing
   `Vec<ImportSpec>` on `CompiledSnippet` / `CompiledVariant`.
2. **Renderer** (`codemmet-core/src/snippet/render.rs`): collects the
   `ImportSpec`s of every snippet it actually renders — the variant-selected
   one when a variant matched (replace semantics, §1), every sub-snippet of a
   composite's re-expansion, every node of a `>`/`+` abbreviation —
   deduplicated by spec, into a new `Expansion.requires: Vec<ImportSpec>`
   field.
3. **Provider** (`codemmet-tsctx/src/imports.rs`, new module mirroring
   `extract.rs`): per-language import-statement kinds + insertion-point walk,
   surfaced through the new `ContextProvider::import_scan` default method
   (§4). `NullContextProvider` inherits the `None` default.
4. **LSP** (`crates/codemmet/src/lsp/`):
   - *completion*: after `expansion_item` renders, call `import_scan`, drop
     specs whose `dedup_key` token-matches a scanned statement, convert the
     insertion byte offset through the request's existing `LineIndex`, and set
     `additional_text_edits` on the item.
   - *commands*: same computation; `codemmet.expand` / `codemmet.wrap` results
     gain `"extraEdits": [ {range, newText}, … ]` (absent when empty, so
     existing clients are untouched).
5. **Neovim plugin** (`lua/codemmet/lsp.lua`): `apply_snippet` grows an
   optional `extra_edits` parameter — applied *after* the main edit, shifting
   any edit positioned below the replaced range by the main edit's line delta
   (§2).
6. **CLI** (`crates/codemmet/src/cli/expand.rs`): `--json` emits
   `requires`; plain mode ignores it. `check` warnings per §7.

## Schema

```toml
# Per-snippet (rust.toml)
[snippets.hmap]
description = "HashMap binding"
requires = ["std::collections::HashMap"]
body = '''
let mut ${1:map} = HashMap::new();$0
'''

# Per-variant: declared requires REPLACE the snippet-level list.
[snippets.example]
requires = ["std::fmt::Display"]
body = "…"
[[snippets.example.variants]]
context = "type-body"
requires = []          # this variant needs nothing
body = "…"

# Composite: aggregates from class/init/… automatically; may add its own.
[snippets.dcls]
description = "dataclass scaffold (composite)"
requires = ["dataclasses:dataclass"]
expands = "class:${name}>field*"
params = [{ name = "name", kind = "ident", default = "Name" }]
```

Validation (in `compile_snippet`, alongside the existing body/param rules):
every spec parses for the pack's language; duplicates within one list are an
error; `requires` on a `disabled = true` entry is ignored like every other
field.

## Verification

- **Loader unit tests** (`config/loader.rs`): valid specs compile per
  language; malformed specs (`"<stdio.h"`, `""`, bare C header) produce
  `ConfigError::Invalid` naming the trigger; variant replace-vs-inherit
  semantics; composite aggregation through a fixture pack.
- **Formatter unit tests** (`lang/imports.rs`): spec → rendered line →
  dedup key round-trips for all five v1 languages.
- **Dedup + insertion-point unit tests** (`codemmet-tsctx`, the M3
  scope-test pattern: fixture files with a cursor marker): per language, (a)
  scan finds the existing statements, (b) insertion offset with imports /
  with zero imports / Python docstring / `from __future__` / C `#pragma
  once` and `#ifndef` guards / Go grouped block, (c) token-match positives
  (brace group, `from x import a, b`) and negatives (glob, `HashMapExt`).
- **LSP integration tests** (`crates/codemmet/tests/lsp.rs`, the existing
  child-process harness speaking framed JSON-RPC — the in-process
  `Connection::memory()` alternative is possible but the repo's precedent is
  the real binary): completion of `hmap` in a Rust doc asserts
  `additionalTextEdits` with the exact range and `use` line; the same doc
  *with* the import present asserts the field is absent; a
  `codemmet.expand` request asserts `extraEdits`; a doc under
  `NullContextProvider` conditions (unsupported language id) asserts no
  extra edits and an unchanged main edit.
- **CLI golden test** (`tests/cli.rs`): `--json` output carries `requires`.
- **Fuzz**: adversarial `requires` strings in pack fixtures never panic the
  loader (extends the existing config fuzz surface).

## Milestones

- **IR0 — schema + formatter + engine + CLI (no tree-sitter).** `requires`
  fields, spec parsing/validation, `Expansion.requires` collection
  (variants + composites), `--json` field, `check` errors/warnings.
  Golden-tested through the CLI; first reviewable slice.
- **IR1 — Rust end-to-end.** `import_scan` for Rust (scan + insertion +
  dedup), `additionalTextEdits` in completion, the `hmap` builtin flagship,
  LSP integration test.
- **IR2 — command path + Neovim.** `extraEdits` on `codemmet.expand`/`wrap`,
  plugin apply with the line-delta rule.
- **IR3 — remaining v1 languages.** Go / Python / C / C++ scans and
  insertion points, builtin pack `requires` per the survey, fixture tests.
- **IR4 — docs + polish.** A `docs/` guide, cookbook examples, playground
  note (WASM shows the `requires` list textually), `PLAN.md` entry.

## Open questions

- **Do builtin bodies un-qualify?** Making `hmap` render `HashMap::new()` +
  `requires` is the point of the feature, but CLI plain-mode and
  no-tree-sitter users then get code that needs a manual import where today's
  fully-qualified body just works. Options: (a) un-qualify (recommended — the
  editor is the primary surface, and the CLI JSON still reports the
  requirement); (b) keep qualified bodies and let `requires` be purely
  additive for new snippets. Owner call.
- **Variant `requires` semantics** — replace-when-declared (recommended,
  §1) vs additive union. Cheap to flip before IR0.
- **Preview hint** — should `codemmet.preview`'s result carry the pending
  import list so the plugin can render a dimmed `+ use …` annotation? Out of
  v1; decide whether it is IR4 or never.
- **Same-line collision rule** (§6): dropping the import edit is safest, but
  an alternative is prepending the import text into the main edit itself.
- **Raw-line escape hatch** (`raw:` prefix) — wait for a concrete user-pack
  need, or ship in IR0 while the schema is fresh?
