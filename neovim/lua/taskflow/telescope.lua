--- Optional Telescope extension for TaskFlow.
-- Load with:  require('telescope').load_extension('taskflow')
local M = {}

local function has_telescope()
  return pcall(require, 'telescope')
end

function M.taskflow(opts)
  if not has_telescope() then
    vim.notify('[TaskFlow] telescope.nvim is required for this command', vim.log.levels.ERROR)
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
        local status_sym = ({
          todo = '[ ]', in_progress = '[~]', blocked = '[!]',
          done = '[x]', cancelled = '[-]',
        })[task.status] or '   '
        return {
          value   = task,
          display = status_sym .. '  ' .. (task.title or task.id)
            .. require('taskflow.ui').format_tags(task.tags),
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
