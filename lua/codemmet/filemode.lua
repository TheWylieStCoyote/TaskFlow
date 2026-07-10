---File-mode support for `.codemmet` source buffers: inner-language
---resolution (mirroring the CLI's `#lang` > filename-inference ladder) and
---rendering a buffer to a code file via the `codemmet render` CLI.
local M = {}

---Inner file extension → LSP `languageId`, mirroring the extension table of
---the CLI's language profiles (`crates/codemmet-core/src/lang/profile.rs`).
---React extensions use the same `languageId` a real `.jsx`/`.tsx` buffer
---would attach with; the server maps either spelling to the same profile.
M.extensions = {
  rs = "rust",
  py = "python",
  c = "c",
  h = "c",
  cpp = "cpp",
  cc = "cpp",
  cxx = "cpp",
  hpp = "cpp",
  hh = "cpp",
  hxx = "cpp",
  js = "javascript",
  mjs = "javascript",
  cjs = "javascript",
  jsx = "javascriptreact",
  ts = "typescript",
  mts = "typescript",
  cts = "typescript",
  tsx = "typescriptreact",
  go = "go",
}

---How many leading lines to scan for a `#lang` directive. A legal `#lang`
---must precede the first abbreviation, so a short header scan suffices.
local HEADER_SCAN_LINES = 50

---Resolve the inner language of a `.codemmet` buffer: a `#lang` directive in
---the header wins; otherwise the inner extension of a `NAME.EXT.codemmet`
---buffer name is mapped through `M.extensions`. The returned id is anything
---the server's language ladder accepts (id, extension, or LSP languageId).
---@param bufnr integer
---@return string|nil language id, or nil when none resolves
function M.language_for(bufnr)
  for _, line in ipairs(vim.api.nvim_buf_get_lines(bufnr, 0, HEADER_SCAN_LINES, false)) do
    local lang = line:match("^%s*#lang%s+(%S+)")
    if lang then
      return lang
    end
    if not (line:match("^%s*$") or line:match("^%s*#")) then
      break -- the first abbreviation line: a legal `#lang` cannot follow it
    end
  end
  local name = vim.api.nvim_buf_get_name(bufnr)
  local inner = name:match("^(.*)%.codemmet$")
  local ext = inner and inner:match("%.(%w+)$")
  return ext and M.extensions[ext] or nil
end

---The `cmd` configured through `setup()`, the fallback for locating the
---binary when no codemmet client is attached to the buffer.
local configured_cmd = { "codemmet", "lsp" }

---Remember the `setup()` command.
---@param cmd string[]
function M.set_cmd(cmd)
  configured_cmd = cmd
end

---The codemmet binary to drive for a buffer: the attached client's `cmd[1]`
---(the capture.lua pattern), falling back to the `setup()` cmd.
---@param bufnr integer
---@return string|nil
local function binary_for(bufnr)
  local client = require("codemmet.lsp").get_client(bufnr)
  local cmd = (client and client.config.cmd) or configured_cmd
  local binary = cmd and cmd[1]
  if type(binary) ~= "string" then
    return nil
  end
  return binary
end

---The working directory to run the CLI in, so it resolves project snippet
---packs relative to the file. Falls back to the editor's cwd for unsaved
---buffers.
---@param bufnr integer
---@return string
local function buffer_cwd(bufnr)
  local name = vim.api.nvim_buf_get_name(bufnr)
  if name == "" then
    return vim.uv.cwd()
  end
  return vim.fs.dirname(name)
end

---@class codemmet.RenderToOpts
---@field file? string target path; defaults to the sidecar name (the buffer name minus `.codemmet`)
---@field bufnr? integer buffer to render (default: the current buffer)

---Render the buffer through `codemmet render -` and write the result to the
---target file. Atomic: the CLI emits nothing on a failed render, so on any
---error the target is not touched and the CLI's `<stdin>:LINE:` diagnostic
---is shown instead.
---@param opts? codemmet.RenderToOpts
function M.render_to(opts)
  opts = opts or {}
  local bufnr = opts.bufnr or vim.api.nvim_get_current_buf()
  local name = vim.api.nvim_buf_get_name(bufnr)
  local target = opts.file
  if not target or target == "" then
    target = name:match("^(.*)%.codemmet$")
    if not target or target == "" then
      vim.notify(
        "codemmet: no render target: name the buffer `NAME.EXT.codemmet` or pass a file to :CodemmetRenderTo",
        vim.log.levels.WARN
      )
      return
    end
  end
  local binary = binary_for(bufnr)
  if not binary then
    vim.notify("codemmet: cannot locate the codemmet binary to render", vim.log.levels.WARN)
    return
  end

  local cmd = { binary, "render" }
  local lang = M.language_for(bufnr)
  if lang then
    vim.list_extend(cmd, { "--lang", lang })
  end
  table.insert(cmd, "-") -- read the source from stdin
  local text = table.concat(vim.api.nvim_buf_get_lines(bufnr, 0, -1, false), "\n") .. "\n"

  vim.system(cmd, { cwd = buffer_cwd(bufnr), stdin = text, text = true }, function(obj)
    vim.schedule(function()
      if obj.code ~= 0 then
        local msg = (obj.stderr and obj.stderr ~= "" and obj.stderr)
          or ("render exited " .. obj.code)
        vim.notify("codemmet: " .. vim.trim(msg), vim.log.levels.WARN)
        return
      end
      local file, err = io.open(target, "w")
      if not file then
        vim.notify(
          "codemmet: cannot write " .. target .. ": " .. tostring(err),
          vim.log.levels.WARN
        )
        return
      end
      file:write(obj.stdout)
      file:close()
      vim.notify("codemmet: rendered " .. vim.fn.fnamemodify(target, ":~:."), vim.log.levels.INFO)
    end)
  end)
end

return M
