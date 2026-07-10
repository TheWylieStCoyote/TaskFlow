-- Headless end-to-end test for the codemmet render-preview split.
-- Run from the repo root:
--   cargo build -p codemmet && nvim --headless --clean -l tests/nvim/render_preview.lua

local repo = vim.uv.cwd()
vim.opt.runtimepath:prepend(repo)

local binary = repo .. "/target/debug/codemmet"

---Print a failure (plus buffer contents, if given) and exit non-zero.
---@param msg string
---@param bufnr? integer
local function fail(msg, bufnr)
  io.write("RENDER PREVIEW FAIL: " .. msg .. "\n")
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

-- The plugin's user commands come from plugin/codemmet.lua on the runtimepath.
vim.cmd("runtime! plugin/codemmet.lua")

local preview = require("codemmet.render_preview")
preview.config.cmd = binary
preview.config.debounce_ms = 50 -- keep the test snappy; the default is 150

---Edit the source buffer's lines. Headless nvim never emits TextChanged
---itself (there is no main-loop normal-mode cycle in `-l` scripts), so fire
---it explicitly — the plugin's real buffer-local autocmd, debounce timer,
---and generation token all still run.
---@param bufnr integer
---@param start integer 0-based start line
---@param stop integer 0-based end line (exclusive)
---@param lines string[]
local function edit(bufnr, start, stop, lines)
  vim.api.nvim_buf_set_lines(bufnr, start, stop, false, lines)
  vim.api.nvim_exec_autocmds("TextChanged", { buffer = bufnr })
end

---Wait until the preview buffer's contents satisfy `predicate`.
---@param bufnr integer the preview buffer
---@param predicate fun(lines: string[]): boolean
---@return boolean
local function wait_for(bufnr, predicate)
  return vim.wait(5000, function()
    return vim.api.nvim_buf_is_valid(bufnr)
      and predicate(vim.api.nvim_buf_get_lines(bufnr, 0, -1, false))
  end, 25)
end

-- Scenario 1: toggling on opens a scratch split with the rendered output.
local src = vim.api.nvim_create_buf(true, false)
vim.api.nvim_buf_set_name(src, vim.fn.tempname() .. "-main.rs.codemmet")
vim.api.nvim_set_current_buf(src)
vim.api.nvim_buf_set_lines(src, 0, -1, false, { "#lang rust", "", "use:std::fmt" })

vim.cmd("CodemmetRenderPreview")
local state = preview.state(src)
if not state then
  fail("toggle on: no preview state was created", src)
end
if #vim.api.nvim_list_wins() ~= 2 then
  fail("toggle on: expected a second (split) window")
end
if vim.api.nvim_get_current_buf() ~= src then
  fail("toggle on: focus should stay on the source buffer")
end
if vim.bo[state.buf].buftype ~= "nofile" or vim.bo[state.buf].modifiable then
  fail("toggle on: the preview buffer must be a non-modifiable nofile scratch buffer")
end
if not wait_for(state.buf, function(lines)
  return lines[1] == "use std::fmt;"
end) then
  fail("toggle on: the split never showed the rendered output", state.buf)
end
if vim.bo[state.buf].filetype ~= "rust" then
  fail("toggle on: preview filetype should be rust, got " .. vim.bo[state.buf].filetype)
end
io.write("render_preview: open scenario passed\n")

-- Scenario 2: editing the source re-renders the split after the debounce.
edit(src, 3, 3, { "fn:main" })
if
  not wait_for(state.buf, function(lines)
    return lines[1] == "use std::fmt;" and vim.tbl_contains(lines, "fn main() {")
  end)
then
  fail("edit: the split never picked up the new fn:main line", state.buf)
end
io.write("render_preview: edit scenario passed\n")

-- Scenario 3: an invalid line shows the <stdin>:N: diagnostic in the split,
-- highlighted as an error, and the source buffer itself is untouched.
local before = vim.api.nvim_buf_get_lines(src, 0, -1, false)
edit(src, 3, 4, { "???bogus???" })
before[4] = "???bogus???"
if
  not wait_for(state.buf, function(lines)
    return (lines[1] or ""):find("<stdin>:4:", 1, true) ~= nil
  end)
then
  fail("error: the split never showed the <stdin>:4: diagnostic", state.buf)
end
if not vim.deep_equal(vim.api.nvim_buf_get_lines(src, 0, -1, false), before) then
  fail("error: the source buffer was modified by the preview", src)
end
local ns = vim.api.nvim_create_namespace("codemmet_render_preview")
local marks = vim.api.nvim_buf_get_extmarks(state.buf, ns, 0, -1, { details = true })
local highlighted = false
for _, mark in ipairs(marks) do
  if mark[4] and mark[4].line_hl_group == "CodemmetRenderError" then
    highlighted = true
  end
end
if not highlighted then
  fail("error: the diagnostic was not highlighted as an error", state.buf)
end
io.write("render_preview: error scenario passed\n")

-- Scenario 4: a stale error never lingers — fixing the line brings output back.
edit(src, 3, 4, { "fn:main" })
if
  not wait_for(state.buf, function(lines)
    return lines[1] == "use std::fmt;" and vim.tbl_contains(lines, "fn main() {")
  end)
then
  fail("recover: the split never recovered after the bad line was fixed", state.buf)
end
io.write("render_preview: recover scenario passed\n")

-- Scenario 5: toggling off closes the split and cleans up.
local preview_buf = state.buf
vim.cmd("CodemmetRenderPreview")
if preview.state(src) then
  fail("toggle off: preview state should be gone")
end
if #vim.api.nvim_list_wins() ~= 1 then
  fail("toggle off: the split window should be closed")
end
if vim.api.nvim_buf_is_valid(preview_buf) then
  fail("toggle off: the scratch preview buffer should be wiped")
end
local autocmds = vim.api.nvim_get_autocmds({ buffer = src })
for _, au in ipairs(autocmds) do
  if (au.desc or ""):find("render preview", 1, true) then
    fail("toggle off: a render-preview autocmd leaked on the source buffer")
  end
end
io.write("render_preview: toggle-off scenario passed\n")

-- Scenario 6: wiping the source buffer closes the split too. A third window
-- (showing an unrelated buffer) stands in for the rest of a real session, so
-- both the source's window and the preview split can actually close.
local src2 = vim.api.nvim_create_buf(true, false)
vim.api.nvim_buf_set_name(src2, vim.fn.tempname() .. "-gone.rs.codemmet")
vim.api.nvim_set_current_buf(src2)
vim.api.nvim_buf_set_lines(src2, 0, -1, false, { "use:std::fmt" })
preview.attach(src2)
local state2 = preview.state(src2)
if
  not state2
  or not wait_for(state2.buf, function(lines)
    return lines[1] == "use std::fmt;"
  end)
then
  fail("wipe: the second preview never rendered", state2 and state2.buf or nil)
end
local bystander = vim.api.nvim_create_buf(true, false)
vim.api.nvim_open_win(bystander, false, { split = "below", win = 0 })
local preview_win2 = state2.win
vim.cmd("bwipeout! " .. src2)
local closed = vim.wait(5000, function()
  return preview.state(src2) == nil and not vim.api.nvim_win_is_valid(preview_win2)
end, 25)
if not closed then
  fail("wipe: the split did not close when the source buffer was wiped")
end
io.write("render_preview: source-wipe scenario passed\n")

io.write("RENDER PREVIEW OK\n")
os.exit(0)
