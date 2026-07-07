---Capture a line-wise selection into the project snippet pack via the server's
---`codemmet.capture` command. The server generalizes the selected code into a
---parameterized snippet, writes it to `.codemmet/snippets/<lang>.toml`, and
---reloads so the new trigger is expandable immediately.
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

---Capture the selected lines into the project pack under trigger `name`.
---Prompts via `vim.ui.input` for the name unless `opts.name` is given.
---@param opts? codemmet.CaptureOpts
function M.capture(opts)
  opts = opts or {}
  local bufnr = vim.api.nvim_get_current_buf()
  local client = lsp.get_client(bufnr)
  if not client then
    vim.notify("codemmet: no codemmet client attached to this buffer", vim.log.levels.WARN)
    return
  end

  local line1, line2 = resolve_lines(opts)
  local range = {
    start = { line = line1 - 1, character = 0 },
    ["end"] = {
      line = line2 - 1,
      character = lsp.line_end_character(bufnr, line2 - 1, client.offset_encoding),
    },
  }

  ---@param name string
  local function send(name)
    lsp.exec(client, bufnr, "codemmet.capture", {
      {
        uri = vim.uri_from_bufnr(bufnr),
        range = range,
        name = name,
        description = opts.description,
      },
    }, function(err, result)
      if err then
        vim.notify("codemmet: " .. (err.message or vim.inspect(err)), vim.log.levels.WARN)
        return
      end
      if not result or not result.path then
        return
      end
      local params = (result.params and #result.params > 0)
          and (" [" .. table.concat(result.params, ", ") .. "]")
        or ""
      vim.notify(
        "codemmet: captured `" .. result.trigger .. "`" .. params .. " -> " .. result.path,
        vim.log.levels.INFO
      )
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
