---Expand a multi-file snippet across files.
---
---Like capture, this touches the filesystem, so it runs the `codemmet scaffold`
---CLI (the single source of truth for path resolution, rendering, and the
---create/append/inject actions) rather than the read-only LSP server. We ask
---for a JSON manifest with tabstops kept, write every non-primary file to disk,
---and drop the primary file's content in as a live `vim.snippet` session so its
---tabstops are interactive.
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

---@param path string
---@return string?
local function read_file(path)
  local fd = io.open(path, "r")
  if not fd then
    return nil
  end
  local content = fd:read("*a")
  fd:close()
  return content
end

---@param path string
---@param content string
local function write_file(path, content)
  vim.fn.mkdir(vim.fs.dirname(path), "p")
  local fd = assert(io.open(path, "w"))
  fd:write(content)
  fd:close()
end

---A Rust/tree-sitter byte `offset` into `text` as a 1-based row and 0-based
---byte column, for `nvim_win_set_cursor`.
---@param text string
---@param offset integer
---@return integer row, integer col
local function offset_to_pos(text, offset)
  offset = math.min(offset, #text)
  local prefix = text:sub(1, offset)
  local row, nl = 1, 0
  local i = prefix:find("\n", 1, true)
  while i do
    row, nl = row + 1, i
    i = prefix:find("\n", i + 1, true)
  end
  return row, offset - nl
end

---Write a non-primary manifest entry to disk (plain text — non-primary content
---is rendered without tabstops).
---@param entry table
local function apply_to_disk(entry)
  if entry.action == "append" then
    local existing = read_file(entry.path) or ""
    local block = "\n" .. entry.content
    if not block:match("\n$") then
      block = block .. "\n"
    end
    write_file(entry.path, existing .. block)
  elseif entry.action == "inject" then
    local existing = read_file(entry.path) or ""
    local off = math.min(entry.offset or #existing, #existing)
    write_file(entry.path, existing:sub(1, off) .. entry.content .. existing:sub(off + 1))
  else -- create
    local content = entry.content
    if not content:match("\n$") then
      content = content .. "\n"
    end
    write_file(entry.path, content)
  end
end

---Open the primary file and insert its content as a live snippet, so the
---tabstops (`$1`, `$0`) are interactive. Left unsaved for the user to fill.
---@param entry table
local function open_primary(entry)
  vim.fn.mkdir(vim.fs.dirname(entry.path), "p")
  vim.cmd.edit(vim.fn.fnameescape(entry.path))
  local bufnr = vim.api.nvim_get_current_buf()
  if entry.action == "append" then
    local last = vim.api.nvim_buf_line_count(bufnr)
    local tail = vim.api.nvim_buf_get_lines(bufnr, last - 1, last, false)[1] or ""
    vim.api.nvim_win_set_cursor(0, { last, #tail })
    vim.snippet.expand("\n" .. entry.content)
  elseif entry.action == "inject" then
    local existing = read_file(entry.path) or ""
    local row, col = offset_to_pos(existing, entry.offset or #existing)
    vim.api.nvim_win_set_cursor(0, { row, col })
    vim.snippet.expand(entry.content)
  else -- create: fresh buffer, expand at the top
    vim.api.nvim_win_set_cursor(0, { 1, 0 })
    vim.snippet.expand(entry.content)
  end
end

---Apply a decoded manifest: non-primary files to disk, the primary opened
---interactively (last, so it ends up focused).
---@param manifest table[]
local function apply(manifest)
  local primary
  local names = {}
  for _, entry in ipairs(manifest) do
    if entry.primary then
      primary = entry
    else
      apply_to_disk(entry)
    end
    table.insert(names, entry.action .. " " .. vim.fs.basename(entry.path))
  end
  if primary then
    open_primary(primary)
  end
  vim.notify("codemmet: " .. table.concat(names, ", "), vim.log.levels.INFO)
end

---Run `codemmet scaffold --json` for `invocation` in `bufnr` and apply it.
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
  vim.list_extend(cmd, { "--tabstops", "keep", "--json", invocation })
  vim.system(cmd, { cwd = root, text = true }, function(obj)
    vim.schedule(function()
      if obj.code ~= 0 then
        local msg = (obj.stderr ~= "" and obj.stderr) or ("scaffold exited " .. obj.code)
        vim.notify("codemmet: " .. vim.trim(msg), vim.log.levels.WARN)
        return
      end
      local ok, manifest = pcall(vim.json.decode, obj.stdout)
      if not ok or type(manifest) ~= "table" then
        vim.notify("codemmet: could not parse the scaffold manifest", vim.log.levels.WARN)
        return
      end
      apply(manifest)
    end)
  end)
end

---Expand a multi-file snippet: write its supporting files and open the primary
---file with a live snippet. Prompts for the invocation unless `opts.abbr` is
---given.
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
