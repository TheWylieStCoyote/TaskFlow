---Dogfooding capture: append timestamped, context-tagged notes to a file so
---friction is recorded in the moment. Driven by `:CodemmetNote`.
local M = {}

-- Where notes are appended; overridable via `setup({ notes_file = ... })`.
local file = vim.fn.stdpath("data") .. "/codemmet/notes.md"

local HEADER = table.concat({
  "# codemmet dogfooding notes",
  "",
  "Captured with `:CodemmetNote`. Each entry records when, the language,",
  "`file:line`, your note, and the line under the cursor for context.",
  "",
  "",
}, "\n")

---Point notes at a different file (no-op when `path` is nil/empty).
---@param path? string
function M.set_file(path)
  if path and path ~= "" then
    file = path
  end
end

---The current notes file path.
---@return string
function M.file()
  return file
end

---Append a note, tagged with the current buffer's language, file, line number,
---and the text of the line under the cursor.
---@param text string
function M.note(text)
  text = vim.trim(text or "")
  if text == "" then
    vim.notify("codemmet: empty note, nothing saved", vim.log.levels.WARN)
    return
  end

  local ft = vim.bo.filetype
  if ft == "" then
    ft = "-"
  end
  local name = vim.api.nvim_buf_get_name(0)
  name = name ~= "" and vim.fn.fnamemodify(name, ":t") or "[No Name]"
  local row = vim.api.nvim_win_get_cursor(0)[1]
  local line = vim.trim(vim.api.nvim_get_current_line())

  local entry =
    string.format("- %s [%s] %s:%d — %s", os.date("%Y-%m-%d %H:%M"), ft, name, row, text)
  if line ~= "" then
    entry = entry .. string.format("  ·  line: `%s`", line)
  end

  vim.fn.mkdir(vim.fn.fnamemodify(file, ":h"), "p")
  local existed = vim.uv.fs_stat(file) ~= nil
  local handle, err = io.open(file, "a")
  if not handle then
    vim.notify("codemmet: could not write notes file: " .. tostring(err), vim.log.levels.ERROR)
    return
  end
  if not existed then
    handle:write(HEADER)
  end
  handle:write(entry .. "\n")
  handle:close()
  vim.notify("codemmet: note saved to " .. file, vim.log.levels.INFO)
end

return M
