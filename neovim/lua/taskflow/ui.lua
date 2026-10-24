--- Floating window and picker helpers for TaskFlow.
local M = {}

-- Tracks the :TaskFlowOpen float: buffer, window (nil when hidden), job id.
local state = { buf = nil, win = nil, job = nil }

local function float_config()
  local cfg    = require('taskflow.config').get()
  local width  = math.floor(vim.o.columns * cfg.float_width)
  local height = math.floor(vim.o.lines   * cfg.float_height)
  return {
    relative = 'editor', width = width, height = height,
    row = math.floor((vim.o.lines - height) / 2),
    col = math.floor((vim.o.columns - width) / 2),
    style = 'minimal', border = 'rounded',
    title = ' TaskFlow ', title_pos = 'center',
  }
end

--- True when the TUI float window is currently shown.
function M.float_is_open()
  return state.win ~= nil and vim.api.nvim_win_is_valid(state.win)
end

--- True when a hidden float buffer with a live job exists.
function M.float_is_hidden()
  return not M.float_is_open() and state.buf ~= nil
    and vim.api.nvim_buf_is_valid(state.buf) and state.job ~= nil
end

--- Hide the float window, keeping buffer and job alive.
function M.hide_float()
  if M.float_is_open() then
    vim.api.nvim_win_close(state.win, true)
  end
  state.win = nil
end

--- Re-show a hidden float buffer in a new window.
function M.show_float()
  if not M.float_is_hidden() then return nil end
  state.win = vim.api.nvim_open_win(state.buf, true, float_config())
  vim.cmd('startinsert')
  return state.win
end

--- Open a floating terminal window running the given command.
---@param cmd string  shell command to run inside the terminal
---@return integer    window id
function M.open_float_terminal(cmd)
  local buf = vim.api.nvim_create_buf(false, true)
  local win = vim.api.nvim_open_win(buf, true, float_config())

  state.buf, state.win = buf, win
  vim.bo[buf].bufhidden = 'hide'
  local term_opts = {
    on_exit = function()
      if state.buf == buf then
        state.buf, state.win, state.job = nil, nil, nil
      end
      if vim.api.nvim_win_is_valid(win) then
        pcall(vim.api.nvim_win_close, win, true)
      end
      if vim.api.nvim_buf_is_valid(buf) then
        pcall(vim.api.nvim_buf_delete, buf, { force = true })
      end
    end,
  }
  if vim.fn.has('nvim-0.11') == 1 then
    state.job = vim.fn.jobstart(cmd, vim.tbl_extend('force', term_opts, { term = true }))
  else
    state.job = vim.fn.termopen(cmd, term_opts) -- 0.10 fallback
  end
  vim.cmd('startinsert')
  return win
end

--- Show a floating list of task summaries with keybindings.
---@param tasks  table[]  list of TaskSummary objects from the socket
---@param on_select function(task)  called when user presses <CR>
--- Render a task's tags as ` #a #b`, or '' when there are none.
---@param tags string[]|nil
---@return string
function M.format_tags(tags)
  if type(tags) ~= 'table' or #tags == 0 then return '' end
  local parts = {}
  for i, tag in ipairs(tags) do parts[i] = '#' .. tag end
  return '  ' .. table.concat(parts, ' ')
end

function M.open_task_list(tasks, on_select)
  if not tasks or #tasks == 0 then
    vim.notify('[TaskFlow] No tasks to display', vim.log.levels.INFO)
    return
  end

  local lines = {}
  for i, t in ipairs(tasks) do
    local status_sym = ({
      todo        = '[ ]',
      in_progress = '[~]',
      blocked     = '[!]',
      done        = '[x]',
      cancelled   = '[-]',
    })[t.status] or '   '
    local priority_sym = ({
      urgent = '!!!',
      high   = '!! ',
      medium = '!  ',
      low    = '.  ',
      none   = '   ',
    })[t.priority] or '   '
    lines[i] = string.format('%s %s  %s%s', status_sym, priority_sym, t.title or '', M.format_tags(t.tags))
  end

  local cfg    = require('taskflow.config').get()
  local width  = math.floor(vim.o.columns * cfg.float_width)
  local height = math.min(#lines + 2, math.floor(vim.o.lines * cfg.float_height))
  local row    = math.floor((vim.o.lines   - height) / 2)
  local col    = math.floor((vim.o.columns - width)  / 2)

  local buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
  vim.bo[buf].modifiable = false
  vim.bo[buf].bufhidden  = 'wipe'

  local win = vim.api.nvim_open_win(buf, true, {
    relative  = 'editor',
    width     = width,
    height    = height,
    row       = row,
    col       = col,
    style     = 'minimal',
    border    = 'rounded',
    title     = ' TaskFlow – Tasks (q/Esc: close, Enter: focus) ',
    title_pos = 'center',
  })

  local function close()
    if vim.api.nvim_win_is_valid(win) then
      vim.api.nvim_win_close(win, true)
    end
  end

  local opts = { buffer = buf, nowait = true, noremap = true, silent = true }
  vim.keymap.set('n', 'q',   close, opts)
  vim.keymap.set('n', '<Esc>', close, opts)
  vim.keymap.set('n', '<CR>', function()
    local idx = vim.api.nvim_win_get_cursor(win)[1]
    local task = tasks[idx]
    close()
    if task and on_select then
      on_select(task)
    end
  end, opts)
end

return M
