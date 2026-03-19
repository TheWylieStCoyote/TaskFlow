--- Tests for taskflow.config
describe('config', function()
  before_each(function()
    -- Reset module cache before each test
    package.loaded['taskflow.config'] = nil
  end)

  it('default_socket_path contains taskflow.sock', function()
    local config = require('taskflow.config')
    local path = config.default_socket_path()
    assert.truthy(path:find('taskflow.sock', 1, true))
  end)

  it('respects XDG_DATA_HOME override', function()
    local saved = os.getenv('XDG_DATA_HOME')
    -- Set env var if supported (vim.env)
    if vim.env then
      vim.env.XDG_DATA_HOME = '/tmp/test_xdg'
      package.loaded['taskflow.config'] = nil
      local config = require('taskflow.config')
      local path = config.default_socket_path()
      assert.truthy(path:find('/tmp/test_xdg', 1, true))
      -- Restore
      vim.env.XDG_DATA_HOME = saved or ''
    end
  end)

  it('setup() deep-merges options', function()
    local config = require('taskflow.config')
    config.setup({ timeout_ms = 9999 })
    local cfg = config.get()
    assert.equals(9999, cfg.timeout_ms)
    -- Other defaults should be preserved
    assert.equals(30, cfg.statusline_refresh_s)
  end)

  it('socket_path config override takes precedence', function()
    local config = require('taskflow.config')
    config.setup({ socket_path = '/custom/path/tf.sock' })
    local path = config.default_socket_path()
    assert.equals('/custom/path/tf.sock', path)
  end)
end)
