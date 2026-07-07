---Capture a line-wise selection into a snippet pack.
---
---The codemmet LSP server is deliberately read-only, so the capture is done by
---invoking the `codemmet capture` CLI (the single source of truth for the
---generalization + pack-writing). After the CLI writes the pack, we ask the
---server to reload its snippet caches (a read-only re-read from disk) so the
---new trigger is expandable in the same session.
local lsp = require("codemmet.lsp")

local M = {}

---@class codemmet.CaptureOpts
---@field name? string trigger name; when given the vim.ui.input prompt is skipped
---@field description? string optional snippet description
---@field line1? integer 1-based first line of the region (e.g. a user command's range)
---@field line2? integer 1-based last line of the region

---Resolve the 1-based line span to capture: explicit opts, the visual
---selection, or the cursor line. Leaves visual mode if it was active.
---@param opts codemmet.CaptureOpts
---@return integer line1, integer line2
local function resolve_lines(opts)
  if opts.line1 and opts.line2 then
    return math.min(opts.line1, opts.line2), math.max(opts.line1, opts.line2)
  end
  local mode = vim.api.nvim_get_mode().mode
  if mode:find("^[vV\22]") then
    local anchor = vim.fn.getpos("v")[2]
    local cursor = vim.fn.getpos(".")[2]
    local esc = vim.api.nvim_replace_termcodes("<Esc>", true, false, true)
    vim.api.nvim_feedkeys(esc, "n", false)
    return math.min(anchor, cursor), math.max(anchor, cursor)
  end
  local row = vim.api.nvim_win_get_cursor(0)[1]
  return row, row
end

---The working directory to run the CLI in, so it resolves the project pack
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

---Ask the server to re-read its snippet packs (read-only), so a freshly
---captured trigger is usable without reopening the buffer.
---@param client vim.lsp.Client
---@param bufnr integer
local function reload_server(client, bufnr)
  lsp.exec(client, bufnr, "codemmet.reloadSnippets", {}, function(err)
    if err then
      vim.notify(
        "codemmet: captured, but snippet reload failed: " .. (err.message or vim.inspect(err)),
        vim.log.levels.WARN
      )
    end
  end)
end

---Capture the selected lines into the project pack under trigger `name`, by
---running `codemmet capture`. Prompts via `vim.ui.input` for the name unless
---`opts.name` is given.
---@param opts? codemmet.CaptureOpts
function M.capture(opts)
  opts = opts or {}
  local bufnr = vim.api.nvim_get_current_buf()
  local client = lsp.get_client(bufnr)
  if not client then
    vim.notify("codemmet: no codemmet client attached to this buffer", vim.log.levels.WARN)
    return
  end
  local binary = client.config.cmd and client.config.cmd[1]
  if type(binary) ~= "string" then
    vim.notify("codemmet: cannot locate the codemmet binary to capture", vim.log.levels.WARN)
    return
  end

  local line1, line2 = resolve_lines(opts)
  local lines = vim.api.nvim_buf_get_lines(bufnr, line1 - 1, line2, false)
  local code = table.concat(lines, "\n")
  local lang = vim.bo[bufnr].filetype
  local cwd = buffer_cwd(bufnr)

  ---@param name string
  local function send(name)
    local cmd = { binary, "capture", "--lang", lang, "--name", name }
    if opts.description and opts.description ~= "" then
      table.insert(cmd, "--description")
      table.insert(cmd, opts.description)
    end
    table.insert(cmd, "-") -- read the code from stdin
    vim.system(cmd, { cwd = cwd, stdin = code, text = true }, function(obj)
      vim.schedule(function()
        if obj.code ~= 0 then
          local msg = (obj.stderr ~= "" and obj.stderr) or ("capture exited " .. obj.code)
          vim.notify("codemmet: " .. vim.trim(msg), vim.log.levels.WARN)
          return
        end
        reload_server(client, bufnr)
        vim.notify("codemmet: " .. vim.trim(obj.stdout), vim.log.levels.INFO)
      end)
    end)
  end

  if opts.name and opts.name ~= "" then
    send(opts.name)
    return
  end

  vim.ui.input({ prompt = "codemmet capture as: " }, function(input)
    if input == nil or input == "" then
      return -- cancelled
    end
    send(input)
  end)
end

return M
