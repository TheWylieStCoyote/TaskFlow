---Wrap a line-wise selection in a codemmet abbreviation via the server's
---`codemmet.wrap` command.
local lsp = require("codemmet.lsp")

local M = {}

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

return M
