--- Extract TODO/FIXME/HACK/NOTE comments from a buffer.
local M = {}

-- Patterns to match.  Each entry is { label, lua_pattern }.
-- The pattern must capture the comment text after the keyword.
local PATTERNS = {
  { 'TODO',  '[%s#/%-*]*TODO[%s:!]*(.+)$' },
  { 'FIXME', '[%s#/%-*]*FIXME[%s:!]*(.+)$' },
  { 'HACK',  '[%s#/%-*]*HACK[%s:!]*(.+)$' },
  { 'NOTE',  '[%s#/%-*]*NOTE[%s:!]*(.+)$' },
}

--- Scan the given buffer for comment annotations.
---
---@param  bufnr integer  buffer number (0 = current)
---@return table[]        list of { text=string, lnum=integer, pattern=string }
function M.scan_buffer(bufnr)
  bufnr = bufnr or 0
  if bufnr == 0 then
    bufnr = vim.api.nvim_get_current_buf()
  end

  local lines = vim.api.nvim_buf_get_lines(bufnr, 0, -1, false)
  local results = {}

  for lnum, line in ipairs(lines) do
    for _, pat in ipairs(PATTERNS) do
      local label, pattern = pat[1], pat[2]
      local text = line:match(pattern)
      if text then
        text = text:match('^%s*(.-)%s*$') -- trim whitespace
        if text ~= '' then
          table.insert(results, {
            text    = text,
            lnum    = lnum,
            pattern = label,
          })
        end
        break -- only match one pattern per line
      end
    end
  end

  return results
end

return M
