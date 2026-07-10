# Troubleshooting

This page collects the problems people hit most often when using codemmet inside
an editor, each written as a short symptom, cause, and fix. If your problem is
not here, jump to [still stuck?](#still-stuck) at the bottom.

For setup details this page refers back to, see [editor setup](editors.md) and
[installation](installation.md). In Neovim with the shipped plugin, start with
`:checkhealth codemmet` — it checks the binary, its version and tree-sitter
support, the snippet packs, and the server attach status in one report.

### The server does not start in `.jsx` or `.tsx` files

**Symptom.** codemmet works in `.js` and `.ts` files but does nothing in `.jsx`
or `.tsx` files.

**Cause.** Neovim assigns the filetypes `javascriptreact` and `typescriptreact`
to `.jsx` and `.tsx` files. If your configuration's `filetypes` list omits those
two, the server never starts for those buffers.

**Fix.** Include `javascriptreact` and `typescriptreact` in your `filetypes`.
The shipped plugin's default already lists both, so this only bites when you
override `filetypes` and drop them.

### No completions appear

**Symptom.** You type an abbreviation and no expansion completion is offered.

**Cause.** Usually one of:

- The server is not attached to the buffer.
- The buffer's filetype is not in your `filetypes` list.
- The `completion` initialization option is set to `"off"` or `"operators-only"`.
- The cursor is inside a string or a comment, where completion is intentionally
  suppressed.

**Fix.** Check that the server is attached with `:LspInfo` or
`:checkhealth lsp`. Confirm the filetype is one codemmet is configured for.
If you set `completion = "off"`, no completions will ever appear; if you set
`"operators-only"`, see the next entry. Move the cursor out of a string or
comment to confirm the suppression is what you are seeing. See the
[initializationOptions reference](editors.md#initializationoptions-reference).

### Nothing happens for a bare word like `fn` in operators-only mode

**Symptom.** With `completion = "operators-only"`, typing a plain abbreviation
such as `fn` offers no completion.

**Cause.** The `"operators-only"` mode offers completions only when the
abbreviation contains a composition operator (`>`, `+`, `*`, `:`, `^`). A bare
word does not qualify, by design — that mode exists to cut noise from words that
look like ordinary identifiers.

**Fix.** Use the explicit expand keymap (`<A-e>` by default) or the
`:CodemmetExpand` command, which expand regardless of mode. If you would rather
have completions for bare abbreviations too, switch `completion` back to
`"always"`.

### "unknown trigger" error

**Symptom.** codemmet reports an unknown trigger when you try to expand.

**Cause.** The trigger you used is not part of that language's snippet pack.

**Fix.** Run `codemmet list --lang <id>` to see the valid triggers for that
language. codemmet suggests the nearest matching trigger in its error, so check
for a typo. See the [language reference](languages.md) for the supported
languages and their trigger sets.

### Custom snippets do not load

**Symptom.** Your own snippet pack has no effect.

**Cause.** Usually the file is misnamed — it must be `<language-id>.toml`, for
example `rust.toml` — or it is in the wrong directory, or it contains a schema
error.

**Fix.** Run `codemmet check`, which validates the user override directory: it
warns about misnamed files and reports schema errors. Confirm the directory it
checks matches the one you configured (`--config` on the CLI, `snippets_dir` in
`initializationOptions`, or the default platform directory). See
[custom snippets](snippets.md) for the pack format and the
[CLI reference](cli.md) for `codemmet check`.

### Expansion is indented wrong or double-indented

**Symptom.** The expanded code lands at the wrong indentation, or every line is
indented twice.

**Cause.** codemmet emits relative indentation and the editor re-indents from the
insertion point. Old builds could double up here, and editor indentation settings
also matter.

**Fix.** Make sure you are on a recent build; this was a fixed issue. Check that
your editor's `shiftwidth` and `expandtab` are set the way you expect, since the
editor applies them when it places the snippet. For command-line expansion, use
the `--indent` and `--base-indent` options documented in the
[CLI reference](cli.md).

### `codemmet: command not found` in the editor

**Symptom.** The editor reports that `codemmet` cannot be found when starting the
server.

**Cause.** The plugin's `cmd` defaults to `codemmet` on your `PATH`, and the
binary is not there. Compiling with `build = "cargo build --release"` leaves the
binary in `target/release/` without adding it to `PATH`.

**Fix.** Either install it onto your `PATH` with
`cargo install --path crates/codemmet`, or point `cmd` at the absolute path of the
built binary, for example
`cmd = { "/path/to/codemmet/target/release/codemmet", "lsp" }`. See
[installation](installation.md).

### A minimal build has no context awareness

**Symptom.** With a `--no-default-features` build, `&self`/`self` insertion does
not happen and completion is offered inside strings and comments.

**Cause.** Context awareness — the `&self`/`self` decision and the suppression of
completion inside strings and comments — depends on tree-sitter, which is only
present in the default-features build.

**Fix.** Reinstall codemmet with default features. See
[installation](installation.md).

### Go expansions use spaces or the wrong indent in a brand-new empty file

**Symptom.** In a fresh, empty Go file, expansions come out with spaces or an
unexpected indent.

**Cause.** When codemmet cannot detect indentation from the buffer — as in an
empty file — it falls back to the language's default indent, which is tabs for
Go. Once the file has some indented lines, detection takes over.

**Fix.** This resolves itself once the file has indented content. Editors that
send formatting options (`shiftwidth`, `expandtab`) override the fallback, so a
correctly configured editor gets the indentation you expect from the start.

## Still stuck?

Three commands cover most remaining diagnosis:

- `codemmet check` — validates your user override directory and reports misnamed
  files and schema errors. Run it when custom snippets misbehave.
- `codemmet list --lang <id>` — lists the valid triggers for a language. Run it
  when a trigger is rejected. See the [CLI reference](cli.md).
- `:checkhealth lsp` (in Neovim) — confirms whether the codemmet server is
  attached to the current buffer. Run it when completions or commands do nothing.

If none of that helps, revisit [editor setup](editors.md) and
[installation](installation.md) to confirm your configuration.
