---Render the buffer — or a line range — as a `.codemmet` source into code.
---
---This is file mode (`docs/cli.md`) in the editor: the region is fed to the
---`codemmet render` CLI on stdin and, on success, replaced by the rendered
---code. The CLI renders the whole input in memory and emits nothing on
---failure, so the buffer is never touched by a failed render; the
---diagnostic's `<stdin>:N:` line number is mapped back to a buffer line and
---the cursor jumps there.
---
---A whole-buffer render re-enters snippet mode by default: the CLI is asked
---for `--tabstops keep` (one LSP snippet document, tabstops renumbered
---across the whole file) and the result is `vim.snippet.expand()`ed into
---the emptied buffer, so `<Tab>` walks construct to construct from the
---first tabstop. Visual/range renders always take the plain-text path — a
---range sits inside existing code, where a buffer-wide snippet session
---would fight the surrounding text. Opt out via
---`setup({ render = { snippet_reentry = false } })`.
local lsp = require("codemmet.lsp")

local M = {}

---@class codemmet.RenderConfig
---@field snippet_reentry? boolean whole-buffer renders enter vim.snippet navigation (default true)

---The active render configuration (see `codemmet.SetupOpts`).
local config = { snippet_reentry = true }

---Apply the `render` section of the setup options.
---@param opts? codemmet.RenderConfig
function M.configure(opts)
  config = vim.tbl_extend("force", config, opts or {})
end

---@class codemmet.RenderOpts
---@field line1? integer 1-based first line of the region (e.g. a user command's range)
---@field line2? integer 1-based last line of the region

---Resolve the 1-based line span to render: explicit opts, the visual
---selection, or the whole buffer. Leaves visual mode if it was active.
---`whole` is true only for the implicit whole-buffer spelling — the one
---that may re-enter snippet mode.
---@param bufnr integer
---@param opts codemmet.RenderOpts
---@return integer line1, integer line2, boolean whole
local function resolve_lines(bufnr, opts)
  if opts.line1 and opts.line2 then
    return math.min(opts.line1, opts.line2), math.max(opts.line1, opts.line2), false
  end
  local mode = vim.api.nvim_get_mode().mode
  if mode:find("^[vV\22]") then
    local anchor = vim.fn.getpos("v")[2]
    local cursor = vim.fn.getpos(".")[2]
    local esc = vim.api.nvim_replace_termcodes("<Esc>", true, false, true)
    vim.api.nvim_feedkeys(esc, "n", false)
    return math.min(anchor, cursor), math.max(anchor, cursor), false
  end
  return 1, vim.api.nvim_buf_line_count(bufnr), true
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
---of the output (the plain-text path: visual/range renders, and whole-buffer
---renders with `snippet_reentry = false`).
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

---Replace the whole buffer with the keep-mode render, expanded as one
---snippet so tabstop navigation starts at the first construct.
---
---Atomicity is preserved end to end: `vim.snippet.expand` needs the cursor
---in the target buffer, so if the buffer is no longer current nothing is
---applied; and if expansion itself fails, the emptied buffer is restored to
---the saved abbreviation lines before reporting.
---@param bufnr integer
---@param stdout string
local function apply_snippet(bufnr, stdout)
  if vim.api.nvim_get_current_buf() ~= bufnr then
    vim.notify("codemmet: buffer is no longer active; render not applied", vim.log.levels.WARN)
    return
  end
  local body = stdout
  if body:sub(-1) == "\n" then
    body = body:sub(1, -2) -- the trailing newline is not an extra line
  end
  local saved = vim.api.nvim_buf_get_lines(bufnr, 0, -1, false)
  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, {})
  vim.api.nvim_win_set_cursor(0, { 1, 0 })
  local ok, err = pcall(vim.snippet.expand, body)
  if not ok then
    vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, saved)
    vim.notify("codemmet: cannot expand rendered snippet: " .. tostring(err), vim.log.levels.ERROR)
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

  local line1, line2, whole = resolve_lines(bufnr, opts)
  local lines = vim.api.nvim_buf_get_lines(bufnr, line1 - 1, line2, false)
  local source = table.concat(lines, "\n") .. "\n"
  local lang = vim.bo[bufnr].filetype
  local tick = vim.b[bufnr].changedtick
  local reenter = whole and config.snippet_reentry and vim.snippet ~= nil

  local cmd = { binary, "render", "--lang", lang, "--indent", indent_unit(bufnr), "-" }
  if reenter then
    -- One LSP snippet document, tabstops renumbered across the whole file.
    table.insert(cmd, 3, "--tabstops")
    table.insert(cmd, 4, "keep")
  end
  vim.system(cmd, { cwd = buffer_cwd(bufnr), stdin = source, text = true }, function(obj)
    vim.schedule(function()
      if not vim.api.nvim_buf_is_valid(bufnr) then
        return
      end
      if vim.b[bufnr].changedtick ~= tick then
        vim.notify("codemmet: buffer changed while rendering; not applied", vim.log.levels.WARN)
        return
      end
      if obj.code ~= 0 then
        report(bufnr, line1, obj.stderr, obj.code)
      elseif reenter and (obj.stdout or "") ~= "" then
        apply_snippet(bufnr, obj.stdout)
      else
        apply(bufnr, line1, line2, obj.stdout or "")
      end
    end)
  end)
end

return M
