-- Headless end-to-end smoke test for the codemmet Neovim plugin.
-- Run from the repo root:
--   cargo build -p codemmet && nvim --headless --clean -l tests/nvim/smoke.lua

local repo = vim.uv.cwd()
vim.opt.runtimepath:prepend(repo)

local binary = repo .. "/target/debug/codemmet"

---Print a failure (plus buffer contents, if given) and exit non-zero.
---@param msg string
---@param bufnr? integer
local function fail(msg, bufnr)
  io.write("SMOKE FAIL: " .. msg .. "\n")
  if bufnr and vim.api.nvim_buf_is_valid(bufnr) then
    io.write("--- buffer contents ---\n")
    io.write(table.concat(vim.api.nvim_buf_get_lines(bufnr, 0, -1, false), "\n") .. "\n")
    io.write("-----------------------\n")
  end
  os.exit(1)
end

if vim.fn.executable(binary) ~= 1 then
  fail(binary .. " is not built; run `cargo build -p codemmet` first")
end

-- Spy on vim.lsp.completion.enable (when this Neovim has it) so scenario 1 can
-- assert the plugin auto-enabled built-in completion on attach.
local completion_calls = {}
if vim.lsp.completion then
  local real_enable = vim.lsp.completion.enable
  ---@diagnostic disable-next-line: duplicate-set-field
  vim.lsp.completion.enable = function(enable, client_id, bufnr, opts)
    completion_calls[#completion_calls + 1] = { enable = enable, opts = opts }
    return real_enable(enable, client_id, bufnr, opts)
  end
end

require("codemmet").setup({
  cmd = { binary, "lsp" },
  -- `project_snippets` lets the server load `.codemmet/snippets` packs, so the
  -- capture scenario's freshly written trigger is visible after a reload.
  init_options = {
    snippets_dir = "/tmp/codemmet-smoke-nonexistent",
    project_snippets = true,
  },
  -- Table form of the preview option: exercises normalization and the
  -- configurable debounce while keeping the preview off for the scenarios
  -- (scenario 5 drives it explicitly).
  preview = { enabled = false, debounce_ms = 30 },
})

---Create a rust buffer with the given lines and wait for the codemmet client to attach.
---@param name string
---@param lines string[]
---@return integer bufnr
local function rust_buf(name, lines)
  local bufnr = vim.api.nvim_create_buf(true, false)
  vim.api.nvim_buf_set_name(bufnr, vim.fn.tempname() .. "-" .. name .. ".rs")
  vim.api.nvim_set_current_buf(bufnr)
  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
  vim.bo[bufnr].filetype = "rust" -- fires the FileType autocmd, starting the server
  local attached = vim.wait(5000, function()
    local client = vim.lsp.get_clients({ name = "codemmet", bufnr = bufnr })[1]
    return client ~= nil and client.initialized == true
  end, 50)
  if not attached then
    fail("codemmet LSP client did not attach to the " .. name .. " buffer", bufnr)
  end
  return bufnr
end

---Leave any active snippet session / insert mode before the next scenario.
local function reset_state()
  if vim.snippet.active() then
    vim.snippet.stop()
  end
  vim.cmd("stopinsert")
end

-- Scenario 1: expand "fori10" with a realistic normal-mode cursor — after <Esc> the
-- cursor sits ON the last typed character ("0", byte column 5), not one past it.
local expand_buf = rust_buf("expand", { "fori10" })
vim.api.nvim_win_set_cursor(0, { 1, 5 })
require("codemmet").expand()

local expanded = vim.wait(5000, function()
  return vim.api.nvim_buf_get_lines(expand_buf, 0, 1, false)[1] == "for i in 0..10 {"
end, 50)
if not expanded then
  fail('expand: line 1 never became "for i in 0..10 {"', expand_buf)
end
io.write("smoke: expand scenario passed\n")

-- With completion on by default (and no cmp/blink loaded here), the attach above
-- must have enabled built-in LSP completion with autotrigger — on 0.11+ only.
if require("codemmet.completion").supported() then
  local call = completion_calls[1]
  if not call or call.enable ~= true or not (call.opts and call.opts.autotrigger == true) then
    fail("completion: vim.lsp.completion.enable was not called with autotrigger on attach")
  end
  io.write("smoke: completion auto-enable scenario passed\n")
else
  io.write("smoke: completion auto-enable skipped (Neovim < 0.11)\n")
end

reset_state()

-- Scenario 2: wrap a println line in "fori10" (abbreviation passed programmatically
-- to bypass the interactive vim.ui.input prompt).
local wrap_buf = rust_buf("wrap", { 'println!("x");' })
require("codemmet").wrap({ abbreviation = "fori10", line1 = 1, line2 = 1 })

local wrapped = vim.wait(5000, function()
  local lines = vim.api.nvim_buf_get_lines(wrap_buf, 0, -1, false)
  return lines[1] == "for i in 0..10 {" and (lines[2] or ""):match('^%s+println!%("x"%);') ~= nil
end, 50)
if not wrapped then
  fail("wrap: buffer never contained the println nested inside the for loop", wrap_buf)
end
io.write("smoke: wrap scenario passed\n")

reset_state()

-- Scenario 3: expand an abbreviation on an already-indented line; the snippet's
-- continuation lines must pick up the base indent exactly once (a server that bakes
-- base_indent into the LSP snippet would double-indent here).
local indent_buf = rust_buf("indent", { "fn main() {", "    fori10", "}" })
vim.api.nvim_win_set_cursor(0, { 2, 9 }) -- normal-mode cursor ON the trailing "0"
require("codemmet").expand()

local indent_expected = {
  "fn main() {",
  "    for i in 0..10 {",
  "        ",
  "    }",
  "}",
}
local indented = vim.wait(5000, function()
  return vim.deep_equal(vim.api.nvim_buf_get_lines(indent_buf, 0, -1, false), indent_expected)
end, 50)
if not indented then
  fail("indent: buffer never matched the expected indented expansion", indent_buf)
end
io.write("smoke: indent scenario passed\n")

reset_state()

-- Scenario 4: wrap an already-indented line; the server's range starts after the
-- existing indent, so the leading whitespace survives and the wrapped construct
-- keeps its position (a column-0 range would land the loop flush left).
local wrap_indent_buf = rust_buf("wrap-indent", { "fn main() {", '    println!("x");', "}" })
require("codemmet").wrap({ abbreviation = "fori10", line1 = 2, line2 = 2 })

local wrap_indent_expected = {
  "fn main() {",
  "    for i in 0..10 {",
  '        println!("x");',
  "    }",
  "}",
}
local wrap_indented = vim.wait(5000, function()
  return vim.deep_equal(
    vim.api.nvim_buf_get_lines(wrap_indent_buf, 0, -1, false),
    wrap_indent_expected
  )
end, 50)
if not wrap_indented then
  fail("wrap-indent: buffer never matched the expected indented wrap", wrap_indent_buf)
end
io.write("smoke: wrap-indent scenario passed\n")

reset_state()

-- Scenario 5: live ghost-line preview. Headless nvim can't hold insert mode
-- (so the TextChangedI-driven path can't run here), but we exercise both halves
-- directly: the real `codemmet.preview` round-trip returns the clean expansion,
-- and `preview.show` renders it as virtual lines without editing the buffer.
local preview = require("codemmet.preview")
local preview_buf = rust_buf("preview", { "fn main() {", "    fori10", "}" })
local client = vim.lsp.get_clients({ name = "codemmet", bufnr = preview_buf })[1]

local captured
require("codemmet.lsp").exec(client, preview_buf, "codemmet.preview", {
  -- Cursor at the end of line 2 ("    fori10" = 10 columns), 0-based line 1.
  { uri = vim.uri_from_bufnr(preview_buf), position = { line = 1, character = 10 } },
}, function(err, result)
  captured = { err = err, result = result }
end)
local got = vim.wait(5000, function()
  return captured ~= nil
end, 50)
if not got or captured.err or not (captured.result and captured.result.text) then
  fail("preview: codemmet.preview did not return an expansion", preview_buf)
end
if not captured.result.text:find("for i in 0..10 {", 1, true) then
  fail("preview: expansion was not the clean strip form: " .. captured.result.text, preview_buf)
end

preview.show(preview_buf, captured.result.range, captured.result.text)
local preview_ns = vim.api.nvim_create_namespace("codemmet_preview")
local marks = vim.api.nvim_buf_get_extmarks(preview_buf, preview_ns, 0, -1, { details = true })
local rendered = false
for _, mark in ipairs(marks) do
  local virt = mark[4] and mark[4].virt_lines
  if virt and virt[1] and virt[1][1] and virt[1][1][1]:find("for i in 0..10 {", 1, true) then
    rendered = true
  end
end
if not rendered then
  fail("preview: no ghost virtual line was rendered", preview_buf)
end
-- The preview never edits the buffer.
if vim.api.nvim_buf_get_lines(preview_buf, 1, 2, false)[1] ~= "    fori10" then
  fail("preview: the buffer text should be unchanged by the preview", preview_buf)
end
-- clear() removes the ghost lines (and drops any pending request bookkeeping).
preview.clear(preview_buf)
if #vim.api.nvim_buf_get_extmarks(preview_buf, preview_ns, 0, -1, {}) ~= 0 then
  fail("preview: clear() left ghost virtual lines behind", preview_buf)
end
io.write("smoke: preview scenario passed\n")

reset_state()

-- Scenario 6: :CodemmetNote appends a context-tagged entry to the notes file.
local notes = require("codemmet.notes")
local notes_file = vim.fn.tempname() .. "-notes.md"
notes.set_file(notes_file)
local note_buf = rust_buf("note", { "    let mut x = 3;" })
vim.api.nvim_win_set_cursor(0, { 1, 4 })
require("codemmet").note("fori should accept a step")

if vim.uv.fs_stat(notes_file) == nil then
  fail("note: notes file was not created at " .. notes_file, note_buf)
end
local contents = table.concat(vim.fn.readfile(notes_file), "\n")
if not contents:find("fori should accept a step", 1, true) then
  fail("note: the note text was not written", note_buf)
end
if not (contents:find("[rust]", 1, true) and contents:find("let mut x = 3;", 1, true)) then
  fail("note: entry missing the language tag or line context", note_buf)
end
io.write("smoke: note scenario passed\n")

reset_state()

-- Scenario 7: capture a selection into the project pack, then expand the new
-- trigger back. `:CodemmetCapture` shells out to the `codemmet capture` CLI
-- (the server is read-only), which generalizes the repeated `Widget` into a
-- parameter and writes `.codemmet/snippets/rust.toml`; the plugin then reloads
-- the server so the trigger is immediately expandable in the same session.
local capture_buf = rust_buf("capture", { "fn make() -> Widget {", "    Widget::new()", "}" })
local capture_client = vim.lsp.get_clients({ name = "codemmet", bufnr = capture_buf })[1]
-- The CLI runs in the buffer's directory; with no `.git`/`.codemmet` ancestor
-- above the tempdir it writes to `<dir>/.codemmet/snippets/rust.toml`.
local pack = vim.fs.dirname(vim.api.nvim_buf_get_name(capture_buf))
  .. "/.codemmet/snippets/rust.toml"

require("codemmet").capture({ name = "mkw", line1 = 1, line2 = 3 })

local wrote = vim.wait(5000, function()
  return vim.uv.fs_stat(pack) ~= nil
end, 50)
if not wrote then
  fail("capture: pack file was not written at " .. pack, capture_buf)
end
local pack_text = table.concat(vim.fn.readfile(pack), "\n")
if not (pack_text:find("[snippets.mkw]", 1, true) and pack_text:find("${Widget}", 1, true)) then
  fail("capture: pack missing the captured trigger:\n" .. pack_text, capture_buf)
end

-- Wait for a reload to complete (idempotent), so the server sees the new
-- trigger before we expand it — this is what capture does under the hood.
local reloaded = false
require("codemmet.lsp").exec(capture_client, capture_buf, "codemmet.reloadSnippets", {}, function()
  reloaded = true
end)
if not vim.wait(5000, function()
  return reloaded
end, 50) then
  fail("capture: reloadSnippets did not complete", capture_buf)
end

-- Expand the captured trigger: proof the registry reloaded live.
vim.api.nvim_buf_set_lines(capture_buf, 0, -1, false, { "mkw" })
vim.api.nvim_win_set_cursor(0, { 1, 2 }) -- normal-mode cursor ON the trailing "w"
require("codemmet").expand()
local capture_expanded = vim.wait(5000, function()
  return vim.api.nvim_buf_get_lines(capture_buf, 0, 1, false)[1] == "fn make() -> Widget {"
end, 50)
if not capture_expanded then
  fail("capture: expanding the captured trigger did not reload the registry", capture_buf)
end
-- Clean up the pack the capture wrote into the temp directory.
os.remove(pack)
io.write("smoke: capture scenario passed\n")

reset_state()

-- Scenario 8: completion config shape — boolean shorthand, mode → init_options
-- precedence, and engine detection. Pure config assertions (no server, no buffers),
-- so re-running setup() here cannot disturb the finished scenarios.
require("codemmet").setup({ completion = { mode = "operators-only" } })
if require("codemmet").config.init_options.completion ~= "operators-only" then
  fail("completion config: completion.mode did not flow into init_options.completion")
end
require("codemmet").setup({
  completion = { mode = "operators-only" },
  init_options = { completion = "off" },
})
if require("codemmet").config.init_options.completion ~= "off" then
  fail("completion config: explicit init_options.completion must win over completion.mode")
end
require("codemmet").setup({ completion = false })
local bool_config = require("codemmet").config
if bool_config.completion.enabled ~= false or bool_config.completion.autotrigger ~= true then
  fail("completion config: boolean shorthand did not normalize to { enabled = false, ... }")
end
package.loaded.cmp = {}
if require("codemmet.completion").engine() ~= "nvim-cmp" then
  fail("completion config: a loaded nvim-cmp was not detected")
end
package.loaded.cmp = nil
io.write("smoke: completion config scenario passed\n")
-- Scenario 9: scaffold a multi-file snippet. The builtin rust `mmod` appends
-- `mod helper;` to the current file (written to disk) and creates `helper.rs`
-- as the primary, which is opened with a live snippet (its `$0` tabstop is
-- interactive, so the buffer shows the rendered body).
local scaffold_buf = rust_buf("scaffold", { "fn main() {}" })
local scaffold_file = vim.api.nvim_buf_get_name(scaffold_buf)
local scaffold_dir = vim.fs.dirname(scaffold_file)
vim.cmd.write() -- the `self`-append needs the file on disk
require("codemmet").scaffold({ abbr = "mmod:helper" })

-- The primary file `helper.rs` opens with the expanded `pub fn helper()` body.
local scaffolded = vim.wait(5000, function()
  return vim.api.nvim_buf_get_name(0):match("helper%.rs$") ~= nil
    and vim.api.nvim_buf_get_lines(0, 0, 1, false)[1] == "pub fn helper() {"
end, 50)
if not scaffolded then
  fail("scaffold: helper.rs did not open with the expanded module body")
end
-- The supporting file (the current module) got `mod helper;` appended on disk.
local decl = table.concat(vim.fn.readfile(scaffold_file), "\n")
if not decl:find("mod helper;", 1, true) then
  fail("scaffold: `mod helper;` was not appended to " .. scaffold_file)
end
os.remove(scaffold_dir .. "/helper.rs")
io.write("smoke: scaffold scenario passed\n")

reset_state()

io.write("SMOKE OK\n")
os.exit(0)
