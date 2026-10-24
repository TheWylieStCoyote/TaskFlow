--- Tests for idempotent setup() and the async statusline refresh.
describe('setup / statusline', function()
  local socket

  before_each(function()
    package.loaded['taskflow'] = nil
    package.loaded['taskflow.config'] = nil
    socket = require('taskflow.socket')
  end)

  it('setup twice leaves exactly one live timer and the commands registered', function()
    local tf = require('taskflow')
    tf.setup({ socket_path = '/tmp/nonexistent_test.sock', statusline_refresh_s = 3600 })
    local first = tf._timer()
    assert.truthy(first)
    tf.setup({ socket_path = '/tmp/nonexistent_test.sock', statusline_refresh_s = 1800 })
    local second = tf._timer()
    assert.truthy(second)
    assert.are_not.equal(first, second)
    assert.is_true(first:is_closing())
    assert.is_false(second:is_closing())
    assert.equals(1800, require('taskflow.config').get().statusline_refresh_s)
    local cmds = vim.api.nvim_get_commands({})
    for _, n in ipairs({ 'TaskFlowAdd', 'TaskFlowList', 'TaskFlowCapture', 'TaskFlowStatus', 'TaskFlowOpen' }) do
      assert.truthy(cmds[n], n)
    end
  end)

  describe('_refresh_statusline', function()
    local orig_request, calls, pending

    before_each(function()
      orig_request = socket.request
      calls, pending = 0, nil
      socket.request = function(_, _, cb) calls = calls + 1; pending = cb end
    end)
    after_each(function() socket.request = orig_request end)

    it('updates the cache from the callback and fires User TaskFlowStatus', function()
      local tf = require('taskflow')
      local got
      local id = vim.api.nvim_create_autocmd('User', {
        pattern = 'TaskFlowStatus',
        callback = function(ev) got = ev.data end,
      })
      assert.equals('', tf.statusline())
      tf._refresh_statusline()
      assert.equals('', tf.statusline()) -- nothing until the callback runs
      pending(true, { success = true, data = { incomplete_count = 4, focus_task = { title = 'x' } } })
      vim.api.nvim_del_autocmd(id)
      assert.equals(' 4 tasks', tf.statusline())
      assert.truthy(got)
      assert.equals(4, got.incomplete_count)
      assert.equals('x', got.focus_task.title)
    end)

    it('does not issue a second request while one is in flight', function()
      local tf = require('taskflow')
      tf._refresh_statusline()
      tf._refresh_statusline()
      assert.equals(1, calls)
      pending(false, nil, 'boom')
      tf._refresh_statusline()
      assert.equals(2, calls)
      pending(false, nil, 'boom')
    end)
  end)
end)
