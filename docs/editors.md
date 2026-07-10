# Editor setup

codemmet is a language server. The `codemmet` binary speaks the Language Server
Protocol over stdio when you launch it as `codemmet lsp`, so any LSP-capable
editor can drive it. There is no editor-specific magic to learn — you point your
editor's LSP client at that command, associate it with the languages codemmet
supports, and you get expansions.

There are two ways to trigger an expansion:

- **Completion as you type.** As you type an abbreviation, the server offers the
  expansion as a snippet-kind completion item. Accepting it inserts the
  expansion with tabstops (`$1`, `${1:x}`, `$0`) you can jump between. This works
  in every LSP client with completion support and needs no extra key bindings.
  Completion is intentionally suppressed inside strings and comments (the server
  is tree-sitter aware). After a [composite](snippets.md#composite-snippets)
  trigger and its argument, typing a comma also offers that composite's
  selectable **option names** (`classp:Point,` → `eq`, `hash`, `len`).
- **Explicit expand and wrap commands.** The server exposes two workspace
  commands: `codemmet.expand` expands the abbreviation before the cursor, and
  `codemmet.wrap` wraps a selection in an abbreviation. Binding these to keys
  gives you an on-demand expansion that does not depend on the completion menu.
  This path needs a client that can invoke arbitrary commands (`workspace/executeCommand`)
  and apply the returned snippet edit.

The server also ships read-only introspection commands any client can call:
`codemmet.list` returns every trigger registered for a document (resolved
through that document's snippet tiers, with a rendered body and the scope
variant that would fire at a given position), and `codemmet.context` returns
the expansion context at a position — the scope stack and the in-scope
collections data-driven repeats can expand over. The Neovim plugin surfaces
them as a snippet browser and a context viewer (see below).

For a general overview see the [documentation home](README.md), and for
installing the binary see the [installation](installation.md) guide.

## Neovim (the shipped plugin)

This repository ships a Neovim plugin under `lua/` and `plugin/`. It requires
**Neovim >= 0.10** (it uses `vim.snippet`). The plugin starts the server for the
right filetypes, enables completion as you type (built-in LSP completion on
Neovim 0.11+, or through your completion plugin), and installs expand and wrap
key bindings. `:checkhealth codemmet` reports the binary, version support, and
whether completion wiring is active.

### Install

Point a plugin manager at the repository. The plugin calls `codemmet` on your
`PATH` by default, so the binary must be installed and reachable. Here is a
verified lazy.nvim specification:

```lua
{
  "<your-github>/codemmet",
  build = "cargo build --release",
  ft = {
    "rust", "python", "c", "cpp",
    "javascript", "javascriptreact", "typescript", "typescriptreact",
    "go",
  },
  opts = {
    cmd = { "codemmet", "lsp" },              -- or an absolute path to the release binary
    keymaps = { expand = "<A-e>", wrap = "<leader>cw" },
  },
}
```

Note the difference between building and installing:

- `build = "cargo build --release"` compiles the binary when the plugin is
  installed, but it leaves the result in `target/release/codemmet` — it does
  **not** put `codemmet` on your `PATH`.
- The plugin's `cmd` defaults to `{ "codemmet", "lsp" }`, which looks up
  `codemmet` on `PATH`.

So you must do one of two things: either run `cargo install --path crates/codemmet`
to place `codemmet` on your `PATH`, or set `cmd` to the absolute path of the built
binary, for example `cmd = { "/path/to/codemmet/target/release/codemmet", "lsp" }`.
See [installation](installation.md) for details, and
[troubleshooting](troubleshooting.md) if the editor reports `command not found`.

### Local install (no remote repository)

You do not need to publish the repository to GitHub. lazy.nvim loads a plugin
from a local directory when you give it `dir` instead of a `"owner/repo"`
string. Point `dir` at your clone and, because `build` leaves the binary inside
that clone, point `cmd` at the built binary by absolute path:

```lua
{
  dir = "/home/you/src/codemmet",             -- your local clone
  name = "codemmet",
  build = "cargo build --release -p codemmet", -- builds ./target/release/codemmet
  ft = {
    "rust", "python", "c", "cpp",
    "javascript", "javascriptreact", "typescript", "typescriptreact",
    "go",
  },
  opts = {
    cmd = { "/home/you/src/codemmet/target/release/codemmet", "lsp" },
    keymaps = { expand = "<A-e>", wrap = "<leader>cw" },
  },
}
```

`name = "codemmet"` makes lazy run `require("codemmet").setup(opts)` (it derives
the module from the plugin name). If you would rather keep `cmd` short, drop the
`build` line, run `cargo install --path /home/you/src/codemmet/crates/codemmet`
yourself, and let `cmd` default to `{ "codemmet", "lsp" }` — then the plugin
spec only needs `dir`, `name`, `ft`, and `opts.keymaps`.

The same idea works for other managers: packer uses no network for a local
path if you clone it into its `start`/`opt` directory yourself; with a bare
`runtimepath` setup, `vim.opt.runtimepath:append("/home/you/src/codemmet")`
then `require("codemmet").setup({...})` in your `init.lua` is all it takes.

### `setup()` options

Calling `require("codemmet").setup(opts)` configures the plugin. (With lazy.nvim,
the `opts` table above is passed to `setup()` for you.) All fields are optional:

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `cmd` | `string[]` | `{ "codemmet", "lsp" }` | Command used to start the language server. Use an absolute path if `codemmet` is not on `PATH`. |
| `filetypes` | `string[]` | rust, python, c, cpp, javascript, javascriptreact, typescript, typescriptreact, go | Filetypes the server is started for. |
| `keymaps` | `table` | `{ expand = "<A-e>", wrap = "<leader>cw", snippets = "<leader>cl", context = "<leader>cv" }` | Key bindings to install. Set any field to `false` to disable that binding. |
| `completion` | `boolean` or `table` | `{ enabled = true, autotrigger = true }` | Completion wiring; see [completion](#completion). A boolean is shorthand for `{ enabled = ... }`. |
| `init_options` | `table` | `{}` | Forwarded verbatim as the LSP `initializationOptions`. See the [initializationOptions reference](#initializationoptions-reference). |
| `preview` | `boolean` or `table` | `false` | Live ghost-line preview of the expansion while you type (see below). `true` is shorthand for `{ enabled = true, debounce_ms = 60 }`. |
| `notes_file` | `string` | `<data>/codemmet/notes.md` | Where `:CodemmetNote` appends dogfooding notes. |

On setup, a `FileType` autocommand starts the server with `vim.lsp.start` for each
configured filetype. Completion works out of the box — see [completion](#completion).

### Keymaps and commands

The key bindings are installed buffer-locally when the server attaches
(`LspAttach`):

| Binding | Default | Modes | Action |
| --- | --- | --- | --- |
| `keymaps.expand` | `<A-e>` | insert and normal | Expand the abbreviation immediately before the cursor. In normal mode the expansion is dot-repeatable: `.` re-inserts and expands the last abbreviation at the cursor. |
| `keymaps.wrap` | `<leader>cw` | visual and normal (operator) | Wrap lines in an abbreviation (prompts for it). In normal mode it takes a motion or text object (`<leader>cw ip`) and is dot-repeatable. |
| `keymaps.snippets` | `<leader>cl` | normal | Browse the buffer's snippets in a picker and expand the picked one. |
| `keymaps.context` | `<leader>cv` | normal | Show the codemmet context (scopes, collections) at the cursor. |

The plugin also defines these user commands:

| Command | Range | Action |
| --- | --- | --- |
| `:CodemmetExpand` | no | Expand the abbreviation before the cursor. |
| `:CodemmetWrap` | yes | Wrap the given line range in an abbreviation. |
| `:CodemmetCapture {name}` | yes | Capture the selected lines into the project snippet pack as a reusable snippet. Prompts for `{name}` when omitted. |
| `:CodemmetSnippets` | no | Browse the buffer's snippets in a picker and expand the picked one at the cursor. |
| `:CodemmetContext` | no | Show the codemmet context (scope stack, in-scope collections) at the cursor. |
| `:CodemmetPreviewToggle` | no | Toggle the live ghost-line preview in this buffer. |
| `:CodemmetTrain {args}` | no | Open the interactive [syntax trainer](trainer.md) in a terminal split, training the current buffer's language. Extra `{args}` go to `codemmet train` verbatim. |
| `:CodemmetNote {text}` | no | Append a dogfooding note (with the current file/line as context) to the notes file. Prompts when `{text}` is omitted. |

`:CodemmetCapture` runs the `codemmet capture` CLI on the selection (feeding it on standard input), which generalizes it — identifiers that repeat become `${param}` mirrors — and writes it into the resolved `.codemmet/snippets/<lang>.toml` pack. The plugin then calls the server's read-only `codemmet.reloadSnippets` command so the new trigger expands immediately in the same session. The language server itself never writes files: the CLI is the only writer, and `reloadSnippets` just re-reads packs from disk. See the [`capture` CLI reference](cli.md#capture) for the generalization rules and where the pack lands.

`:CodemmetTrain` opens [`codemmet train`](trainer.md) — the full-screen interactive trainer — in a `botright` terminal split and enters terminal mode, so keys go straight to the TUI. The buffer's filetype picks the trained language when the plugin supports it (Neovim's filetype doubles as the LSP language id, which `--lang` accepts directly); unknown filetypes fall back to the trainer's own default. Any arguments are appended verbatim, e.g. `:CodemmetTrain --mode drills --level 9`. The binary comes from the attached server's `cmd` (falling back to the configured `cmd`, then `codemmet` on `$PATH`), and a binary built without the `train` cargo feature produces a notification instead of a dead split. The trainer is a session you start deliberately rather than an editing motion, so it gets no default keymap — bind `require("codemmet").train` yourself if you want one.

`:CodemmetSnippets` (default keymap `<leader>cl`) is for the "I know it exists
but forgot the trigger" moment: it lists every trigger registered for the
buffer through `vim.ui.select` — so Telescope, fzf-lua, or snacks.nvim provide
fuzzy filtering automatically. Entries are resolved through the buffer's own
snippet tiers and annotated with the scope variant that would fire at the
cursor. Accepting an entry applies its pre-rendered body as a snippet; an
entry without one falls back to inserting the trigger and expanding it through
the ordinary flow, so a trigger with required params works too (its unfilled
params become tabstops).

Set a keymap field to `false` to skip that binding, for example
`keymaps = { expand = "<A-e>", wrap = false }`. You can still reach the disabled
action through its user command or by calling the Lua function directly.

### Health check

`:checkhealth codemmet` verifies the whole chain in one go: the binary is found
and executable, its version matches the plugin checkout, tree-sitter context
support is compiled in (a minimal build degrades silently — see
[troubleshooting](troubleshooting.md#a-minimal-build-has-no-context-awareness)),
the snippet packs pass `codemmet check`, and the language server is running.
Run it first whenever an expansion does not happen; for the individual
symptoms it can surface, see [troubleshooting](troubleshooting.md).

### Live preview

With the preview enabled, the plugin shows the expansion as **dimmed virtual
lines below the current line** while you type an abbreviation in insert mode —
no buffer edit, no completion menu required. It updates as you type (debounced)
and clears when you accept the abbreviation, leave insert mode, or move away.

```lua
require("codemmet").setup({ preview = true })
```

The option also takes a table for finer control:

```lua
require("codemmet").setup({
  preview = {
    enabled = true,     -- default true when a table is given
    debounce_ms = 100,  -- typing-pause before the server is queried (default 60)
  },
})
```

The boolean form remains fully supported: `preview = true` is shorthand for
`{ enabled = true, debounce_ms = 60 }`, and `false` (the default) or omitting
the option disables the preview. In the table form, `enabled` defaults to
`true` — passing a table opts in unless you set `enabled = false` explicitly
(useful to pre-configure `debounce_ms` for `:CodemmetPreviewToggle` without
turning the preview on everywhere). `debounce_ms` must be a positive integer;
an invalid value falls back to the default 60 ms with a warning at setup.

When you type faster than the server replies, the plugin cancels the
superseded in-flight `codemmet.preview` request (and ignores its response if
it arrives anyway), so a stale ghost expansion never overwrites a newer one.

The preview is off by default because it updates on every keystroke; enable it
globally as above, or toggle it per buffer at runtime:

```vim
:CodemmetPreviewToggle
```

Under the hood this uses a read-only `codemmet.preview` server command that
returns the clean (strip-mode) expansion of the abbreviation before the cursor,
so it works in any client that can call `workspace/executeCommand` and render
virtual text — the shipped Neovim plugin renders it with `nvim_buf_set_extmark`
virtual lines.

### Snippet browser and context viewer

`:CodemmetSnippets` (default `<leader>cl`) opens a `vim.ui.select` picker over
every trigger registered for the current buffer — through that buffer's own
snippet tiers, so project packs and path-mapped overrides are included. Each
entry shows the trigger, its description, the scope variant that would fire at
the cursor (e.g. `fn` inside an `impl` shows `[type-body]`), and a one-line
hint of what it expands to. Picking an entry expands it at the cursor with
tabstops, exactly like the expand keymap. Because it goes through
`vim.ui.select`, pickers such as telescope, fzf-lua, or snacks take over
automatically when they are installed.

`:CodemmetContext` (default `<leader>cv`) shows the codemmet context at the
cursor in a small floating window: the syntactic scope stack (e.g.
`root > fn-body`) and the in-scope collections — enum variants, struct fields,
list bindings — that [data-driven repeats](context-codegen.md) like
`match:Msg>arm*` can expand over. Useful for checking what a `*@name` repeat
would see before you type it. The float closes when the cursor moves.

Both are read-only server commands (`codemmet.list`, `codemmet.context`), so
any LSP client able to call `workspace/executeCommand` can build the same
surfaces.

### Code actions

codemmet advertises `textDocument/codeAction`, so no plugin wiring is needed:
put the cursor inside a `match` over an enum value and trigger your editor's
code actions to get **"Fill match arms from `Name`"**, which inserts one arm per
variant (see [context-aware & data-driven expansion](context-codegen.md#code-actions)).
In Neovim:

```vim
:lua vim.lsp.buf.code_action()
```

or bind it, e.g. `vim.keymap.set("n", "<leader>ca", vim.lsp.buf.code_action)`.
The edit is plain text, so it applies in any LSP client.

### Wrapping a selection

The `wrap` binding operates line-wise. Select the lines you want to wrap in
visual mode and press `<leader>cw` (or run `:CodemmetWrap` over a range). By
default the plugin prompts you for the wrapping abbreviation through
`vim.ui.input`.

If you want your own mapping that skips the prompt, call `wrap` with an
abbreviation:

```lua
require("codemmet").wrap({ abbreviation = "fori10" })
```

The selected lines become the content the abbreviation wraps around. This is
useful for a fixed, frequently used wrapping abbreviation bound to its own key.

### Wrap as an operator, and dot-repeat

In **normal mode**, the same `wrap` binding is an operator: it waits for a
motion or text object and wraps the lines it covers (always line-wise, like the
visual mapping). `<leader>cw ip` wraps the paragraph under the cursor,
`<leader>cw j` wraps the current and next line, `<leader>cw i{` wraps a brace
block. The abbreviation is prompted for as usual.

Both the wrap operator and normal-mode expand are **dot-repeatable**, and `.`
replays the last operation's inputs rather than your keystrokes:

- After `<leader>cw ip` with `fori10`, move to another block and press `.` —
  the same abbreviation is re-applied over the repeated motion at the new
  cursor position, with **no prompt**.
- After a normal-mode `<A-e>` expansion, press `.` anywhere — the last
  abbreviation is re-inserted at the cursor and expanded there.

If you prefer your own mappings, `require("codemmet").wrap_operator` and
`require("codemmet").expand_operator` are the expression-mapping entry points
(bind them with `expr = true`); they arm `operatorfunc` and return `g@` /
`g@l`.

One standard-Vim caveat: if the applied snippet leaves tabstops and you type
into them, that insertion becomes the latest change, so `.` repeats the typed
text instead of the wrap or expansion (the same trade-off every snippet plugin
makes). Leave the snippet without typing (`<Esc>`) and `.` keeps repeating the
codemmet operation.

### Completion

Completion is wired up for you, on by default. When the server attaches, the
plugin enables Neovim's **built-in LSP completion** for it with autotrigger
(`vim.lsp.completion.enable(true, client_id, bufnr, { autotrigger = true })`),
so expansions pop up in the completion menu as you type — no completion plugin
required. The full option shape, with its defaults:

```lua
require("codemmet").setup({
  completion = {
    enabled = true,     -- auto-enable built-in LSP completion on attach
    autotrigger = true, -- open the menu automatically while typing
    mode = nil,         -- server gate: "always" | "operators-only" | "off"
                        -- nil keeps the server default ("always")
  },
})
```

A plain boolean still works as shorthand: `completion = false` disables the
wiring entirely (and `completion = true` is the default behavior spelled out).

Two things to know:

- **nvim-cmp / blink.cmp users need nothing extra.** Those engines already
  consume LSP completion sources, so codemmet's items show up in them
  automatically — and when either is loaded the plugin skips the built-in
  auto-enable to avoid a double completion menu. (Detection looks at loaded
  plugins on each attach; if you lazy-load your engine *after* opening a
  codemmet buffer and see two menus, set `completion = false`.)
- **Built-in auto-enable needs Neovim 0.11+.** The plugin itself runs on 0.10,
  but `vim.lsp.completion` with per-client enable landed in 0.11; on 0.10 the
  wiring quietly no-ops. Run `:checkhealth codemmet` to see whether completion
  wiring is active or skipped and why — the health check also reports the
  binary (`codemmet --version`) and your Neovim's support.

`completion.mode` is the first-class way to set the server's completion gate
(described in the [initializationOptions reference](#initializationoptions-reference)
below): it flows into `initializationOptions.completion`. Precedence: if you
also pass `init_options = { completion = ... }` explicitly, that verbatim value
wins and `completion.mode` is ignored.

```lua
-- Only offer completions for abbreviations with a composition operator:
require("codemmet").setup({
  completion = { mode = "operators-only" },
})
```

### initializationOptions reference

The server accepts these `initializationOptions`. With the plugin, pass them
through `init_options`; both are optional:

| Option | Type | Values | Meaning |
| --- | --- | --- | --- |
| `snippets_dir` | string | a directory path | Directory of user snippet override packs (`<lang>.toml` files). If omitted, the default platform directory is used. |
| `completion` | string | `"always"` (default), `"operators-only"`, `"off"` | When to offer expansion completions. |
| `project_snippets` | boolean | `false` (default), `true` | Load project-local packs from `.codemmet/snippets` (layered up the directory tree). Off by default; see the note below. |
| `paths_file` | string | a file path | Path to a personal `paths.toml` mapping path prefixes to snippet directories (see [custom snippets](snippets.md#personal-path-scoped-packs)). If omitted, the default `<config>/codemmet/paths.toml` is used. |

The default snippet directories are:

| Platform | Default directory |
| --- | --- |
| Linux | `~/.config/codemmet/snippets/` |
| macOS | `~/Library/Application Support/codemmet/snippets/` |
| Windows | `%APPDATA%\codemmet\snippets\` |

The `completion` values behave as follows:

- `"always"` — offer expansion completions whenever an abbreviation is present.
- `"operators-only"` — offer completions only when the abbreviation contains a
  composition operator (`>`, `+`, `*`, `:`, `^`). This cuts noise from bare words
  that look like ordinary identifiers. You still expand bare abbreviations with
  the explicit expand keymap or command.
- `"off"` — no completion items at all; you rely on the expand and wrap commands
  or keymaps. Unknown values are treated as `"always"`.

With the plugin, prefer setting the gate through the first-class
`completion = { mode = ... }` option (see [completion](#completion)); it fills
in `init_options.completion` for you. An explicit `init_options.completion`
takes precedence over `completion.mode`.

Example, forwarding both options through the plugin:

```lua
require("codemmet").setup({
  init_options = {
    snippets_dir = vim.fn.expand("~/.config/codemmet/snippets"),
    completion = "operators-only", -- or: completion = { mode = "operators-only" }
  },
})
```

`project_snippets` is off by default on purpose. The CLI auto-loads
`.codemmet/snippets` from a repository, but the editor opens whatever project
you navigate to — including untrusted clones — and a project pack can silently
redefine what your abbreviations expand to (it cannot run code, but it can
mislead). Enable it only for workspaces you trust:

```lua
require("codemmet").setup({
  init_options = { project_snippets = true },
})
```

See [custom snippets](snippets.md) for the snippet pack format that `snippets_dir`
points at, and the precedence rules for project vs user packs.

### Neovim without the plugin

You do not need the shipped plugin. You can start the server directly with
`vim.lsp.start` from your own `FileType` autocommand:

```lua
vim.api.nvim_create_autocmd("FileType", {
  pattern = {
    "rust", "python", "c", "cpp",
    "javascript", "javascriptreact", "typescript", "typescriptreact",
    "go",
  },
  callback = function(ev)
    vim.lsp.start({
      name = "codemmet",
      cmd = { "codemmet", "lsp" },
      init_options = { completion = "always" },
    }, { bufnr = ev.buf })
  end,
})
```

Completion works immediately through your completion UI. For explicit expansion
you would send `workspace/executeCommand` with `codemmet.expand` or
`codemmet.wrap` yourself and apply the returned snippet edit — which is exactly
the plumbing the shipped plugin already does for you, so most people use the
plugin for the keymaps and start raw only when they want full control.

## Other editors and the generic LSP recipe

codemmet works in any LSP-capable editor. The general recipe is the same
everywhere:

1. Configure a new language server whose launch command is `codemmet lsp`
   (stdio transport). If `codemmet` is not on `PATH`, use the absolute path to
   the binary.
2. Associate that server with the language filetypes you want (Rust, Python, C,
   C++, JavaScript, JavaScript React, TypeScript, TypeScript React, Go).
3. Optionally pass `initializationOptions` (`snippets_dir`, `completion`) if your
   client supports sending them.

With that, the server surfaces snippet-kind completions as you type — this part
works in every LSP client. To get an ergonomic explicit-expand key binding, your
editor also needs to support invoking a workspace command
(`workspace/executeCommand` with `codemmet.expand` or `codemmet.wrap`) and
applying the snippet edit the server returns. Editors vary in how much of this
they expose:

- **Helix** — configure `codemmet lsp` as a language server in `languages.toml`
  and attach it to the relevant languages; completions appear automatically.
- **Zed** — register `codemmet lsp` as a custom language server for the
  configured languages; completions appear automatically.
- **VS Code** — see the note below.

Be honest with yourself about capabilities: plain completion works everywhere,
but a dedicated expand keybind requires client support for command invocation
and applying a returned snippet edit.

### A note on VS Code

There is no packaged codemmet VS Code extension. You can still use codemmet in
VS Code by wiring `codemmet lsp` up through a generic LSP bridge extension and
associating it with the supported languages. That gives you completion. A
first-class extension is not part of version 0.1.0.

## See also

- [installation](installation.md) — installing the `codemmet` binary.
- [custom snippets](snippets.md) — writing snippet packs for `snippets_dir`.
- [abbreviation syntax](syntax.md) — the abbreviation language.
- [language reference](languages.md) — supported languages and their triggers.
- [CLI reference](cli.md) — using codemmet from the command line.
- [troubleshooting](troubleshooting.md) — when something does not work.
