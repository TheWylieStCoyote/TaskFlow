---Wrap a line-wise selection in a codemmet abbreviation via the server's
---`codemmet.wrap` command.
local lsp = require("codemmet.lsp")

local M = {}

---Dot-repeat state for the wrap operator. `fresh` is set by the mapping entry
---point (M.operator) right before `g@`, so the operatorfunc can tell a brand-new
---invocation (prompt for the abbreviation) from a `.` repeat, which re-invokes
---the operatorfunc directly without going through the mapping (use the stored
---abbreviation, never prompt). This is the standard operatorfunc-repeat idiom.
local state = { fresh = false, abbreviation = nil }

---@class codemmet.WrapOpts
---@field abbreviation? string wrap abbreviation; when given the vim.ui.input prompt is skipped
---@field line1? integer 1-based first line of the region (e.g. a user command's range)
---@field line2? integer 1-based last line of the region

---Resolve the 1-based line span to wrap: explicit opts, the visual selection,
---or the cursor line. Leaves visual mode if it was active.
---@param opts codemmet.WrapOpts
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

---Wrap the selected lines (visual selection, `opts.line1..opts.line2`, or the cursor
---line) in an abbreviation. Prompts via `vim.ui.input` unless `opts.abbreviation` is given.
---@param opts? codemmet.WrapOpts
function M.wrap(opts)
  opts = opts or {}
  local bufnr = vim.api.nvim_get_current_buf()
  local client = lsp.get_client(bufnr)
  if not client then
    vim.notify("codemmet: no codemmet client attached to this buffer", vim.log.levels.WARN)
    return
  end

  local line1, line2 = resolve_lines(opts)

  -- Line-wise range: character 0 of the first line through end-of-line of the last line.
  local range = {
    start = { line = line1 - 1, character = 0 },
    ["end"] = {
      line = line2 - 1,
      character = lsp.line_end_character(bufnr, line2 - 1, client.offset_encoding),
    },
  }

  ---@param abbreviation string
  local function send(abbreviation)
    lsp.exec(client, bufnr, "codemmet.wrap", {
      {
        uri = vim.uri_from_bufnr(bufnr),
        range = range,
        abbreviation = abbreviation,
        formatting_options = lsp.formatting_options(bufnr),
      },
    }, function(err, result)
      if err then
        vim.notify("codemmet: " .. (err.message or vim.inspect(err)), vim.log.levels.WARN)
        return
      end
      if not result or not result.snippet then
        return
      end
      if vim.api.nvim_get_current_buf() ~= bufnr then
        return
      end
      lsp.apply_snippet(bufnr, result.range, result.snippet, client.offset_encoding)
    end)
  end

  if opts.abbreviation and opts.abbreviation ~= "" then
    send(opts.abbreviation)
    return
  end

  vim.ui.input({ prompt = "codemmet wrap: " }, function(input)
    if input == nil or input == "" then
      return -- cancelled
    end
    send(input)
  end)
end

---Normal-mode operator entry point for `keymaps.wrap`. Use from an
---expression mapping: it arms `operatorfunc` and returns `"g@"`, so the
---mapping composes with any motion or text object (`<leader>cw ip`,
---`<leader>cw j`, ...). Prompts for the abbreviation on first use; `.`
---then re-applies the same abbreviation over the repeated motion at the
---new cursor position, without prompting.
---@return string keys for the expression mapping ("g@")
function M.operator()
  state.fresh = true
  vim.go.operatorfunc = "v:lua.require'codemmet.wrap'.opfunc"
  return "g@"
end

---`operatorfunc` callback for the wrap operator. Receives the motion range
---through the `'[`/`']` marks (always treated line-wise, like visual wrap).
---Fresh invocations prompt via `vim.ui.input` and store the abbreviation;
---`.` repeats reuse the stored one.
function M.opfunc()
  local line1 = vim.api.nvim_buf_get_mark(0, "[")[1]
  local line2 = vim.api.nvim_buf_get_mark(0, "]")[1]
  local fresh = state.fresh
  state.fresh = false

  if not fresh and state.abbreviation then
    M.wrap({ abbreviation = state.abbreviation, line1 = line1, line2 = line2 })
    return
  end

  vim.ui.input({ prompt = "codemmet wrap: " }, function(input)
    if input == nil or input == "" then
      return -- cancelled; a later `.` will prompt again
    end
    state.abbreviation = input
    M.wrap({ abbreviation = input, line1 = line1, line2 = line2 })
  end)
end

return M
