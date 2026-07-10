---Snippet browser and context viewer, backed by the server's read-only
---`codemmet.list` and `codemmet.context` commands.
local lsp = require("codemmet.lsp")

local M = {}

---A result field as a non-empty string, or nil (JSON null decodes to vim.NIL).
---@param value any
---@return string|nil
local function str(value)
  if type(value) == "string" and value ~= "" then
    return value
  end
  return nil
end

---The `{ uri, position }` argument both commands take, at the window cursor.
---@param bufnr integer
---@param client vim.lsp.Client
---@return { uri: string, position: lsp.Position }
local function cursor_args(bufnr, client)
  return {
    uri = vim.uri_from_bufnr(bufnr),
    position = vim.lsp.util.make_position_params(0, client.offset_encoding).position,
  }
end

---The codemmet client for the current buffer, notifying when absent.
---@return integer|nil bufnr, vim.lsp.Client|nil client
local function attached_client()
  local bufnr = vim.api.nvim_get_current_buf()
  local client = lsp.get_client(bufnr)
  if not client then
    vim.notify("codemmet: no codemmet client attached to this buffer", vim.log.levels.WARN)
    return nil, nil
  end
  return bufnr, client
end

---One picker line: aligned trigger, description, the variant that would fire
---at the cursor, and a one-line body hint.
---@param item table one `codemmet.list` snippet entry
---@param width integer trigger column width
---@return string
local function format_item(item, width)
  local parts = { ("%-" .. width .. "s"):format(item.trigger) }
  local description = str(item.description)
  if description then
    parts[#parts + 1] = description
  end
  local context = str(item.activeContext)
  if context then
    parts[#parts + 1] = "[" .. context .. "]"
  end
  local hint = str(item.hint)
  if hint then
    parts[#parts + 1] = "· " .. hint
  end
  return table.concat(parts, "  ")
end

---Browse every trigger registered for this buffer (via `vim.ui.select`) and
---expand the picked one at the cursor. Entries are annotated with the scope
---variant that would fire at the cursor position.
function M.pick()
  local bufnr, client = attached_client()
  if not bufnr or not client then
    return
  end
  local args = cursor_args(bufnr, client)
  lsp.exec(client, bufnr, "codemmet.list", { args }, function(err, result)
    if err then
      vim.notify("codemmet: " .. (err.message or vim.inspect(err)), vim.log.levels.WARN)
      return
    end
    local snippets = result and result.snippets or {}
    if #snippets == 0 then
      vim.notify("codemmet: no snippets for this buffer", vim.log.levels.WARN)
      return
    end
    local width = 0
    for _, item in ipairs(snippets) do
      width = math.max(width, #item.trigger)
    end
    vim.ui.select(snippets, {
      prompt = "codemmet snippets (" .. (str(result.language) or "?") .. ")",
      format_item = function(item)
        return format_item(item, width)
      end,
    }, function(choice)
      if not choice or vim.api.nvim_get_current_buf() ~= bufnr then
        return
      end
      local body = str(choice.body)
      if not body then
        -- No pre-rendered body (e.g. the server withheld one): insert the
        -- trigger and run the ordinary expand flow — a bare trigger always
        -- expands, with its unfilled params becoming tabstops.
        local row = args.position.line
        local col = args.position.character
        local line = vim.api.nvim_buf_get_lines(bufnr, row, row + 1, false)[1] or ""
        col = math.min(col, #line)
        vim.api.nvim_buf_set_text(bufnr, row, col, row, col, { choice.trigger })
        -- Place the normal-mode cursor ON the trigger's last character —
        -- exactly where it would sit after typing it, which is the position
        -- the expand flow expects.
        vim.api.nvim_win_set_cursor(0, { row + 1, col + #choice.trigger - 1 })
        lsp.expand()
        return
      end
      -- A zero-width range at the cursor: nothing to delete, just expand.
      local range = { start = args.position, ["end"] = args.position }
      lsp.apply_snippet(bufnr, range, body, client.offset_encoding)
    end)
  end)
end

---Render one collection item: `value` with its `type` member when present.
---@param item { value: string, members: table<string, string> }
---@return string
local function format_collection_item(item)
  local ty = type(item.members) == "table" and str(item.members.type) or nil
  if ty then
    return item.value .. ": " .. ty
  end
  return item.value
end

---The display lines for a `codemmet.context` result.
---@param result table
---@return string[]
local function context_lines(result)
  local lines = { "language: " .. (str(result.language) or "?") }
  local scopes = {}
  for _, scope in ipairs(result.scopes or {}) do
    scopes[#scopes + 1] = scope
  end
  lines[#lines + 1] = "scopes:   " .. table.concat(scopes, " > ")
  if result.inStringOrComment == true then
    lines[#lines + 1] = "cursor:   inside a string or comment"
  end
  local collections = result.collections or {}
  if #collections == 0 then
    lines[#lines + 1] = "collections: (none in scope)"
    return lines
  end
  lines[#lines + 1] = "collections:"
  for _, collection in ipairs(collections) do
    local values = {}
    for _, item in ipairs(collection.items or {}) do
      values[#values + 1] = format_collection_item(item)
    end
    lines[#lines + 1] = "  " .. collection.name .. ": " .. table.concat(values, ", ")
  end
  return lines
end

---Show the codemmet context at the cursor — the scope stack and the in-scope
---collections data-driven repeats can expand over — in a floating window.
function M.context()
  local bufnr, client = attached_client()
  if not bufnr or not client then
    return
  end
  local args = cursor_args(bufnr, client)
  lsp.exec(client, bufnr, "codemmet.context", { args }, function(err, result)
    if err then
      vim.notify("codemmet: " .. (err.message or vim.inspect(err)), vim.log.levels.WARN)
      return
    end
    if not result or vim.api.nvim_get_current_buf() ~= bufnr then
      return
    end
    -- Closes itself on cursor move; a repeat invocation focuses it.
    vim.lsp.util.open_floating_preview(context_lines(result), "", {
      border = "rounded",
      focus_id = "codemmet-context",
    })
  end)
end

return M
