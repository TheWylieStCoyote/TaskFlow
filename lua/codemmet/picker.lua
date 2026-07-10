---Browse the trigger vocabulary for the current buffer's language and expand
---the chosen trigger at the cursor.
---
---The vocabulary's single source of truth is the CLI (`codemmet list --json`),
---so — like capture.lua — this module shells out to the same binary the LSP
---client was started with, then presents the entries through `vim.ui.select`
---(Telescope, fzf-lua, and snacks.nvim all take that UI over automatically).
---Accepting an entry inserts the trigger at the cursor and immediately runs
---the ordinary expand flow, so the user lands in the snippet's tabstops; a
---trigger with required params still works because expand handles bare
---triggers (unfilled params become tabstops).
local lsp = require("codemmet.lsp")

local M = {}

---@class codemmet.ListEntry
---@field trigger string
---@field description string

---The working directory to run the CLI in, so it resolves project-local packs
---relative to the file. Falls back to the editor's cwd for unsaved buffers.
---@param bufnr integer
---@return string
local function buffer_cwd(bufnr)
  local name = vim.api.nvim_buf_get_name(bufnr)
  if name == "" then
    return vim.uv.cwd()
  end
  return vim.fs.dirname(name)
end

---Parse `codemmet list --json` output.
---@param stdout string
---@return codemmet.ListEntry[]|nil entries, string|nil err
function M.parse(stdout)
  local ok, decoded = pcall(vim.json.decode, stdout)
  if not ok or not vim.islist(decoded) then
    return nil, "unexpected `codemmet list --json` output"
  end
  for _, entry in ipairs(decoded) do
    if type(entry) ~= "table" or type(entry.trigger) ~= "string" then
      return nil, "unexpected `codemmet list --json` entry"
    end
  end
  return decoded, nil
end

---Insert `trigger` at the 0-based byte position (row, col) and expand it via
---the server, landing the cursor in the snippet's tabstops.
---@param bufnr integer
---@param row integer 0-based row captured when the picker opened
---@param col integer 0-based byte column captured when the picker opened
---@param trigger string
local function insert_and_expand(bufnr, row, col, trigger)
  if not vim.api.nvim_buf_is_valid(bufnr) then
    return
  end
  if vim.api.nvim_get_current_buf() ~= bufnr then
    vim.api.nvim_set_current_buf(bufnr)
  end
  -- The buffer may have changed while the picker was open; clamp the saved
  -- position so the insertion stays in bounds.
  row = math.min(row, vim.api.nvim_buf_line_count(bufnr) - 1)
  local line = vim.api.nvim_buf_get_lines(bufnr, row, row + 1, false)[1] or ""
  col = math.min(col, #line)

  vim.api.nvim_buf_set_text(bufnr, row, col, row, col, { trigger })
  -- Place the normal-mode cursor ON the trigger's last character — exactly
  -- where it would sit after typing the trigger and leaving insert mode,
  -- which is the position lsp.expand() expects.
  vim.api.nvim_win_set_cursor(0, { row + 1, col + #trigger - 1 })
  lsp.expand()
end

---Open a `vim.ui.select` picker over every trigger registered for the current
---buffer's language and expand the accepted trigger at the cursor.
function M.list()
  local bufnr = vim.api.nvim_get_current_buf()
  local client = lsp.get_client(bufnr)
  if not client then
    vim.notify("codemmet: no codemmet client attached to this buffer", vim.log.levels.WARN)
    return
  end
  local binary = client.config.cmd and client.config.cmd[1]
  if type(binary) ~= "string" then
    vim.notify("codemmet: cannot locate the codemmet binary to list triggers", vim.log.levels.WARN)
    return
  end

  local lang = vim.bo[bufnr].filetype
  local row, col = unpack(vim.api.nvim_win_get_cursor(0))
  row = row - 1

  local cmd = { binary, "list", "--lang", lang, "--json" }
  vim.system(cmd, { cwd = buffer_cwd(bufnr), text = true }, function(obj)
    vim.schedule(function()
      if obj.code ~= 0 then
        local msg = (obj.stderr ~= "" and obj.stderr) or ("list exited " .. obj.code)
        vim.notify("codemmet: " .. vim.trim(msg), vim.log.levels.WARN)
        return
      end
      local entries, err = M.parse(obj.stdout)
      if not entries then
        vim.notify("codemmet: " .. err, vim.log.levels.WARN)
        return
      end
      if #entries == 0 then
        vim.notify("codemmet: no triggers registered for " .. lang, vim.log.levels.INFO)
        return
      end

      vim.ui.select(entries, {
        prompt = "codemmet triggers (" .. lang .. ")",
        kind = "codemmet_trigger",
        ---@param entry codemmet.ListEntry
        format_item = function(entry)
          return string.format("%-12s %s", entry.trigger, entry.description or "")
        end,
      }, function(entry)
        if not entry then
          return -- cancelled
        end
        insert_and_expand(bufnr, row, col, entry.trigger)
      end)
    end)
  end)
end

return M
