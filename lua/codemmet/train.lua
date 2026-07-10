---Open the interactive syntax trainer (`codemmet train`) in a terminal split.
---
---The trainer is a full-screen ratatui TUI, so it runs as a real `:terminal`
---job (`vim.fn.termopen`) rather than through `vim.system`. The buffer's
---filetype picks the trained language when the plugin supports it, and any
---extra arguments are appended to the command line verbatim.
local lsp = require("codemmet.lsp")

local M = {}

---@class codemmet.TrainOpts
---@field args? string[] extra arguments appended verbatim to `codemmet train`
---@field cmd? string[] configured server command; binary fallback when no client is attached
---@field filetypes? string[] filetypes the plugin supports, used for the `--lang` mapping

---Resolve the codemmet binary: the attached client's command (the same binary
---that serves this buffer), else the configured `cmd`, else `codemmet` on PATH.
---@param bufnr integer
---@param cmd? string[]
---@return string
local function resolve_binary(bufnr, cmd)
  local client = lsp.get_client(bufnr)
  local binary = client and client.config.cmd and client.config.cmd[1]
  if type(binary) == "string" then
    return binary
  end
  if type(cmd) == "table" and type(cmd[1]) == "string" then
    return cmd[1]
  end
  return "codemmet"
end

---The `--lang` value for a buffer: its filetype, when the plugin supports it.
---Neovim's filetype doubles as the LSP `languageId` the server attaches with,
---and the trainer's `--lang` accepts ids, extensions, or LSP language ids —
---so the filetype passes through unchanged (`javascriptreact` included).
---Unknown filetypes return nil, leaving the trainer's own default language.
---@param bufnr integer
---@param filetypes? string[]
---@return string|nil
local function language_for(bufnr, filetypes)
  local filetype = vim.bo[bufnr].filetype
  if filetype ~= "" and vim.tbl_contains(filetypes or {}, filetype) then
    return filetype
  end
  return nil
end

---The working directory to run the trainer in, so it picks up project-local
---snippet packs relative to the file (same rule as `codemmet.capture`).
---@param bufnr integer
---@return string
local function buffer_cwd(bufnr)
  local name = vim.api.nvim_buf_get_name(bufnr)
  if name == "" then
    return vim.uv.cwd()
  end
  return vim.fs.dirname(name)
end

---Open `codemmet train` in a botright terminal split and enter terminal mode.
---Notifies (rather than opening a dead split) when the binary is missing or
---was built without the `train` cargo feature.
---@param opts? codemmet.TrainOpts
function M.train(opts)
  opts = opts or {}
  local bufnr = vim.api.nvim_get_current_buf()
  local binary = resolve_binary(bufnr, opts.cmd)

  if vim.fn.executable(binary) ~= 1 then
    vim.notify("codemmet: `" .. binary .. "` is not executable", vim.log.levels.WARN)
    return
  end

  -- `train` is a cargo feature; a build without it has no such subcommand
  -- (clap exits 2 with "unrecognized subcommand"). Probe with `--help` —
  -- cheap and non-interactive — so we can notify instead of opening a split
  -- that dies immediately.
  local probe = vim.system({ binary, "train", "--help" }, { text = true }):wait()
  if probe.code ~= 0 then
    local stderr = vim.trim(probe.stderr or "")
    local msg
    if stderr:find("unrecognized subcommand", 1, true) then
      msg = "`" .. binary .. "` was built without the `train` feature"
    elseif stderr ~= "" then
      msg = stderr
    else
      msg = "`" .. binary .. " train --help` exited " .. probe.code
    end
    vim.notify("codemmet: " .. msg, vim.log.levels.WARN)
    return
  end

  local cmd = { binary, "train" }
  local lang = language_for(bufnr, opts.filetypes)
  if lang then
    vim.list_extend(cmd, { "--lang", lang })
  end
  vim.list_extend(cmd, opts.args or {})

  vim.cmd("botright new") -- a fresh empty buffer, as termopen requires
  local job = vim.fn.termopen(cmd, { cwd = buffer_cwd(bufnr) })
  if job <= 0 then
    vim.notify("codemmet: failed to start `" .. binary .. " train`", vim.log.levels.WARN)
    vim.api.nvim_win_close(0, true)
    return
  end
  vim.cmd("startinsert") -- terminal mode, so keys reach the TUI
end

return M
