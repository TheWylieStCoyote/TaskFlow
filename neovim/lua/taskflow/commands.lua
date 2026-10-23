--- Implementation of :TaskFlow* user commands.
local M = {}

local function get_socket()
  local cfg = require('taskflow.config')
  return cfg.default_socket_path(), cfg.get().timeout_ms
end

local function socket_request(req)
  local path, timeout_ms = get_socket()
  return require('taskflow.socket').request_sync(path, req, timeout_ms)
end

--- :TaskFlowAdd [title]
-- If title is empty, opens an input prompt.
-- In visual mode the selected text is used as the title.
---@param opts table  nvim_create_user_command opts table
function M.add(opts)
  local title = opts.args and opts.args ~= '' and opts.args or nil

  -- Visual selection
  if not title and opts.range == 2 then
    local lines = vim.api.nvim_buf_get_lines(
      0, opts.line1 - 1, opts.line2, false)
    title = table.concat(lines, ' '):match('^%s*(.-)%s*$')
  end

  -- Prompt if still empty
  if not title or title == '' then
    title = vim.fn.input('Task title: ')
  end
  if not title or title == '' then return end

  local ok, resp, err = socket_request({
    type      = 'pipe',
    operation = 'create',
    entity    = 'task',
    data      = { title = title },
  })
  if ok and resp and resp.success then
    vim.notify('[TaskFlow] Task created: ' .. title, vim.log.levels.INFO)
  else
    vim.notify('[TaskFlow] Failed to create task: ' .. (err or 'unknown'), vim.log.levels.ERROR)
  end
end

--- :TaskFlowList
function M.list(_opts)
  local ok, resp, err = socket_request({
    type      = 'pipe',
    operation = 'list',
    entity    = 'task',
  })
  if not ok or not resp or not resp.success then
    vim.notify('[TaskFlow] ' .. (err or 'list failed'), vim.log.levels.ERROR)
    return
  end

  local tasks = resp.data or {}
  require('taskflow.ui').open_task_list(tasks, function(task)
    -- Focus the selected task in the TUI
    local fok, fresp, ferr = socket_request({ type = 'focus_task', task_id = task.id })
    if fok and fresp and fresp.success then
      vim.notify('[TaskFlow] Focused: ' .. task.title, vim.log.levels.INFO)
    else
      vim.notify('[TaskFlow] Focus failed: ' .. (ferr or '?'), vim.log.levels.WARN)
    end
  end)
end

--- :TaskFlowCapture
-- Extracts TODO/FIXME annotations from the current buffer and batch-creates tasks.
function M.capture(_opts)
  local items = require('taskflow.capture').scan_buffer(0)
  if #items == 0 then
    vim.notify('[TaskFlow] No TODO/FIXME annotations found', vim.log.levels.INFO)
    return
  end

  local created = 0
  local failed  = 0
  for _, item in ipairs(items) do
    local ok, resp = socket_request({
      type      = 'pipe',
      operation = 'create',
      entity    = 'task',
      data      = { title = item.text, tags = { 'captured', item.pattern:lower() } },
    })
    if ok and resp and resp.success then
      created = created + 1
    else
      failed = failed + 1
    end
  end

  vim.notify(
    string.format('[TaskFlow] Captured %d task(s) (%d failed)', created, failed),
    failed > 0 and vim.log.levels.WARN or vim.log.levels.INFO
  )
end

--- :TaskFlowStatus
function M.status(_opts)
  local ok, resp, err = socket_request({ type = 'status' })
  if not ok or not resp or not resp.success then
    vim.notify('[TaskFlow] ' .. (err or 'status failed'), vim.log.levels.ERROR)
    return
  end
  local d = resp.data or {}
  local msg = string.format(
    '[TaskFlow] %d incomplete tasks',
    d.incomplete_count or 0
  )
  if d.focus_task and d.focus_task ~= vim.NIL then
    msg = msg .. '  |  Focus: ' .. (d.focus_task.title or d.focus_task.id)
  end
  vim.notify(msg, vim.log.levels.INFO)
end

--- :TaskFlowOpen
-- Opens the TaskFlow TUI in a floating terminal window.
function M.open(_opts)
  require('taskflow.ui').open_float_terminal('taskflow')
end

return M
