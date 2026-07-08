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

For a general overview see the [documentation home](README.md), and for
installing the binary see the [installation](installation.md) guide.

## Neovim (the shipped plugin)

This repository ships a Neovim plugin under `lua/` and `plugin/`. It requires
**Neovim >= 0.10** (it uses `vim.snippet`). The plugin starts the server for the
right filetypes, wires up completion through your existing completion UI, and
installs expand and wrap key bindings.

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
| `keymaps` | `table` | `{ expand = "<A-e>", wrap = "<leader>cw" }` | Key bindings to install. Set either field to `false` to disable that binding. |
| `init_options` | `table` | `{}` | Forwarded verbatim as the LSP `initializationOptions`. See the [initializationOptions reference](#initializationoptions-reference). |
| `preview` | `boolean` | `false` | Show a live ghost-line preview of the expansion while you type (see below). |
| `notes_file` | `string` | `<data>/codemmet/notes.md` | Where `:CodemmetNote` appends dogfooding notes. |

On setup, a `FileType` autocommand starts the server with `vim.lsp.start` for each
configured filetype. Completion then flows through whatever completion UI you use.

### Keymaps and commands

The key bindings are installed buffer-locally when the server attaches
(`LspAttach`):

| Binding | Default | Modes | Action |
| --- | --- | --- | --- |
| `keymaps.expand` | `<A-e>` | insert and normal | Expand the abbreviation immediately before the cursor. |
| `keymaps.wrap` | `<leader>cw` | visual | Wrap the selected lines in an abbreviation (prompts for it). |

The plugin also defines these user commands:

| Command | Range | Action |
| --- | --- | --- |
| `:CodemmetExpand` | no | Expand the abbreviation before the cursor. |
| `:CodemmetWrap` | yes | Wrap the given line range in an abbreviation. |
| `:CodemmetCapture {name}` | yes | Capture the selected lines into the project snippet pack as a reusable snippet. Prompts for `{name}` when omitted. |
| `:CodemmetPreviewToggle` | no | Toggle the live ghost-line preview in this buffer. |
| `:CodemmetNote {text}` | no | Append a dogfooding note (with the current file/line as context) to the notes file. Prompts when `{text}` is omitted. |

`:CodemmetCapture` runs the `codemmet capture` CLI on the selection (feeding it on standard input), which generalizes it — identifiers that repeat become `${param}` mirrors — and writes it into the resolved `.codemmet/snippets/<lang>.toml` pack. The plugin then calls the server's read-only `codemmet.reloadSnippets` command so the new trigger expands immediately in the same session. The language server itself never writes files: the CLI is the only writer, and `reloadSnippets` just re-reads packs from disk. See the [`capture` CLI reference](cli.md#capture) for the generalization rules and where the pack lands.

Set a keymap field to `false` to skip that binding, for example
`keymaps = { expand = "<A-e>", wrap = false }`. You can still reach the disabled
action through its user command or by calling the Lua function directly.

### Live preview

With `preview = true`, the plugin shows the expansion as **dimmed virtual lines
below the current line** while you type an abbreviation in insert mode — no
buffer edit, no completion menu required. It updates as you type (debounced) and
clears when you accept the abbreviation, leave insert mode, or move away.

```lua
require("codemmet").setup({ preview = true })
```

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

### Using a specific completion engine

The server is an ordinary LSP completion source, so any completion UI picks it up
once the server is attached:

- **Built-in `vim.lsp.completion`** — enable it for the buffer and codemmet
  completions appear alongside other LSP sources.
- **nvim-cmp** — add the `nvim_lsp` source (as you would for any language
  server); codemmet's items show up automatically.
- **blink.cmp** — its LSP source surfaces codemmet completions with no extra
  configuration.

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

Example, forwarding both options through the plugin:

```lua
require("codemmet").setup({
  init_options = {
    snippets_dir = vim.fn.expand("~/.config/codemmet/snippets"),
    completion = "operators-only",
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
