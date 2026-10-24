--- Tests for :TaskFlow* command registration
describe('commands', function()
  before_each(function()
    -- Reset module state
    package.loaded['taskflow']          = nil
    package.loaded['taskflow.config']   = nil
    package.loaded['taskflow.commands'] = nil
  end)

  it('registers all user commands after setup()', function()
    -- Use a non-existent socket so no real I/O happens
    require('taskflow').setup({ socket_path = '/tmp/nonexistent_test.sock' })

    local cmds = vim.api.nvim_get_commands({})
    assert.truthy(cmds['TaskFlowAdd'],     'TaskFlowAdd should be registered')
    assert.truthy(cmds['TaskFlowList'],    'TaskFlowList should be registered')
    assert.truthy(cmds['TaskFlowCapture'], 'TaskFlowCapture should be registered')
    assert.truthy(cmds['TaskFlowStatus'],  'TaskFlowStatus should be registered')
    assert.truthy(cmds['TaskFlowOpen'],    'TaskFlowOpen should be registered')
  end)

  it('TaskFlowAdd accepts a range (visual mode)', function()
    require('taskflow').setup({ socket_path = '/tmp/nonexistent_test.sock' })
    local cmds = vim.api.nvim_get_commands({})
    assert.truthy(cmds['TaskFlowAdd'])
    -- range = 1 means the command accepts a range
    assert.truthy(cmds['TaskFlowAdd'].range ~= '')
  end)
end)
