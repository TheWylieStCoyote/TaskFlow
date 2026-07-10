-- Headless end-to-end test for the :CodemmetSnippets picker.
-- Run from the repo root:
--   cargo build -p codemmet && nvim --headless --clean -l tests/nvim/browse.lua

local repo = vim.uv.cwd()
vim.opt.runtimepath:prepend(repo)
-- `--clean -l` skips plugin sourcing; load the user commands explicitly so the
-- test drives the real :CodemmetSnippets entry point.
vim.cmd("runtime! plugin/codemmet.lua")

local binary = repo .. "/target/debug/codemmet"

---Print a failure (plus buffer contents, if given) and exit non-zero.
---@param msg string
---@param bufnr? integer
local function fail(msg, bufnr)
  io.write("BROWSE FAIL: " .. msg .. "\n")
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
  init_options = { snippets_dir = "/tmp/codemmet-browse-nonexistent" },
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

-- Scenario 1: accept an entry. Stub vim.ui.select to record what the picker
-- offered and auto-pick "fori"; its pre-rendered body must be applied as a
-- snippet at the cursor.
local offered ---@type table|nil
vim.ui.select = function(items, opts, on_choice)
  offered = { items = items, opts = opts }
  for _, item in ipairs(items) do
    if item.trigger == "fori" then
      on_choice(item, nil)
      return
    end
  end
  on_choice(nil, nil)
end

local accept_buf = rust_buf("accept", { "" })
vim.api.nvim_win_set_cursor(0, { 1, 0 })
vim.cmd("CodemmetSnippets")

local expanded = vim.wait(5000, function()
  return vim.api.nvim_buf_get_lines(accept_buf, 0, 1, false)[1] == "for i in 0..10 {"
end, 50)
if not expanded then
  fail('accept: line 1 never became "for i in 0..10 {"', accept_buf)
end
if offered == nil or #offered.items == 0 then
  fail("accept: vim.ui.select was not offered any entries")
end
if not offered.opts.prompt:find("rust", 1, true) then
  fail("accept: the prompt does not name the language: " .. tostring(offered.opts.prompt))
end
local first = offered.items[1]
local formatted = offered.opts.format_item(first)
if not formatted:find(first.trigger, 1, true) then
  fail("accept: format_item is missing the trigger: " .. formatted)
end
-- A snippet session with tabstops should be active after the expansion.
if not vim.snippet.active() then
  fail("accept: no snippet session is active after expansion", accept_buf)
end
io.write("browse: accept scenario passed\n")

if vim.snippet.active() then
  vim.snippet.stop()
end
vim.cmd("stopinsert")

-- Scenario 2: an entry WITHOUT a pre-rendered body falls back to inserting the
-- trigger and expanding through the ordinary flow (ported from the merged
-- :CodemmetList picker) — no "needs arguments" notification.
vim.ui.select = function(_, _, on_choice)
  on_choice({ trigger = "fori" }, 1) -- body deliberately absent
end
local notified
local real_notify = vim.notify
vim.notify = function(msg, level)
  notified = { msg = msg, level = level }
end
local fallback_buf = rust_buf("fallback", { "" })
vim.api.nvim_win_set_cursor(0, { 1, 0 })
vim.cmd("CodemmetSnippets")

local fallback_expanded = vim.wait(5000, function()
  return vim.api.nvim_buf_get_lines(fallback_buf, 0, 1, false)[1] == "for i in 0..10 {"
end, 50)
vim.notify = real_notify
if not fallback_expanded then
  fail("fallback: the body-less choice did not insert-and-expand", fallback_buf)
end
if notified and tostring(notified.msg):find("needs arguments", 1, true) then
  fail("fallback: got the old 'needs arguments' notification instead of expanding")
end
io.write("browse: body-less fallback scenario passed\n")

if vim.snippet.active() then
  vim.snippet.stop()
end
vim.cmd("stopinsert")

-- Scenario 3: cancelling the picker must leave the buffer untouched.
vim.ui.select = function(_, _, on_choice)
  on_choice(nil, nil)
end
local cancel_buf = rust_buf("cancel", { "let x = 1;" })
vim.api.nvim_win_set_cursor(0, { 1, 0 })
vim.cmd("CodemmetSnippets")

vim.wait(1000, function()
  return vim.api.nvim_buf_get_lines(cancel_buf, 0, 1, false)[1] ~= "let x = 1;"
end, 50)
if vim.api.nvim_buf_get_lines(cancel_buf, 0, 1, false)[1] ~= "let x = 1;" then
  fail("cancel: the buffer changed although the picker was cancelled", cancel_buf)
end
io.write("browse: cancel scenario passed\n")

io.write("BROWSE OK\n")
os.exit(0)
