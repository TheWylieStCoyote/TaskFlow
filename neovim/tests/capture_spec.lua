--- Tests for taskflow.capture
local capture = require('taskflow.capture')

describe('capture.scan_buffer', function()
  local function make_buf(lines)
    local buf = vim.api.nvim_create_buf(false, true)
    vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
    return buf
  end

  it('extracts TODO comment text', function()
    local buf = make_buf({ '-- TODO: fix auth bug' })
    local items = capture.scan_buffer(buf)
    assert.equals(1, #items)
    assert.equals('fix auth bug', items[1].text)
    assert.equals('TODO', items[1].pattern)
    vim.api.nvim_buf_delete(buf, { force = true })
  end)

  it('extracts FIXME comment text', function()
    local buf = make_buf({ '// FIXME remove this hack' })
    local items = capture.scan_buffer(buf)
    assert.equals(1, #items)
    assert.equals('remove this hack', items[1].text)
    assert.equals('FIXME', items[1].pattern)
    vim.api.nvim_buf_delete(buf, { force = true })
  end)

  it('extracts HACK comment text', function()
    local buf = make_buf({ '# HACK: workaround for upstream bug' })
    local items = capture.scan_buffer(buf)
    assert.equals(1, #items)
    assert.equals('workaround for upstream bug', items[1].text)
    assert.equals('HACK', items[1].pattern)
    vim.api.nvim_buf_delete(buf, { force = true })
  end)

  it('extracts NOTE comment text', function()
    local buf = make_buf({ '-- NOTE this is important' })
    local items = capture.scan_buffer(buf)
    assert.equals(1, #items)
    assert.equals('this is important', items[1].text)
    assert.equals('NOTE', items[1].pattern)
    vim.api.nvim_buf_delete(buf, { force = true })
  end)

  it('produces no false positives on normal code', function()
    local buf = make_buf({
      'local x = 1',
      'function foo() end',
      '-- regular comment',
    })
    local items = capture.scan_buffer(buf)
    assert.equals(0, #items)
    vim.api.nvim_buf_delete(buf, { force = true })
  end)

  it('handles multiple matches in a buffer', function()
    local buf = make_buf({
      '-- TODO: first task',
      'local x = 1',
      '-- FIXME: second task',
    })
    local items = capture.scan_buffer(buf)
    assert.equals(2, #items)
    vim.api.nvim_buf_delete(buf, { force = true })
  end)

  it('trims surrounding whitespace from extracted text', function()
    local buf = make_buf({ '-- TODO:   lots of spaces   ' })
    local items = capture.scan_buffer(buf)
    assert.equals(1, #items)
    assert.equals('lots of spaces', items[1].text)
    vim.api.nvim_buf_delete(buf, { force = true })
  end)
end)
