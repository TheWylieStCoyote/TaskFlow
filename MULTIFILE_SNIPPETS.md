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
| `on_create` | Text prepended only when the file is created (e.g. `#pragma once`, an import block). |
| `primary` | This section receives the tabstops/final cursor (at most one). |

Exactly one of `path`/`target` and one of `body`/`expands` per section.

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
- target exists → **append** (or, once built, **inject** — see below); an
  `on_exists = "create"` section errors if its target already exists.

`--root <DIR>` is the base for `path` targets (default: the working directory).
`--from <FILE>` supplies the current file that `self`/sibling targets need.

## Editor integration

The manifest is the seam. A future `codemmet.scaffold` LSP command returns the
same JSON; the editor creates/opens the files, applies the edits, and focuses
the primary file to drive its tabstops. (Not built in Phase 1.)

## Status / phases

- **Phase 1 (this):** core + CLI. `path`/`target` resolution, per-piece
  rendering, the manifest, `scaffold --json`/`--write`, and `check` validation.
  `on_exists = "inject"` **degrades to `append`** with a note.
- **Phase 2:** tree-sitter structural **injection** — add a method into an
  existing class body / access section, a field into a struct — so `inject`
  becomes real.
- **Phase 3:** the `codemmet.scaffold` LSP command and the Neovim
  `:CodemmetScaffold` wrapper.

## Known limitations (Phase 1)

- `inject` appends rather than placing content structurally.
- Unknown sub-triggers inside a piece's `expands` are caught at scaffold time,
  not by `check` (composite-style whole-set validation for pieces is future
  work).
- No cross-file tabstops (a fundamental LSP limitation): only the primary file
  is interactive.
