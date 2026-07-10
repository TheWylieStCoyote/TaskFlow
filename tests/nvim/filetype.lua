-- Headless end-to-end test for `.codemmet` filetype support.
-- Run from the repo root:
--   cargo build -p codemmet && nvim --headless --clean -l tests/nvim/filetype.lua

local repo = vim.uv.cwd()
vim.opt.runtimepath:prepend(repo)
vim.o.hidden = true -- scenarios switch buffers without writing them

local binary = repo .. "/target/debug/codemmet"

---Print a failure (plus buffer contents, if given) and exit non-zero.
---@param msg string
---@param bufnr? integer
local function fail(msg, bufnr)
  io.write("FILETYPE FAIL: " .. msg .. "\n")
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

-- The plugin file registers the filetype detection and the user commands.
vim.cmd("runtime! plugin/codemmet.lua")

-- Capture notifications: the render failure path reports through vim.notify.
local notifications = {}
vim.notify = function(msg)
  table.insert(notifications, tostring(msg))
end

---The most recent notification containing `text`, if any.
---@param text string
---@return string|nil
local function notified(text)
  for i = #notifications, 1, -1 do
    if notifications[i]:find(text, 1, true) then
      return notifications[i]
    end
  end
  return nil
end

require("codemmet").setup({
  cmd = { binary, "lsp" },
  init_options = { snippets_dir = "/tmp/codemmet-filetype-nonexistent" },
})

-- Scenario 1: filetype detection fires for *.codemmet.
if vim.filetype.match({ filename = "main.rs.codemmet" }) ~= "codemmet" then
  fail("detection: main.rs.codemmet did not match the codemmet filetype")
end
if #vim.api.nvim_get_runtime_file("syntax/codemmet.vim", false) == 0 then
  fail("detection: syntax/codemmet.vim is not on the runtimepath")
end
io.write("filetype: detection scenario passed\n")

---Create a named buffer with the given lines, without touching its filetype.
---@param name string full path to name the buffer
---@param lines string[]
---@return integer bufnr
local function named_buf(name, lines)
  local bufnr = vim.api.nvim_create_buf(true, false)
  vim.api.nvim_buf_set_name(bufnr, name)
  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
  return bufnr
end

---Create a codemmet buffer and make it current; setting the filetype fires
---the FileType autocmd, starting/attaching the server.
---@param name string full path to name the buffer
---@param lines string[]
---@return integer bufnr
local function codemmet_buf(name, lines)
  local bufnr = named_buf(name, lines)
  vim.api.nvim_set_current_buf(bufnr)
  vim.bo[bufnr].filetype = "codemmet"
  return bufnr
end

