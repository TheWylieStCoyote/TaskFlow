---Render the buffer — or a line range — as a `.codemmet` source into code.
---
---This is file mode (`docs/cli.md`) in the editor: the region is fed to the
---`codemmet render` CLI on stdin and, on success, replaced by the rendered
---code. The CLI renders the whole input in memory and emits nothing on
---failure, so the buffer is never touched by a failed render; the
---diagnostic's `<stdin>:N:` line number is mapped back to a buffer line and
---the cursor jumps there.
local lsp = require("codemmet.lsp")

local M = {}

---@class codemmet.RenderOpts
---@field line1? integer 1-based first line of the region (e.g. a user command's range)
---@field line2? integer 1-based last line of the region

---Resolve the 1-based line span to render: explicit opts, the visual
---selection, or the whole buffer. Leaves visual mode if it was active.
---@param bufnr integer
---@param opts codemmet.RenderOpts
---@return integer line1, integer line2
local function resolve_lines(bufnr, opts)
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
  return 1, vim.api.nvim_buf_line_count(bufnr)
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

---The buffer's indent unit as a literal `--indent` value, so the rendered
---code matches the editor's settings rather than the language default.
---@param bufnr integer
---@return string
local function indent_unit(bufnr)
  local options = lsp.formatting_options(bufnr)
  if options.insert_spaces then
    return string.rep(" ", options.tab_size)
  end
  return "\t"
end

---Replace the region with the rendered code and put the cursor at the top
---of the output (v1; re-entering snippet mode at the first construct is a
---possible later refinement).
---@param bufnr integer
---@param line1 integer
---@param line2 integer
---@param stdout string
local function apply(bufnr, line1, line2, stdout)
  local rendered = vim.split(stdout, "\n", { plain = true })
  if rendered[#rendered] == "" then
    table.remove(rendered) -- the output's trailing newline is not an extra line
  end
  vim.api.nvim_buf_set_lines(bufnr, line1 - 1, line2, false, rendered)
  if vim.api.nvim_get_current_buf() == bufnr then
    local top = math.min(line1, vim.api.nvim_buf_line_count(bufnr))
    vim.api.nvim_win_set_cursor(0, { top, 0 })
  end
end

---Report a failed render: move the cursor to the offending buffer line (the
---CLI reports `<stdin>:N:` relative to the piped region) and notify.
---@param bufnr integer
---@param line1 integer
---@param stderr string|nil
---@param exit_code integer
local function report(bufnr, line1, stderr, exit_code)
  local msg = vim.trim(stderr or "")
  if msg == "" then
    msg = "render exited " .. exit_code
  end
  local source_line = msg:match("<stdin>:(%d+):")
  if source_line and vim.api.nvim_get_current_buf() == bufnr then
    local target = line1 - 1 + tonumber(source_line)
    target = math.min(target, vim.api.nvim_buf_line_count(bufnr))
    vim.api.nvim_win_set_cursor(0, { math.max(target, 1), 0 })
  end
  vim.notify("codemmet: " .. msg, vim.log.levels.ERROR)
end

---Render the selected lines (visual selection, `opts.line1..opts.line2`, or
---the whole buffer) as a `.codemmet` source and replace them with the code.
---@param opts? codemmet.RenderOpts
function M.render(opts)
  opts = opts or {}
  local bufnr = vim.api.nvim_get_current_buf()
  local client = lsp.get_client(bufnr)
  if not client then
    vim.notify("codemmet: no codemmet client attached to this buffer", vim.log.levels.WARN)
    return
  end
  local binary = client.config.cmd and client.config.cmd[1]
  if type(binary) ~= "string" then
    vim.notify("codemmet: cannot locate the codemmet binary to render", vim.log.levels.WARN)
    return
  end

  local line1, line2 = resolve_lines(bufnr, opts)
  local lines = vim.api.nvim_buf_get_lines(bufnr, line1 - 1, line2, false)
  local source = table.concat(lines, "\n") .. "\n"
  local lang = vim.bo[bufnr].filetype
  local tick = vim.b[bufnr].changedtick

  local cmd = { binary, "render", "--lang", lang, "--indent", indent_unit(bufnr), "-" }
  vim.system(cmd, { cwd = buffer_cwd(bufnr), stdin = source, text = true }, function(obj)
    vim.schedule(function()
      if not vim.api.nvim_buf_is_valid(bufnr) then
        return
      end
      if vim.b[bufnr].changedtick ~= tick then
        vim.notify("codemmet: buffer changed while rendering; not applied", vim.log.levels.WARN)
        return
      end
      if obj.code == 0 then
        apply(bufnr, line1, line2, obj.stdout or "")
      else
        report(bufnr, line1, obj.stderr, obj.code)
      end
    end)
  end)
end

return M
