# Multi-file snippets

A **multi-file snippet** is one trigger whose content spans several files. It
builds on the composite idea — reuse the trigger vocabulary you already have —
but instead of expanding into a single tree, each *piece* targets a file. One
invocation scaffolds a whole cross-file feature:

- **Django**: a model in `models.py`, a view in `views.py`, a route in `urls.py`.
- **C/C++**: a declaration in the header and a definition in the source.
- **Any project convention** that spreads one logical change across files.

```toml
[snippets.djfeature]
description = "Django model + view + route"
params = [{ name = "app", kind = "ident", default = "app" }]

[snippets.djfeature.files.model]
path      = "${app}/models.py"
on_create = "from django.db import models"
expands   = "class:${app}Model>field"

[snippets.djfeature.files.view]
path    = "${app}/views.py"
primary = true                      # this file keeps the tabstops
body    = '''
def ${app}_view(request):
	${1:pass}
	return render(request, "${app}.html")$0
'''

[snippets.djfeature.files.urls]
path      = "${app}/urls.py"
on_exists = "append"
body      = "path('${app}/', ${app}_view),"
```

```sh
codemmet scaffold --lang python --root . "djfeature:blog" --write
# create blog/models.py
# create blog/views.py
# create blog/urls.py
```

## How it differs from a composite

A composite expands to **one abbreviation tree** and is chainable
(`classp:P>def:x`). A multi-file snippet expands to a **manifest** — a list of
`{path, content, action}` — and is **standalone** (invoked as `djfeature:blog`,
never as a node in a `>`/`+`/`*` chain), because its output is not a single
tree. The two are separate constructs; a file piece's content, however, reuses
the same `body`/`expands` machinery a composite does.

## Schema

A multi-file snippet has `[snippets.<trigger>.files.<label>]` sections instead
of a top-level `body`/`expands`/`composite` (mutually exclusive with all of
them; `scope`/`variants`/`options` are not allowed). `description` and `params`
work as usual; params are **shared** across every section.

Each `[files.<label>]` section:

| Key | Meaning |
| --- | --- |
| `path` | Target path relative to the scaffold root, `${param}`-substituted. |
| `target` | *Instead of* `path`: `"self"` (the current file) or a sibling by extension (`".cpp"`). |
| `body` | A template body (LSP snippet syntax + `${param}`). |
| `expands` | *Instead of* `body`: a composite-style abbreviation re-expanded for this file. |
| `on_exists` | `inject` \| `append` \| `create` when the target already exists (default `inject`). |
| `into` | For `inject`: the name of the container (class/struct/…) to inject into, `${param}`-substituted. Absent injects into the last container. |
| `on_create` | Text prepended only when the file is created (e.g. `#pragma once`, an import block). |
| `primary` | This section receives the tabstops/final cursor (at most one). |

Exactly one of `path`/`target` and one of `body`/`expands` per section.

## Built-in multi-file snippets

Each language ships one to get started:

| Lang | Trigger | What it scaffolds |
| --- | --- | --- |
| Rust | `mmod:name` | a `name.rs` module (primary) + `mod name;` appended to the current file |
| Python | `mmod:name` | a `name.py` module (primary) + a `test_name.py` importing it |
| C | `munit:name` | a `name.h` header (guard + decl) + a `name.c` source (primary) |
| C++ | `mcls:Name` | a `Name.hpp` header with a class (primary) + a `Name.cpp` source |

## Parameters are concrete

Unlike a composite (where an unbound param becomes a mirrored tabstop),
multi-file params resolve to **concrete text** before rendering: the supplied
argument, else the declared `default`. Two reasons:

1. a param usually appears in a file **path**, which must be a real value; and
2. LSP tabstops cannot be kept in sync across files, so a param used in several
   files could not mirror anyway.

Tabstops therefore come only from a piece's own `body`/`expands` (its `$1`,
`$0`, and any sub-trigger defaults), and only the **primary** file keeps them —
every other file is rendered in strip mode. The primary file is the one marked
`primary = true`, else the `self`-targeted section, else the first.

## Applying: the manifest

`codemmet scaffold` resolves the invocation into a manifest and then, depending
on the flags:

- **default** — prints a human-readable dry run;
- **`--json`** — prints `[{path, action, content, primary}]` for an editor to
  apply;
- **`--write`** — applies it directly.

The **action** per file is decided against the filesystem:

- target missing → **create** (content prefixed with `on_create`);
- target exists → **inject** or **append**, per the section's `on_exists`; an
  `on_exists = "create"` section errors if its target already exists.

**Injection** (`on_exists = "inject"`, the default) places the content
*structurally*: tree-sitter finds a type-like container (class, struct, impl,
trait, interface) in the existing file and inserts the content as a new member
at the body's indentation. It targets the container named by `into` (else the
last one), and for a C/C++ class it lands the member at the end of the
`public:` section rather than after a trailing `private:` one. With no matching
container — or in a tree-sitter-free build — it falls back to appending. A JSON
manifest entry for an inject carries the byte `offset` so an editor can apply it
without re-parsing.

`--root <DIR>` is the base for `path` targets (default: the working directory).
`--from <FILE>` supplies the current file that `self`/sibling targets need.

## Editor integration

In Neovim, `:CodemmetScaffold {abbr}` (or `require("codemmet").scaffold`) shells
out to `codemmet scaffold --json --tabstops keep` with `--from` the current
file and `--root` the project root (the read-only server never writes). It
writes every supporting file to disk and opens the **primary** file with its
content inserted as a live `vim.snippet` session, so the primary's tabstops are
interactive. The JSON manifest is the seam any other editor can build on.

## Status / phases

- **Phase 1:** core + CLI. `path`/`target` resolution, per-piece rendering, the
  manifest, `scaffold --json`/`--write`, and `check` validation.
- **Phase 2:** tree-sitter structural **injection** — insert a member into a
  class/struct/impl body of an existing file (Rust, C, C++, Python, JS/TS, Go),
  targeting a named container (`into`) and the C/C++ `public:` section.
- **Phase 3:** the Neovim `:CodemmetScaffold` wrapper, with the primary file
  opened as a live, interactive snippet.

## Known limitations

- Unknown sub-triggers inside a piece's `expands` are caught at scaffold time,
  not by `check` (composite-style whole-set validation for pieces is future
  work).
- No cross-file tabstops (a fundamental LSP limitation): only the primary file
  is interactive; supporting files are inserted as plain text.
