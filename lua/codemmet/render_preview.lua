---Live render preview: a scratch vertical split showing the current buffer
---rendered as a `.codemmet` source file (`codemmet render`, see
---`FILE_MODE_PLAN.md`), refreshed on a debounced TextChanged/TextChangedI.
---
---The render is atomic and side-effect-free: on a nonzero exit the split
---shows the `<stdin>:N:`-numbered diagnostic (highlighted as an error)
---instead of stale output, and the source buffer is never touched.
local M = {}

local NS = vim.api.nvim_create_namespace("codemmet_render_preview")
local ERROR_HL = "CodemmetRenderError"

---@class codemmet.RenderPreviewConfig
---@field cmd? string codemmet binary (default: the attached LSP client's, else "codemmet")
---@field debounce_ms integer delay between the last edit and the re-render
M.config = { cmd = nil, debounce_ms = 150 }

---@class codemmet.RenderPreviewState
---@field win integer the preview window
---@field buf integer the scratch preview buffer
---@field timer uv.uv_timer_t debounce timer
---@field generation integer token: stale async render replies are dropped
---@field group integer augroup id for the source buffer's autocmds
---@field filetype string the filetype currently set on the preview buffer

---Active previews, keyed by source bufnr.
---@type table<integer, codemmet.RenderPreviewState>
local states = {}

local function ensure_highlight()
  if vim.fn.hlexists(ERROR_HL) == 0 then
    vim.api.nvim_set_hl(0, ERROR_HL, { link = "DiagnosticError", default = true })
  end
end

---The codemmet binary to run: explicit config, the attached LSP client's
---command, or plain "codemmet" from PATH.
---@param bufnr integer
---@return string
local function binary(bufnr)
  if M.config.cmd then
    return M.config.cmd
  end
  local client = require("codemmet.lsp").get_client(bufnr)
  local cmd = client and client.config.cmd
  if type(cmd) == "table" and type(cmd[1]) == "string" then
    return cmd[1]
  end
  return "codemmet"
end

---The source's target language, mirroring the CLI's resolution ladder:
---a `#lang` directive, the `NAME.EXT.codemmet` inner extension, or the
---buffer's own filetype. `#lang` wins server-side regardless; resolving it
---here too keeps the split's syntax highlighting in step.
---@param bufnr integer
---@return string? lang a language id or file extension, if one resolves
local function resolve_lang(bufnr)
  for _, line in ipairs(vim.api.nvim_buf_get_lines(bufnr, 0, -1, false)) do
    local id = line:match("^#lang%s+(%S+)")
    if id then
      return id
    end
    if line ~= "" and not line:match("^#") then
      break -- directives only count before the first abbreviation
    end
  end
  local inner = vim.api.nvim_buf_get_name(bufnr):match("(.+)%.codemmet$")
  local ext = inner and inner:match("%.(%w+)$")
  if ext then
    return ext
  end
  local ft = vim.bo[bufnr].filetype
  if ft ~= "" and ft ~= "codemmet" then
    return ft
  end
  return nil
end

---The nvim filetype for a language id or extension ("rs" -> "rust"), for
---syntax highlighting in the split.
---@param lang string?
---@return string
local function filetype_for(lang)
  if not lang then
    return ""
  end
  return vim.filetype.match({ filename = "codemmet-preview." .. lang }) or lang
end

---Replace the preview buffer's contents (it stays non-modifiable outside
---this window of time) and clear any previous error highlight.
---@param state codemmet.RenderPreviewState
---@param lines string[]
local function set_lines(state, lines)
  vim.bo[state.buf].modifiable = true
  vim.api.nvim_buf_set_lines(state.buf, 0, -1, false, lines)
  vim.bo[state.buf].modifiable = false
  vim.api.nvim_buf_clear_namespace(state.buf, NS, 0, -1)
end

