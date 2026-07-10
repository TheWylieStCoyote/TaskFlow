-- Headless end-to-end test for dot-repeat and operator support in the
-- codemmet Neovim plugin. Run from the repo root:
--   cargo build -p codemmet && nvim --headless --clean -l tests/nvim/dotrepeat.lua

local repo = vim.uv.cwd()
vim.opt.runtimepath:prepend(repo)

local binary = repo .. "/target/debug/codemmet"

---Print a failure (plus buffer contents, if given) and exit non-zero.
---@param msg string
---@param bufnr? integer
local function fail(msg, bufnr)
  io.write("DOTREPEAT FAIL: " .. msg .. "\n")
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
  init_options = { snippets_dir = "/tmp/codemmet-dotrepeat-nonexistent" },
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

---Feed keys (termcodes replaced) and process them synchronously.
---@param keys string
local function feed(keys)
  vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes(keys, true, false, true), "x", false)
end

---Leave any active snippet session / insert mode, the way a user would (<Esc>).
---Must not clobber the `.` register holding the last g@ operator.
local function reset_state()
  feed("<Esc>")
  if vim.snippet.active() then
    vim.snippet.stop()
  end
  vim.cmd("stopinsert")
end

-- Controllable vim.ui.input stub: `answer` is returned on the next prompt;
-- when `forbidden` is set, any prompt fails the test (dot-repeat must never prompt).
local input_stub = { answer = nil, forbidden = nil, calls = 0 }
---@diagnostic disable-next-line: duplicate-set-field
vim.ui.input = function(_, on_confirm)
  input_stub.calls = input_stub.calls + 1
  if input_stub.forbidden then
    fail("vim.ui.input was called during a `.` repeat (" .. input_stub.forbidden .. ")")
  end
  on_confirm(input_stub.answer)
end

-- Scenario 1: wrap as an operator. `<leader>cw ip` (mapleader is unset in this
-- clean session, so the key is `\cw`) wraps the first paragraph, prompting for
-- the abbreviation once. Then `.` on the second paragraph repeats the wrap with
-- the same abbreviation over the new motion range — and must not prompt.
local wrap_buf = rust_buf("wrap-operator", {
  'println!("a");',
  'println!("b");',
  "",
  'println!("c");',
  'println!("d");',
})

input_stub.answer = "fori10"
vim.api.nvim_win_set_cursor(0, { 1, 0 })
feed("\\cwip")

local first_wrap = {
  "for i in 0..10 {",
  '    println!("a");',
  '    println!("b");',
  "}",
  "",
  'println!("c");',
  'println!("d");',
}
local wrapped = vim.wait(5000, function()
  return vim.deep_equal(vim.api.nvim_buf_get_lines(wrap_buf, 0, -1, false), first_wrap)
end, 50)
if not wrapped then
  fail("wrap-operator: `\\cwip` did not wrap the first paragraph", wrap_buf)
end
if input_stub.calls ~= 1 then
  fail("wrap-operator: expected exactly one abbreviation prompt, got " .. input_stub.calls)
end
io.write("dotrepeat: wrap-operator scenario passed\n")

reset_state()

-- `.` on the second paragraph: same abbreviation, new motion range, no prompt.
input_stub.forbidden = "wrap repeat"
vim.api.nvim_win_set_cursor(0, { 6, 0 })
feed(".")

local second_wrap = {
  "for i in 0..10 {",
  '    println!("a");',
  '    println!("b");',
  "}",
  "",
  "for i in 0..10 {",
  '    println!("c");',
  '    println!("d");',
  "}",
}
local repeated = vim.wait(5000, function()
  return vim.deep_equal(vim.api.nvim_buf_get_lines(wrap_buf, 0, -1, false), second_wrap)
end, 50)
if not repeated then
  fail("wrap-repeat: `.` did not re-apply the wrap over the second paragraph", wrap_buf)
end
io.write("dotrepeat: wrap-repeat scenario passed\n")

reset_state()
input_stub.forbidden = nil

-- Scenario 2: visual-mode wrap still works as before (prompts, wraps the selection).
local visual_buf = rust_buf("visual-wrap", { 'println!("x");' })
input_stub.answer = "fori10"
vim.api.nvim_win_set_cursor(0, { 1, 0 })
feed("V\\cw")

local visual_wrapped = vim.wait(5000, function()
  local lines = vim.api.nvim_buf_get_lines(visual_buf, 0, -1, false)
  return lines[1] == "for i in 0..10 {" and (lines[2] or ""):match('^%s+println!%("x"%);') ~= nil
end, 50)
if not visual_wrapped then
  fail("visual-wrap: the visual-mode keymap no longer wraps the selection", visual_buf)
end
io.write("dotrepeat: visual-wrap scenario passed\n")

reset_state()

-- Scenario 3: dot-repeatable expand. <A-e> in normal mode expands the
-- abbreviation before the cursor; `.` elsewhere re-inserts the same
-- abbreviation at the cursor and expands it there.
local expand_buf = rust_buf("expand-repeat", { "fori10", "", "// tail" })
vim.api.nvim_win_set_cursor(0, { 1, 5 }) -- normal-mode cursor ON the trailing "0"
feed("<A-e>")

local expand_first = {
  "for i in 0..10 {",
  "    ",
  "}",
  "",
  "// tail",
}
local expanded = vim.wait(5000, function()
  return vim.deep_equal(vim.api.nvim_buf_get_lines(expand_buf, 0, -1, false), expand_first)
end, 50)
if not expanded then
  fail("expand: <A-e> did not expand fori10", expand_buf)
end
io.write("dotrepeat: expand scenario passed\n")

reset_state()

-- `.` on the empty line 4 repeats the expansion there.
input_stub.forbidden = "expand repeat"
vim.api.nvim_win_set_cursor(0, { 4, 0 })
feed(".")

local expand_second = {
  "for i in 0..10 {",
  "    ",
  "}",
  "for i in 0..10 {",
  "    ",
  "}",
  "// tail",
}
local expand_repeated = vim.wait(5000, function()
  return vim.deep_equal(vim.api.nvim_buf_get_lines(expand_buf, 0, -1, false), expand_second)
end, 50)
if not expand_repeated then
  fail("expand-repeat: `.` did not repeat the expansion on the empty line", expand_buf)
end
io.write("dotrepeat: expand-repeat scenario passed\n")

io.write("DOTREPEAT OK\n")
os.exit(0)
