--- Tests for taskflow.ui rendering helpers.
describe('ui', function()
  local ui = require('taskflow.ui')

  it('format_tags renders hash-prefixed tags', function()
    assert.equals('  #captured #todo', ui.format_tags({ 'captured', 'todo' }))
  end)

  it('format_tags is empty for no tags', function()
    assert.equals('', ui.format_tags(nil))
    assert.equals('', ui.format_tags({}))
  end)

  it('open_task_list shows title and tags on one line', function()
    ui.open_task_list({
      { id = '1', title = 'Fix parser', status = 'todo', priority = 'high', tags = { 'captured', 'fixme' } },
      { id = '2', title = 'No tags here', status = 'done', priority = 'none' },
    }, function() end)
    local lines = vim.api.nvim_buf_get_lines(0, 0, -1, false)
    assert.equals(2, #lines)
    assert.truthy(lines[1]:find('Fix parser  #captured #fixme', 1, true), lines[1])
    assert.truthy(lines[2]:find('No tags here', 1, true), lines[2])
    assert.falsy(lines[2]:find('#', 1, true))
    vim.api.nvim_win_close(0, true)
  end)
end)
