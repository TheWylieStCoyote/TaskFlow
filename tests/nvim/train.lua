-- Headless test for the :CodemmetTrain terminal-split wrapper.
-- Run from the repo root:
--   cargo build -p codemmet && nvim --headless --clean -l tests/nvim/train.lua
--
-- vim.fn.termopen is stubbed to capture the terminal job's argv instead of
-- spawning the TUI, so this asserts the command line (--lang mapping from the
-- filetype, verbatim passthrough args) and the graceful no-train-feature path.

local repo = vim.uv.cwd()
vim.opt.runtimepath:prepend(repo)

local binary = repo .. "/target/debug/codemmet"

---Print a failure and exit non-zero.
---@param msg string
local function fail(msg)
  io.write("TRAIN FAIL: " .. msg .. "\n")
  os.exit(1)
end

if vim.fn.executable(binary) ~= 1 then
  fail(binary .. " is not built; run `cargo build -p codemmet` first")
end

-- `--clean -l` does not source plugin/ files; load the user commands explicitly.
vim.cmd("runtime! plugin/codemmet.lua")

require("codemmet").setup({ cmd = { binary, "lsp" } })

-- Stub termopen: capture argv + opts, pretend the job started.
local captured
vim.fn.termopen = function(cmd, opts)
  captured = { cmd = cmd, opts = opts }
  return 42
end

---Create a buffer with the given filetype. For plugin-supported filetypes this
---fires the FileType autocmd and waits for the codemmet client to attach.
---@param filetype string
---@param wait_attach boolean
---@return integer bufnr
local function buf_with_filetype(filetype, wait_attach)
  local bufnr = vim.api.nvim_create_buf(true, false)
  vim.api.nvim_buf_set_name(bufnr, vim.fn.tempname() .. "-train." .. filetype)
  vim.api.nvim_set_current_buf(bufnr)
  vim.bo[bufnr].filetype = filetype
  if wait_attach then
    local attached = vim.wait(5000, function()
      local client = vim.lsp.get_clients({ name = "codemmet", bufnr = bufnr })[1]
      return client ~= nil and client.initialized == true
    end, 50)
    if not attached then
      fail("codemmet LSP client did not attach to the " .. filetype .. " buffer")
    end
  end
  return bufnr
end

---Run :CodemmetTrain {args} and return the argv the (stubbed) terminal job got.
---@param args string
---@return string[] argv, table opts
local function run_train(args)
  captured = nil
  local wins_before = #vim.api.nvim_list_wins()
  vim.cmd("CodemmetTrain" .. (args ~= "" and " " .. args or ""))
  if captured == nil then
    fail("CodemmetTrain " .. args .. ": termopen was never called")
  end
  if #vim.api.nvim_list_wins() ~= wins_before + 1 then
    fail("CodemmetTrain " .. args .. ": expected exactly one new split window")
  end
  return captured.cmd, captured.opts
end

-- Scenario 1: a supported filetype maps to --lang, and the binary comes from
-- the attached client's cmd[1].
local rust_buf = buf_with_filetype("rust", true)
local argv, opts = run_train("")
if not vim.deep_equal(argv, { binary, "train", "--lang", "rust" }) then
  fail("rust buffer: unexpected argv: " .. vim.inspect(argv))
end
-- The job runs in the buffer's directory, so project-local packs are found.
if opts.cwd ~= vim.fs.dirname(vim.api.nvim_buf_get_name(rust_buf)) then
  fail("rust buffer: expected cwd of the buffer's directory, got " .. tostring(opts.cwd))
end
io.write("train: --lang from filetype scenario passed\n")

-- Scenario 2: extra arguments are appended verbatim after the --lang flag.
vim.api.nvim_set_current_buf(rust_buf)
argv = run_train("--level 9 --mode drills")
if not vim.deep_equal(argv, { binary, "train", "--lang", "rust", "--level", "9", "--mode", "drills" }) then
  fail("passthrough: unexpected argv: " .. vim.inspect(argv))
end
io.write("train: passthrough args scenario passed\n")

-- Scenario 3: an unsupported filetype omits --lang (the trainer's own default
-- applies) and the binary falls back to the configured cmd.
buf_with_filetype("markdown", false)
argv = run_train("")
if not vim.deep_equal(argv, { binary, "train" }) then
  fail("markdown buffer: unexpected argv: " .. vim.inspect(argv))
end
io.write("train: unknown filetype fallback scenario passed\n")

-- Scenario 4: a binary built without the `train` cargo feature (probe exits 2
-- with clap's "unrecognized subcommand") notifies instead of opening a split.
local real_system = vim.system
local real_notify = vim.notify
vim.system = function()
  return {
    wait = function()
      return { code = 2, stderr = "error: unrecognized subcommand 'train'\n" }
    end,
  }
end
local notified
vim.notify = function(msg, level)
  notified = { msg = msg, level = level }
end
captured = nil
local wins_before = #vim.api.nvim_list_wins()
vim.cmd("CodemmetTrain")
vim.system = real_system
vim.notify = real_notify
if captured ~= nil then
  fail("no-train build: termopen should not have been called")
end
if #vim.api.nvim_list_wins() ~= wins_before then
  fail("no-train build: no split window should have been opened")
end
if not (notified and notified.msg:find("built without the `train` feature", 1, true)) then
  fail("no-train build: expected a feature notification, got " .. vim.inspect(notified))
end
io.write("train: missing train feature scenario passed\n")

io.write("TRAIN OK\n")
os.exit(0)
