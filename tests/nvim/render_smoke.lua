-- Headless end-to-end smoke test for the plugin's file-mode render
-- (`require("codemmet").render()` / :CodemmetRenderBuffer). Standalone from
-- smoke.lua so it runs even while the capture scenario is red on master.
-- Run from the repo root:
--   cargo build -p codemmet && nvim --headless --clean -l tests/nvim/render_smoke.lua

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

require("codemmet").setup({
  cmd = { binary, "lsp" },
  init_options = { snippets_dir = "/tmp/codemmet-render-smoke-nonexistent" },
})

---Create a rust buffer with the given lines and wait for the codemmet client
---to attach (render resolves the binary from the attached client).
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

---Capture vim.notify calls so error paths can be asserted.
local notifications = {}
local original_notify = vim.notify
---@diagnostic disable-next-line: duplicate-set-field
vim.notify = function(msg, level, opts)
  table.insert(notifications, { msg = msg, level = level })
  return original_notify(msg, level, opts)
end

-- Scenario 1: whole-buffer render — abbreviations become code, interior
-- blank lines pass through, and snippet mode re-enters at the first
-- construct: the buffer shows placeholder text, vim.snippet is active, and
-- jumping walks across constructs to the single final `$0`.
local whole_buf = rust_buf("whole", { "fori10>pr", "", "pr" })
vim.api.nvim_win_set_cursor(0, { 3, 0 })
require("codemmet").render()

local whole_expected = {
  "for i in 0..10 {",
  '    println!("");',
  "}",
  "",
  'println!("");',
}
local whole_ok = vim.wait(5000, function()
  return vim.deep_equal(vim.api.nvim_buf_get_lines(whole_buf, 0, -1, false), whole_expected)
end, 50)
if not whole_ok then
  fail("whole-buffer render never produced the expected code", whole_buf)
end
if not vim.snippet.active() then
  fail("whole-buffer render should re-enter snippet mode", whole_buf)
end
if vim.api.nvim_win_get_cursor(0)[1] ~= 1 then
  fail("snippet re-entry should start at the first tabstop (line 1)", whole_buf)
end
-- `${1:i}` (line 1) → the demoted `$2` inside the first construct's
-- println (line 2) → the last line's surviving `$0` (line 5).
vim.snippet.jump(1)
if vim.api.nvim_win_get_cursor(0)[1] ~= 2 then
  fail("first jump should land inside the first construct (line 2)", whole_buf)
end
vim.snippet.jump(1)
if vim.api.nvim_win_get_cursor(0)[1] ~= 5 then
  fail("second jump should land at the final cursor (line 5)", whole_buf)
end
if not vim.deep_equal(vim.api.nvim_buf_get_lines(whole_buf, 0, -1, false), whole_expected) then
  fail("jumping must not change the buffer text", whole_buf)
end
vim.snippet.stop()
vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes("<Esc>", true, false, true), "nx", false)
io.write("smoke: whole-buffer render + snippet re-entry scenario passed\n")

-- Scenario 2: a failing render leaves the buffer byte-identical (file mode
-- emits nothing on error), moves the cursor to the offending line, and
-- notifies with the <stdin>:N: diagnostic.
local fail_lines = { "pr", "zzznope" }
local fail_buf = rust_buf("fail", fail_lines)
vim.api.nvim_win_set_cursor(0, { 1, 0 })
notifications = {}
require("codemmet").render()

local failed = vim.wait(5000, function()
  for _, entry in ipairs(notifications) do
    if entry.msg:find("zzznope", 1, true) then
      return true
    end
  end
  return false
end, 50)
if not failed then
  fail("failing render never raised a notification", fail_buf)
end
if not vim.deep_equal(vim.api.nvim_buf_get_lines(fail_buf, 0, -1, false), fail_lines) then
  fail("failing render must leave the buffer untouched", fail_buf)
end
if vim.api.nvim_win_get_cursor(0)[1] ~= 2 then
  fail("failing render should move the cursor to the offending line (2)", fail_buf)
end
local err = notifications[#notifications]
if not (err.msg:find("<stdin>:2:", 1, true) and err.level == vim.log.levels.ERROR) then
  fail("failure notification missing the <stdin>:2: line number: " .. err.msg, fail_buf)
end
io.write("smoke: failure-atomicity scenario passed\n")

-- Scenario 3: range render (the :CodemmetRenderBuffer / visual-mode path) —
-- only the selected line is replaced, and its leading whitespace becomes the
-- rendered block's base indent.
local range_buf = rust_buf("range", { "fn main() {", "    fori10>pr", "}" })
require("codemmet").render({ line1 = 2, line2 = 2 })

local range_expected = {
  "fn main() {",
  "    for i in 0..10 {",
  '        println!("");',
  "    }",
  "}",
}
local range_ok = vim.wait(5000, function()
  return vim.deep_equal(vim.api.nvim_buf_get_lines(range_buf, 0, -1, false), range_expected)
end, 50)
if not range_ok then
  fail("range render never produced the expected in-place code", range_buf)
end
if vim.snippet.active() then
  fail("a range render must stay on the plain-text path (no snippet mode)", range_buf)
end
io.write("smoke: range render scenario passed\n")

-- Scenario 4: directives and progressive context through the editor — a
-- #repeat header fills a data-driven repeat, and the enum emitted on an
-- early line feeds the later match (tree-sitter default build).
local prog_buf = rust_buf("progressive", {
  "#repeat E=A,B",
  "arm*@E",
  "enum:Msg>variant*[Quit,Move]",
  "match:m>arm*@Msg",
})
require("codemmet").render()

local prog_expected = {
  "A => ,",
  "B => ,",
  "enum Msg {",
  "    Quit,",
  "    Move,",
  "}",
  "match m {",
  "    Quit => ,",
  "    Move => ,",
  "}",
}
local prog_ok = vim.wait(5000, function()
  return vim.deep_equal(vim.api.nvim_buf_get_lines(prog_buf, 0, -1, false), prog_expected)
end, 50)
if not prog_ok then
  fail("directive + progressive render never produced the expected code", prog_buf)
end
if not vim.snippet.active() then
  fail("progressive whole-buffer render should also re-enter snippet mode", prog_buf)
end
vim.snippet.stop()
vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes("<Esc>", true, false, true), "nx", false)
io.write("smoke: directive/progressive render scenario passed\n")

-- Scenario 5: snippet re-entry opt-out — with `render.snippet_reentry =
-- false` a whole-buffer render takes the plain-text path: same code, cursor
-- at the top, no snippet session.
require("codemmet").setup({
  cmd = { binary, "lsp" },
  init_options = { snippets_dir = "/tmp/codemmet-render-smoke-nonexistent" },
  render = { snippet_reentry = false },
})
local plain_buf = rust_buf("plain", { "fori10>pr", "", "pr" })
vim.api.nvim_win_set_cursor(0, { 3, 0 })
require("codemmet").render()

local plain_ok = vim.wait(5000, function()
  return vim.deep_equal(vim.api.nvim_buf_get_lines(plain_buf, 0, -1, false), whole_expected)
end, 50)
if not plain_ok then
  fail("opted-out whole-buffer render never produced the expected code", plain_buf)
end
if vim.snippet.active() then
  fail("snippet_reentry = false must not enter snippet mode", plain_buf)
end
if vim.api.nvim_win_get_cursor(0)[1] ~= 1 then
  fail("opted-out render should leave the cursor at the top of the output", plain_buf)
end
io.write("smoke: snippet re-entry opt-out scenario passed\n")

io.write("SMOKE OK\n")
os.exit(0)
