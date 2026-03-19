--- TaskFlow Neovim plugin – public API.
--
-- Quick start:
--   require('taskflow').setup({
--     socket_path = '',      -- auto-resolve from XDG_DATA_HOME
--     timeout_ms  = 5000,
--   })
--
-- Statusline:
--   vim.o.statusline = '%{%v:lua.require("taskflow").statusline()%}'
--
-- Commands registered by setup():
--   :TaskFlowAdd [title]   create task (visual selection → title)
--   :TaskFlowList          browse tasks in a floating window
--   :TaskFlowCapture       batch-create from TODO/FIXME in current buffer
--   :TaskFlowStatus        show focus task + count via vim.notify
--   :TaskFlowOpen          open TUI in a floating terminal

local M = {}

-- Cached statusline string, updated by a timer.
local _statusline_cache = ''
local _statusline_timer = nil

--- Run setup: merge config, register commands, start statusline refresh timer.
---@param opts table|nil
function M.setup(opts)
  require('taskflow.config').setup(opts)
  local cmds = require('taskflow.commands')

  vim.api.nvim_create_user_command('TaskFlowAdd', cmds.add, {
    nargs = '?', range = true,
    desc  = 'Create a TaskFlow task (visual selection or prompt)',
  })
  vim.api.nvim_create_user_command('TaskFlowList', cmds.list, {
    nargs = 0,
    desc  = 'Browse TaskFlow tasks in a floating window',
  })
  vim.api.nvim_create_user_command('TaskFlowCapture', cmds.capture, {
    nargs = 0,
    desc  = 'Create tasks from TODO/FIXME in the current buffer',
  })
  vim.api.nvim_create_user_command('TaskFlowStatus', cmds.status, {
    nargs = 0,
    desc  = 'Show current TaskFlow focus task and task count',
  })
  vim.api.nvim_create_user_command('TaskFlowOpen', cmds.open, {
    nargs = 0,
    desc  = 'Open the TaskFlow TUI in a floating terminal',
  })

  -- Statusline refresh timer
  local cfg          = require('taskflow.config').get()
  local interval_ms  = (cfg.statusline_refresh_s or 30) * 1000
  local uv           = vim.uv or vim.loop
  _statusline_timer  = uv.new_timer()
  if _statusline_timer then
    _statusline_timer:start(0, interval_ms, vim.schedule_wrap(function()
      M._refresh_statusline()
    end))
  end

  -- Telescope extension registration (optional)
  if cfg.telescope then
    pcall(function()
      require('telescope').register_extension({
        exports = { taskflow = require('taskflow.telescope').taskflow },
      })
    end)
  end
end

--- Refresh the cached statusline string.
function M._refresh_statusline()
  local cfg  = require('taskflow.config')
  local path = cfg.default_socket_path()
  -- Use a very short timeout for statusline to avoid blocking renders
  local ok, resp = require('taskflow.socket').request_sync(
    path, { type = 'status' }, 500)
  if ok and resp and resp.success then
    local d     = resp.data or {}
    local count = d.incomplete_count or 0
    _statusline_cache = count > 0 and (' ' .. count .. ' tasks') or ''
  else
    _statusline_cache = ''
  end
end

--- Return the statusline string (cached; refreshed every N seconds).
-- Returns '' when the TUI is not running.
---@return string
function M.statusline()
  return _statusline_cache
end

--- Low-level async request helper (re-exported for power users).
---@param req_table table
---@param callback  function
function M.request(req_table, callback)
  local cfg  = require('taskflow.config')
  local path = cfg.default_socket_path()
  require('taskflow.socket').request(path, req_table, callback)
end

--- Low-level synchronous request helper.
---@param req_table  table
---@param timeout_ms integer|nil  defaults to config value
---@return boolean ok
---@return table|nil response
---@return string|nil err
function M.request_sync(req_table, timeout_ms)
  local cfg  = require('taskflow.config')
  local path = cfg.default_socket_path()
  local ms   = timeout_ms or cfg.get().timeout_ms
  return require('taskflow.socket').request_sync(path, req_table, ms)
end

return M
