---Live ghost-line preview: as you type an abbreviation in insert mode, the
---server's `codemmet.preview` command returns the clean expansion, shown as
---dimmed virtual lines below the current line until you accept it or move away.
local M = {}

local lsp = require("codemmet.lsp")

local NS = vim.api.nvim_create_namespace("codemmet_preview")
local HL = "CodemmetPreview"
local DEBOUNCE_MS = 60

-- Per-buffer debounce timers, keyed by bufnr.
local timers = {}

local function ensure_highlight()
  if vim.fn.hlexists(HL) == 0 then
    vim.api.nvim_set_hl(0, HL, { link = "Comment", default = true })
  end
end

---Remove any preview virtual lines from a buffer.
---@param bufnr integer
function M.clear(bufnr)
  if vim.api.nvim_buf_is_valid(bufnr) then
    vim.api.nvim_buf_clear_namespace(bufnr, NS, 0, -1)
  end
end

---Render `text` as dimmed virtual lines below the abbreviation's line.
---
---Public so callers (and tests) can drive a one-shot preview; the automatic
---path calls this from [`refresh`]. It edits nothing — only virtual text.
---@param bufnr integer
---@param range lsp.Range the abbreviation's range (server-reported)
---@param text string the clean expansion
function M.show(bufnr, range, text)
  M.clear(bufnr)
  if not text or text == "" then
    return
  end
  -- Anchor to the line the abbreviation sits on (a candidate is single-line),
  -- so the preview stays put even if the async reply arrives a beat late.
  local row = range["end"].line
  if row >= vim.api.nvim_buf_line_count(bufnr) then
    return
  end
  -- The server omits the base indent on the first expansion line (the caller
  -- is assumed to already sit at that column), so prefix the abbreviation
  -- line's own indent to line 1 for the ghost block to align under the code.
  local indent = ((vim.api.nvim_buf_get_lines(bufnr, row, row + 1, false)[1] or ""):match("^%s*"))
  local virt_lines = {}
  for i, line in ipairs(vim.split(text, "\n", { plain = true })) do
    local shown = i == 1 and indent .. line or line
    virt_lines[#virt_lines + 1] = { { shown, HL } }
  end
  vim.api.nvim_buf_set_extmark(bufnr, NS, row, 0, {
    virt_lines = virt_lines,
    virt_lines_above = false,
  })
end

---Still previewing here? Only when this buffer is current and in insert mode.
local function previewing(bufnr)
  return vim.api.nvim_get_current_buf() == bufnr and vim.api.nvim_get_mode().mode:find("i") ~= nil
end

---Query `codemmet.preview` for the abbreviation before the cursor and render it.
---@param bufnr integer
local function refresh(bufnr)
  if not previewing(bufnr) then
    M.clear(bufnr)
    return
  end
  local client = lsp.get_client(bufnr)
  if not client then
    return
  end
  local position = vim.lsp.util.make_position_params(0, client.offset_encoding).position
  lsp.exec(client, bufnr, "codemmet.preview", {
    {
      uri = vim.uri_from_bufnr(bufnr),
      position = position,
      formatting_options = lsp.formatting_options(bufnr),
    },
  }, function(err, result)
    if err or not previewing(bufnr) then
      return
    end
    if result and result.text then
      M.show(bufnr, result.range, result.text)
    else
      M.clear(bufnr)
    end
  end)
end

---Restart the buffer's debounce timer.
---@param bufnr integer
local function schedule(bufnr)
  local timer = timers[bufnr]
  if not timer then
    timer = vim.uv.new_timer()
    timers[bufnr] = timer
  end
  timer:stop()
  timer:start(
    DEBOUNCE_MS,
    0,
    vim.schedule_wrap(function()
      refresh(bufnr)
    end)
  )
end

---Free a buffer's debounce timer.
---@param bufnr integer
local function drop_timer(bufnr)
  local timer = timers[bufnr]
  if timer then
    timer:stop()
    timer:close()
    timers[bufnr] = nil
  end
end

local function augroup(bufnr)
  return "codemmet_preview_" .. bufnr
end

---Turn the live preview on for a buffer (idempotent).
---@param bufnr integer
function M.attach(bufnr)
  ensure_highlight()
  local group = vim.api.nvim_create_augroup(augroup(bufnr), { clear = true })
  vim.api.nvim_create_autocmd({ "TextChangedI", "CursorMovedI" }, {
    group = group,
    buffer = bufnr,
    desc = "codemmet live preview",
    callback = function()
      schedule(bufnr)
    end,
  })
  vim.api.nvim_create_autocmd({ "InsertLeave", "BufLeave" }, {
    group = group,
    buffer = bufnr,
    callback = function()
      M.clear(bufnr)
    end,
  })
  vim.api.nvim_create_autocmd({ "BufWipeout", "BufDelete" }, {
    group = group,
    buffer = bufnr,
    callback = function()
      drop_timer(bufnr)
    end,
  })
end

---Turn the live preview off for a buffer.
---@param bufnr integer
function M.detach(bufnr)
  pcall(vim.api.nvim_del_augroup_by_name, augroup(bufnr))
  drop_timer(bufnr)
  M.clear(bufnr)
end

---Toggle the live preview for the current buffer.
function M.toggle()
  local bufnr = vim.api.nvim_get_current_buf()
  -- An existing augroup for this buffer means the preview is attached.
  local attached = pcall(vim.api.nvim_get_autocmds, { group = augroup(bufnr) })
  if attached then
    M.detach(bufnr)
    vim.notify("codemmet: live preview off", vim.log.levels.INFO)
  else
    M.attach(bufnr)
    vim.notify("codemmet: live preview on", vim.log.levels.INFO)
  end
end

return M
