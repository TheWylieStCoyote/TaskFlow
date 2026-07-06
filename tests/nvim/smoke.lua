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

require("codemmet").setup({
  cmd = { binary, "lsp" },
  init_options = { snippets_dir = "/tmp/codemmet-smoke-nonexistent" },
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

io.write("SMOKE OK\n")
os.exit(0)
