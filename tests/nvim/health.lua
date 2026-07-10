-- Headless test for `:checkhealth codemmet`.
-- Run from the repo root:
--   cargo build -p codemmet && nvim --headless --clean -l tests/nvim/health.lua

local repo = vim.uv.cwd()
vim.opt.runtimepath:prepend(repo)

local binary = repo .. "/target/debug/codemmet"

---Print a failure (plus the report, if given) and exit non-zero.
---@param msg string
---@param report? string
local function fail(msg, report)
  io.write("HEALTH FAIL: " .. msg .. "\n")
  if report then
    io.write("--- health report ---\n" .. report .. "\n---------------------\n")
  end
  os.exit(1)
end

if vim.fn.executable(binary) ~= 1 then
  fail(binary .. " is not built; run `cargo build -p codemmet` first")
end

---Run `:checkhealth codemmet` and return the rendered report text.
---@return string
local function report()
  vim.cmd("checkhealth codemmet")
  local text = table.concat(vim.api.nvim_buf_get_lines(0, 0, -1, false), "\n")
  vim.cmd("bwipeout!") -- drop the health buffer so the next run starts fresh
  return text
end

-- Scenario 1: a nonexistent binary path must produce an ERROR (and no OK
-- binary line). Runs first so no codemmet client exists yet — the health
-- check falls back to the configured cmd.
require("codemmet").setup({ cmd = { "/nonexistent/codemmet-health-test", "lsp" } })

local bad = report()
if not bad:find("ERROR", 1, true) then
  fail("bad binary: expected an ERROR line", bad)
end
if not bad:find("codemmet binary not found", 1, true) then
  fail("bad binary: expected the 'binary not found' message", bad)
end
if not bad:find("skipping the version", 1, true) then
  fail("bad binary: dependent checks should be skipped", bad)
end
if bad:find("binary found", 1, true) then
  fail("bad binary: unexpected OK binary line", bad)
end
io.write("health: bad-binary scenario passed\n")

-- Scenario 2: happy path with the built binary and an attached client.
require("codemmet").setup({
  cmd = { binary, "lsp" },
  init_options = { snippets_dir = "/tmp/codemmet-health-nonexistent" },
})

local bufnr = vim.api.nvim_create_buf(true, false)
vim.api.nvim_buf_set_name(bufnr, vim.fn.tempname() .. "-health.rs")
vim.api.nvim_set_current_buf(bufnr)
vim.bo[bufnr].filetype = "rust" -- fires the FileType autocmd, starting the server
local attached = vim.wait(5000, function()
  local client = vim.lsp.get_clients({ name = "codemmet", bufnr = bufnr })[1]
  return client ~= nil and client.initialized == true
end, 50)
if not attached then
  fail("codemmet LSP client did not attach to the rust buffer")
end

local good = report()
for _, expected in ipairs({
  "OK binary found",
  "OK `codemmet " .. "0", -- a parsed `codemmet 0.x.y` version line
  "OK tree-sitter context support is compiled in",
  "OK `codemmet check`: ok",
  "OK setup() called; filetypes: rust",
  "is running, attached to",
}) do
  if not good:find(expected, 1, true) then
    fail("happy path: report is missing '" .. expected .. "'", good)
  end
end
if good:find("ERROR", 1, true) then
  fail("happy path: report contains an ERROR line", good)
end
io.write("health: happy-path scenario passed\n")

io.write("HEALTH OK\n")
os.exit(0)
