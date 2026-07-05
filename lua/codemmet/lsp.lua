---LSP plumbing for the codemmet server: client lookup, executeCommand wrapper,
---position/range conversion, and the shared "delete range then expand snippet" apply path.
local M = {}

local has_011 = vim.fn.has("nvim-0.11") == 1

---Get the codemmet client attached to a buffer, if any.
---@param bufnr integer
---@return vim.lsp.Client|nil
function M.get_client(bufnr)
  return vim.lsp.get_clients({ name = "codemmet", bufnr = bufnr })[1]
end

---Send a `workspace/executeCommand` request to the codemmet server.
---@param client vim.lsp.Client
---@param bufnr integer
---@param command string e.g. "codemmet.expand"
---@param arguments any[] command arguments
---@param handler fun(err: lsp.ResponseError|nil, result: any)
function M.exec(client, bufnr, command, arguments, handler)
  local params = { command = command, arguments = arguments }
  if has_011 then
    return client:request("workspace/executeCommand", params, handler, bufnr)
  end
  ---@diagnostic disable-next-line: param-type-mismatch
  return client.request("workspace/executeCommand", params, handler, bufnr)
end

---LSP FormattingOptions for a buffer.
---@param bufnr integer
---@return { tab_size: integer, insert_spaces: boolean }
function M.formatting_options(bufnr)
  local tab_size = vim.bo[bufnr].shiftwidth
  if tab_size == 0 then
    tab_size = vim.bo[bufnr].tabstop
  end
  return { tab_size = tab_size, insert_spaces = vim.bo[bufnr].expandtab }
end

---Convert an LSP character offset on one line to a byte column, clamped to the line.
---@param line string
---@param character integer
---@param offset_encoding string "utf-8"|"utf-16"|"utf-32"
---@return integer byte column (0-based)
local function character_to_byte(line, character, offset_encoding)
  if offset_encoding == "utf-8" then
    return math.min(character, #line)
  end
  if has_011 then
    return vim.str_byteindex(line, offset_encoding, character, false)
  end
  ---@diagnostic disable-next-line: param-type-mismatch
  local ok, byte = pcall(vim.str_byteindex, line, character, offset_encoding == "utf-16")
  return ok and byte or #line
end

---Convert an LSP range to 0-based byte coordinates for `nvim_buf_set_text`.
---@param bufnr integer
---@param range lsp.Range
---@param offset_encoding string "utf-8"|"utf-16"|"utf-32"
---@return integer start_row, integer start_col, integer end_row, integer end_col
function M.range_to_bytes(bufnr, range, offset_encoding)
  local function to_bytes(position)
    local row = position.line
    local line = vim.api.nvim_buf_get_lines(bufnr, row, row + 1, false)[1] or ""
    return row, character_to_byte(line, position.character, offset_encoding)
  end
  local start_row, start_col = to_bytes(range.start)
  local end_row, end_col = to_bytes(range["end"])
  return start_row, start_col, end_row, end_col
end

---LSP character offset of the end of a buffer line.
---@param bufnr integer
---@param row integer 0-based line index
---@param offset_encoding string "utf-8"|"utf-16"|"utf-32"
---@return integer
function M.line_end_character(bufnr, row, offset_encoding)
  local line = vim.api.nvim_buf_get_lines(bufnr, row, row + 1, false)[1] or ""
  if offset_encoding == "utf-8" then
    return #line
  end
  if has_011 then
    return vim.str_utfindex(line, offset_encoding)
  end
  ---@diagnostic disable-next-line: missing-parameter
  local utf32, utf16 = vim.str_utfindex(line)
  return offset_encoding == "utf-16" and utf16 or utf32
end

---Delete `range` from the buffer and expand `snippet` at the range start.
---Works from both insert and normal mode.
---@param bufnr integer
---@param range lsp.Range as returned by the server
---@param snippet string LSP snippet text
---@param offset_encoding string the owning client's offset encoding
function M.apply_snippet(bufnr, range, snippet, offset_encoding)
  local start_row, start_col, end_row, end_col = M.range_to_bytes(bufnr, range, offset_encoding)
  vim.api.nvim_buf_set_text(bufnr, start_row, start_col, end_row, end_col, {})

  -- vim.snippet.expand() inserts at the window cursor, and in normal mode the cursor
  -- cannot normally sit one past end-of-line; 'virtualedit=onemore' keeps the insertion
  -- point exactly at the range start regardless of mode.
  local saved_virtualedit = vim.wo.virtualedit
  vim.wo.virtualedit = "onemore"
  local ok, err = pcall(function()
    vim.api.nvim_win_set_cursor(0, { start_row + 1, start_col })
    vim.snippet.expand(snippet)
  end)
  vim.wo.virtualedit = saved_virtualedit
  if not ok then
    vim.notify("codemmet: " .. tostring(err), vim.log.levels.ERROR)
  end
end

---Expand the abbreviation immediately before the cursor.
---Notifies (rather than errors) when no codemmet client is attached or the server rejects
---the abbreviation.
function M.expand()
  local bufnr = vim.api.nvim_get_current_buf()
  local client = M.get_client(bufnr)
  if not client then
    vim.notify("codemmet: no codemmet client attached to this buffer", vim.log.levels.WARN)
    return
  end

  local position = vim.lsp.util.make_position_params(0, client.offset_encoding).position
  M.exec(client, bufnr, "codemmet.expand", {
    {
      uri = vim.uri_from_bufnr(bufnr),
      position = position,
      formatting_options = M.formatting_options(bufnr),
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
    M.apply_snippet(bufnr, result.range, result.snippet, client.offset_encoding)
  end)
end

return M
