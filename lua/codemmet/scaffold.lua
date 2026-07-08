---Expand a multi-file snippet across files.
---
---Like capture, this touches the filesystem, so it runs the `codemmet scaffold`
---CLI (the single source of truth for path resolution, rendering, and the
---create/append/inject actions) rather than the read-only LSP server. The CLI
---marks the primary file in its `--write` output; we open that afterwards.
local lsp = require("codemmet.lsp")

local M = {}

---@class codemmet.ScaffoldOpts
---@field abbr? string the invocation (trigger + args), e.g. "djfeature:blog";
---  when omitted a `vim.ui.input` prompt is shown

---The project root for `path` targets: the nearest ancestor with `.codemmet`
---or `.git`, else the file's directory, else the editor cwd.
---@param bufnr integer
---@return string
local function project_root(bufnr)
  local name = vim.api.nvim_buf_get_name(bufnr)
  if name == "" then
    return vim.uv.cwd()
  end
  return vim.fs.root(bufnr, { ".codemmet", ".git" }) or vim.fs.dirname(name)
end

---Open the primary file the CLI marked, if any (a line `<action> <path> (primary)`).
---@param stdout string
local function open_primary(stdout)
  local primary
  for line in vim.gsplit(stdout, "\n", { trimempty = true }) do
    local path = line:match("^%S+%s+(.-)%s+%(primary%)$")
    if path then
      primary = path
    end
  end
  if primary then
    vim.cmd.edit(vim.fn.fnameescape(primary))
  end
end

---Run `codemmet scaffold --write` for `invocation` in `bufnr`.
---@param binary string
---@param invocation string
---@param bufnr integer
local function run(binary, invocation, bufnr)
  local root = project_root(bufnr)
  local cmd = { binary, "scaffold", "--lang", vim.bo[bufnr].filetype, "--root", root }
  local file = vim.api.nvim_buf_get_name(bufnr)
  if file ~= "" then
    vim.list_extend(cmd, { "--from", file }) -- for `self`/sibling targets
  end
  vim.list_extend(cmd, { "--write", invocation })
  vim.system(cmd, { cwd = root, text = true }, function(obj)
    vim.schedule(function()
      if obj.code ~= 0 then
        local msg = (obj.stderr ~= "" and obj.stderr) or ("scaffold exited " .. obj.code)
        vim.notify("codemmet: " .. vim.trim(msg), vim.log.levels.WARN)
        return
      end
      open_primary(obj.stdout)
      vim.notify("codemmet: " .. vim.trim(obj.stdout), vim.log.levels.INFO)
    end)
  end)
end

---Expand a multi-file snippet: run `codemmet scaffold --write`, then open the
---primary file. Prompts for the invocation unless `opts.abbr` is given.
---@param opts? codemmet.ScaffoldOpts
function M.scaffold(opts)
  opts = opts or {}
  local bufnr = vim.api.nvim_get_current_buf()
  local client = lsp.get_client(bufnr)
  if not client then
    vim.notify("codemmet: no codemmet client attached to this buffer", vim.log.levels.WARN)
    return
  end
  local binary = client.config.cmd and client.config.cmd[1]
  if type(binary) ~= "string" then
    vim.notify("codemmet: cannot locate the codemmet binary to scaffold", vim.log.levels.WARN)
    return
  end

  if opts.abbr and opts.abbr ~= "" then
    run(binary, opts.abbr, bufnr)
    return
  end
  vim.ui.input({ prompt = "codemmet scaffold: " }, function(input)
    if input == nil or input == "" then
      return -- cancelled
    end
    run(binary, input, bufnr)
  end)
end

return M
