--- Optional Telescope extension for TaskFlow.
-- Load with:  require('telescope').load_extension('taskflow')
local M = {}

local function has_telescope()
  return pcall(require, 'telescope')
end

local STATUS_SYM = {
  todo = '[ ]', in_progress = '[~]', blocked = '[!]',
  done = '[x]', cancelled = '[-]',
}

--- Display text shared by the Telescope entries and the vim.ui.select fallback.
local function display_text(task)
  return (STATUS_SYM[task.status] or '   ') .. '  ' .. (task.title or task.id)
    .. require('taskflow.ui').format_tags(task.tags)
end

--- Fallback picker when telescope.nvim is absent.
local function select_fallback(tasks, path, timeout_ms)
  vim.ui.select(tasks, {
    prompt = 'TaskFlow',
    format_item = display_text,
  }, function(task)
    if task then
      require('taskflow.socket').request_sync(
        path, { type = 'focus_task', task_id = task.id }, timeout_ms)
    end
  end)
end

function M.taskflow(opts)
  local telescope_ok = has_telescope()
  if not telescope_ok then
    local cfg_ = require('taskflow.config')
    local path_, timeout_ = cfg_.default_socket_path(), cfg_.get().timeout_ms
    local ok_, resp_, err_ = require('taskflow.socket').request_sync(
      path_, { type = 'pipe', operation = 'list', entity = 'task' }, timeout_)
    if not ok_ or not resp_ or not resp_.success then
      vim.notify('[TaskFlow] ' .. (err_ or 'failed to list tasks'), vim.log.levels.ERROR)
      return
    end
    select_fallback(resp_.data or {}, path_, timeout_)
    return
  end

  local pickers     = require('telescope.pickers')
  local finders     = require('telescope.finders')
  local conf        = require('telescope.config').values
  local actions     = require('telescope.actions')
  local action_state = require('telescope.actions.state')

  local cfg        = require('taskflow.config')
  local socket_mod = require('taskflow.socket')
  local path       = cfg.default_socket_path()
  local timeout_ms = cfg.get().timeout_ms

  local ok, resp, err = socket_mod.request_sync(
    path, { type = 'pipe', operation = 'list', entity = 'task' }, timeout_ms)

  if not ok or not resp or not resp.success then
    vim.notify('[TaskFlow] ' .. (err or 'failed to list tasks'), vim.log.levels.ERROR)
    return
  end

  local tasks = resp.data or {}

  pickers.new(opts or {}, {
    prompt_title = 'TaskFlow',
    finder = finders.new_table({
      results = tasks,
      entry_maker = function(task)
        return {
          value   = task,
          display = display_text(task),
          ordinal = (task.title or task.id) .. ' ' .. table.concat(task.tags or {}, ' '),
        }
      end,
    }),
    sorter = conf.generic_sorter(opts or {}),
    attach_mappings = function(prompt_bufnr, map)
      -- <CR>: focus task in TUI
      actions.select_default:replace(function()
        actions.close(prompt_bufnr)
        local sel = action_state.get_selected_entry()
        if sel then
          socket_mod.request_sync(path, { type = 'focus_task', task_id = sel.value.id }, timeout_ms)
        end
      end)

      -- <C-d>: delete task
      map('i', '<C-d>', function()
        actions.close(prompt_bufnr)
        local sel = action_state.get_selected_entry()
        if sel then
          socket_mod.request_sync(path, {
            type = 'pipe', operation = 'delete', entity = 'task',
            id   = sel.value.id,
          }, timeout_ms)
        end
      end)

      -- <C-c>: create new task (prompt)
      map('i', '<C-c>', function()
        local title = vim.fn.input('Task title: ')
        if title ~= '' then
          socket_mod.request_sync(path, {
            type = 'pipe', operation = 'create', entity = 'task',
            data = { title = title },
          }, timeout_ms)
        end
        actions.close(prompt_bufnr)
      end)

      return true
    end,
  }):find()
end

return M
