--- Tests for :TaskFlowOpen reuse logic, deprecated-API cleanup, picker fallback.
describe('TaskFlowOpen', function()
  local cmds, ui, socket, notes, spawned
  local orig = {}

  before_each(function()
    package.loaded['taskflow.commands'] = nil
    package.loaded['taskflow.ui'] = nil
    cmds, ui, socket = require('taskflow.commands'), require('taskflow.ui'), require('taskflow.socket')
    orig.sync, orig.notify = socket.request_sync, vim.notify
    notes, spawned = {}, 0
    vim.notify = function(m) notes[#notes + 1] = m end
    ui.open_float_terminal = function() spawned = spawned + 1 end
  end)
  after_each(function()
    socket.request_sync, vim.notify = orig.sync, orig.notify
  end)

  it('does not spawn and notifies when ping is answered', function()
    socket.request_sync = function() return true, { success = true } end
    cmds.open()
    assert.equals(0, spawned)
    assert.equals(1, #notes)
    assert.truthy(notes[1]:find('already running', 1, true))
  end)

  it('spawns when nothing answers', function()
    socket.request_sync = function() return false, nil, 'connect' end
    cmds.open()
    assert.equals(1, spawned)
  end)

  it('hides the float when it is open', function()
    local hidden = 0
    ui.float_is_open = function() return true end
    ui.hide_float = function() hidden = hidden + 1 end
    socket.request_sync = function() error('must not ping') end
    cmds.open()
    assert.equals(1, hidden)
    assert.equals(0, spawned)
  end)

  it('reopens a hidden float buffer', function()
    local shown = 0
    ui.float_is_open = function() return false end
    ui.float_is_hidden = function() return true end
    ui.show_float = function() shown = shown + 1 end
    socket.request_sync = function() error('must not ping') end
    cmds.open()
    assert.equals(1, shown)
    assert.equals(0, spawned)
  end)

  it('real ui state starts with no float open or hidden', function()
    package.loaded['taskflow.ui'] = nil
    local real = require('taskflow.ui')
    assert.is_false(real.float_is_open())
    assert.is_false(real.float_is_hidden())
  end)
end)

describe('ui deprecated APIs', function()
  it('has no nvim_buf_set_option / nvim_win_set_option and termopen only as fallback', function()
    local path = vim.api.nvim_get_runtime_file('lua/taskflow/ui.lua', false)[1]
    local src = table.concat(vim.fn.readfile(path), '\n')
    assert.is_nil(src:find('nvim_buf_set_option', 1, true))
    assert.is_nil(src:find('nvim_win_set_option', 1, true))
    for line in src:gmatch('[^\n]+') do
      if line:find('termopen(', 1, true) then
        assert.truthy(line:find('0.10 fallback', 1, true), line)
      end
    end
  end)

  it('open_task_list still sets buffer options', function()
    require('taskflow.ui').open_task_list({ { id = '1', title = 'a', status = 'todo' } }, function() end)
    local buf = vim.api.nvim_get_current_buf()
    assert.is_false(vim.bo[buf].modifiable)
    assert.equals('wipe', vim.bo[buf].bufhidden)
    vim.api.nvim_win_close(0, true)
  end)
end)

describe('telescope picker fallback', function()
  it('uses vim.ui.select with one entry per task when telescope is missing', function()
    local saved_loaded, saved_preload = package.loaded.telescope, package.preload.telescope
    package.loaded.telescope = nil
    package.preload.telescope = function() error('no telescope') end
    local socket = require('taskflow.socket')
    local orig_sync, orig_select = socket.request_sync, vim.ui.select
    socket.request_sync = function()
      return true, { success = true, data = {
        { id = 'a', title = 'One', status = 'todo', tags = { 't' } },
        { id = 'b', title = 'Two', status = 'done' },
      } }
    end
    local items, fmt
    vim.ui.select = function(i, o) items, fmt = i, o.format_item end
    local ok, err = pcall(function() require('taskflow.telescope').taskflow({}) end)
    package.loaded.telescope, package.preload.telescope = saved_loaded, saved_preload
    socket.request_sync, vim.ui.select = orig_sync, orig_select
    assert.is_true(ok, err)
    assert.equals(2, #items)
    assert.equals('[ ]  One  #t', fmt(items[1]))
    assert.equals('[x]  Two', fmt(items[2]))
  end)
end)