-- Scenario 2: inner-language resolution (the CLI's ladder, editor-side).
local filemode = require("codemmet.filemode")
local by_ext = named_buf(vim.fn.tempname() .. "-ext.rs.codemmet", { "fn:main" })
if filemode.language_for(by_ext) ~= "rust" then
  fail("language: inner extension rs did not resolve to rust", by_ext)
end
local by_directive = named_buf(vim.fn.tempname() .. "-dir.rs.codemmet", {
  "# a comment first",
  "#lang go",
  "fn:main",
})
if filemode.language_for(by_directive) ~= "go" then
  fail("language: a header #lang directive did not win over the extension", by_directive)
end
local late_directive = named_buf(vim.fn.tempname() .. "-late.rs.codemmet", {
  "fn:main",
  "#lang go",
})
if filemode.language_for(late_directive) ~= "rust" then
  fail("language: a #lang after the first abbreviation must not count", late_directive)
end
local no_lang = named_buf(vim.fn.tempname() .. "-none.codemmet", { "fn:main" })
if filemode.language_for(no_lang) ~= nil then
  fail("language: a bare NAME.codemmet without #lang should resolve to nil", no_lang)
end
io.write("filetype: inner-language scenario passed\n")

-- Scenario 3: the server attaches to a .codemmet buffer with the INNER
-- languageId — proven end to end by expanding a rust abbreviation in it.
local expand_buf = codemmet_buf(vim.fn.tempname() .. "-expand.rs.codemmet", { "fori10" })
local attached = vim.wait(5000, function()
  local client = vim.lsp.get_clients({ name = "codemmet", bufnr = expand_buf })[1]
  return client ~= nil and client.initialized == true
end, 50)
if not attached then
  fail("attach: codemmet client did not attach to the .codemmet buffer", expand_buf)
end
vim.api.nvim_win_set_cursor(0, { 1, 5 }) -- normal-mode cursor ON the trailing "0"
require("codemmet").expand()
local expanded = vim.wait(5000, function()
  return vim.api.nvim_buf_get_lines(expand_buf, 0, 1, false)[1] == "for i in 0..10 {"
end, 50)
if not expanded then
  fail("attach: expanding fori10 in a .codemmet buffer never produced rust", expand_buf)
end
io.write("filetype: inner-languageId attach scenario passed\n")

-- Scenario 4: :CodemmetRenderTo writes the sidecar (buffer name minus
-- `.codemmet`) with the rendered content.
local render_src = vim.fn.tempname() .. "-render.rs.codemmet"
local render_buf = codemmet_buf(render_src, { "# scaffold", "", "fn:main" })
vim.cmd("CodemmetRenderTo")
local sidecar = render_src:gsub("%.codemmet$", "")
local rendered = vim.wait(5000, function()
  return vim.uv.fs_stat(sidecar) ~= nil
end, 50)
if not rendered then
  fail("render: sidecar " .. sidecar .. " was never written", render_buf)
end
local sidecar_text = table.concat(vim.fn.readfile(sidecar), "\n")
if sidecar_text ~= "fn main() {\n\n}" then
  fail("render: unexpected sidecar contents:\n" .. sidecar_text, render_buf)
end

-- An explicit target argument overrides the sidecar default.
local custom = vim.fn.tempname() .. "-custom.rs"
vim.cmd("CodemmetRenderTo " .. vim.fn.fnameescape(custom))
local custom_rendered = vim.wait(5000, function()
  return vim.uv.fs_stat(custom) ~= nil
end, 50)
if not custom_rendered then
  fail("render: explicit target " .. custom .. " was never written", render_buf)
end
io.write("filetype: render-to scenario passed\n")

-- Scenario 5: a failing render writes NOTHING — an existing target is left
-- untouched, a missing one stays absent — and notifies with the CLI's
-- `<stdin>:LINE:` diagnostic.
local bad_src = vim.fn.tempname() .. "-bad.rs.codemmet"
local bad_sidecar = bad_src:gsub("%.codemmet$", "")
vim.fn.writefile({ "SENTINEL" }, bad_sidecar)
local bad_buf = codemmet_buf(bad_src, { "zzznotatrigger" })
vim.cmd("CodemmetRenderTo")
local failed = vim.wait(5000, function()
  return notified("<stdin>:1:") ~= nil
end, 50)
if not failed then
  fail("failure: no <stdin>:1: diagnostic was notified", bad_buf)
end
if not notified("unknown trigger") then
  fail("failure: the notification lost the CLI's error message", bad_buf)
end
if table.concat(vim.fn.readfile(bad_sidecar), "\n") ~= "SENTINEL" then
  fail("failure: the existing target was overwritten on a failed render", bad_buf)
end
local absent = vim.fn.tempname() .. "-absent.rs"
vim.cmd("CodemmetRenderTo " .. vim.fn.fnameescape(absent))
vim.wait(1000, function()
  return notifications[#notifications] ~= nil and vim.uv.fs_stat(absent) ~= nil
end, 50)
if vim.uv.fs_stat(absent) ~= nil then
  fail("failure: a failed render must not create the target file", bad_buf)
end
io.write("filetype: failure atomicity scenario passed\n")

-- Scenario 6: render_on_save. Off by default: writing a .codemmet file
-- renders nothing. Re-setup with render_on_save = true: BufWritePost
-- renders the sidecar.
local off_src = vim.fn.tempname() .. "-off.rs.codemmet"
vim.cmd("edit " .. vim.fn.fnameescape(off_src))
vim.api.nvim_buf_set_lines(0, 0, -1, false, { "fn:main" })
vim.cmd("silent write")
vim.wait(500, function()
  return false
end, 50) -- give a (wrongly) registered autocmd time to fire
local off_sidecar = off_src:gsub("%.codemmet$", "")
if vim.uv.fs_stat(off_sidecar) ~= nil then
  fail("render_on_save: default-off still rendered the sidecar on write")
end

require("codemmet").setup({
  cmd = { binary, "lsp" },
  init_options = { snippets_dir = "/tmp/codemmet-filetype-nonexistent" },
  render_on_save = true,
})
local on_src = vim.fn.tempname() .. "-on.rs.codemmet"
vim.cmd("edit " .. vim.fn.fnameescape(on_src))
vim.api.nvim_buf_set_lines(0, 0, -1, false, { "fn:main" })
vim.cmd("silent write")
local on_sidecar = on_src:gsub("%.codemmet$", "")
local saved = vim.wait(5000, function()
  return vim.uv.fs_stat(on_sidecar) ~= nil
end, 50)
if not saved then
  fail("render_on_save: writing the buffer never rendered the sidecar")
end
if table.concat(vim.fn.readfile(on_sidecar), "\n") ~= "fn main() {\n\n}" then
  fail("render_on_save: unexpected sidecar contents")
end
io.write("filetype: render_on_save scenario passed\n")

io.write("FILETYPE OK\n")
os.exit(0)