---Show a successful render, highlighted as the target language.
---@param state codemmet.RenderPreviewState
---@param output string the rendered code (stdout)
---@param filetype string
local function show_output(state, output, filetype)
  local lines = vim.split(output, "\n", { plain = true })
  if lines[#lines] == "" then
    table.remove(lines) -- the trailing newline is not an extra buffer line
  end
  set_lines(state, lines)
  if filetype ~= state.filetype then
    state.filetype = filetype
    vim.bo[state.buf].filetype = filetype
  end
end

---Show a render failure: the `<stdin>:N:` diagnostic, highlighted as error.
---@param state codemmet.RenderPreviewState
---@param message string
local function show_error(state, message)
  ensure_highlight()
  local lines = vim.split(message, "\n", { plain = true, trimempty = true })
  set_lines(state, lines)
  for row = 0, #lines - 1 do
    vim.api.nvim_buf_set_extmark(state.buf, NS, row, 0, { line_hl_group = ERROR_HL })
  end
end

---Render the source buffer through `codemmet render --lang <lang> -` and
---show the result. Async; a generation token drops replies that were
---overtaken by a newer edit.
---@param src integer source bufnr
local function refresh(src)
  local state = states[src]
  if not state or not vim.api.nvim_buf_is_valid(src) then
    return
  end
  state.generation = state.generation + 1
  local generation = state.generation
  local lang = resolve_lang(src)
  local cmd = { binary(src), "render" }
  if lang then
    vim.list_extend(cmd, { "--lang", lang })
  end
  table.insert(cmd, "-")
  local text = table.concat(vim.api.nvim_buf_get_lines(src, 0, -1, false), "\n") .. "\n"
  local ok, err = pcall(vim.system, cmd, { stdin = text, text = true }, function(obj)
    vim.schedule(function()
      local st = states[src]
      -- Dropped: preview toggled off, or a newer render is in flight.
      if not st or st.generation ~= generation or not vim.api.nvim_buf_is_valid(st.buf) then
        return
      end
      if obj.code == 0 then
        show_output(st, obj.stdout or "", filetype_for(lang))
      else
        local message = vim.trim(obj.stderr or "")
        if message == "" then
          message = ("codemmet render exited with code %d"):format(obj.code)
        end
        show_error(st, message)
      end
    end)
  end)
  if not ok then
    show_error(state, "cannot run codemmet render: " .. tostring(err))
  end
end

---(Re)start the debounce timer for a source buffer.
---@param src integer
local function schedule(src)
  local state = states[src]
  if not state then
    return
  end
  state.timer:stop()
  state.timer:start(
    M.config.debounce_ms,
    0,
    vim.schedule_wrap(function()
      refresh(src)
    end)
  )
end

---Tear the preview down: cancel the timer, drop the source's autocmds, and
---close the split (idempotent; safe to call from the teardown autocmds).
---@param src integer source bufnr
function M.detach(src)
  local state = states[src]
  if not state then
    return
  end
  states[src] = nil -- first, so autocmds fired by the teardown are no-ops
  state.timer:stop()
  state.timer:close()
  pcall(vim.api.nvim_del_augroup_by_id, state.group)
  if vim.api.nvim_win_is_valid(state.win) then
    pcall(vim.api.nvim_win_close, state.win, true)
    if vim.api.nvim_win_is_valid(state.win) then
      -- The preview ended up as the last window (wiping the source closed
      -- the source's own window first): show a fresh buffer instead.
      vim.api.nvim_win_set_buf(state.win, vim.api.nvim_create_buf(true, false))
    end
  end
  if vim.api.nvim_buf_is_valid(state.buf) then
    pcall(vim.api.nvim_buf_delete, state.buf, { force = true })
  end
end

---Open the preview split for a source buffer and start tracking it.
---@param src integer source bufnr
function M.attach(src)
  if states[src] then
    return
  end
  local buf = vim.api.nvim_create_buf(false, true)
  vim.bo[buf].buftype = "nofile"
  vim.bo[buf].bufhidden = "wipe"
  vim.bo[buf].swapfile = false
  vim.bo[buf].modifiable = false
  vim.api.nvim_buf_set_name(buf, "codemmet://render-preview/" .. src)
  -- A right-hand vertical split; focus stays on the source.
  local win = vim.api.nvim_open_win(buf, false, { split = "right", win = 0 })

  local group = vim.api.nvim_create_augroup("codemmet_render_preview_" .. src, { clear = true })
  local state = {
    win = win,
    buf = buf,
    timer = vim.uv.new_timer(),
    generation = 0,
    group = group,
    filetype = "",
  }
  states[src] = state

  vim.api.nvim_create_autocmd({ "TextChanged", "TextChangedI" }, {
    group = group,
    buffer = src,
    desc = "codemmet render preview refresh",
    callback = function()
      schedule(src)
    end,
  })
  vim.api.nvim_create_autocmd({ "BufWipeout", "BufDelete" }, {
    group = group,
    buffer = src,
    desc = "codemmet render preview teardown (source gone)",
    callback = function()
      -- The source buffer is mid-teardown; close the split a beat later.
      vim.schedule(function()
        M.detach(src)
      end)
    end,
  })
  vim.api.nvim_create_autocmd("WinClosed", {
    group = group,
    pattern = tostring(win),
    desc = "codemmet render preview teardown (split closed)",
    callback = function()
      vim.schedule(function()
        M.detach(src)
      end)
    end,
  })

  refresh(src) -- first render immediately; edits go through the debounce
end

---Toggle the render preview split for the current buffer.
function M.toggle()
  local src = vim.api.nvim_get_current_buf()
  if states[src] then
    M.detach(src)
    vim.notify("codemmet: render preview off", vim.log.levels.INFO)
  else
    M.attach(src)
    vim.notify("codemmet: render preview on", vim.log.levels.INFO)
  end
end

---Is a preview currently attached to this source buffer? (test hook)
---@param src integer
---@return codemmet.RenderPreviewState?
function M.state(src)
  return states[src]
end

return M
