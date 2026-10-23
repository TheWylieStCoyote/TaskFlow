--- Floating window and picker helpers for TaskFlow.
local M = {}

--- Open a floating terminal window running the given command.
---@param cmd string  shell command to run inside the terminal
---@return integer    window id
function M.open_float_terminal(cmd)
  local cfg   = require('taskflow.config').get()
  local width  = math.floor(vim.o.columns * cfg.float_width)
  local height = math.floor(vim.o.lines   * cfg.float_height)
  local row    = math.floor((vim.o.lines   - height) / 2)
  local col    = math.floor((vim.o.columns - width)  / 2)

  local buf = vim.api.nvim_create_buf(false, true)
  local win = vim.api.nvim_open_win(buf, true, {
    relative = 'editor',
    width    = width,
    height   = height,
    row      = row,
    col      = col,
    style    = 'minimal',
    border   = 'rounded',
    title    = ' TaskFlow ',
    title_pos = 'center',
  })

  vim.fn.termopen(cmd, {
    on_exit = function()
      if vim.api.nvim_win_is_valid(win) then
        vim.api.nvim_win_close(win, true)
      end
    end,
  })
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
  vim.api.nvim_buf_set_option(buf, 'modifiable', false)
  vim.api.nvim_buf_set_option(buf, 'bufhidden',  'wipe')

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
