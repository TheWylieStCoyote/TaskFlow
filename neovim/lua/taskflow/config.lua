--- Configuration defaults and resolution for the TaskFlow Neovim plugin.
local M = {}

--- Default configuration values.
M.defaults = {
  -- Path to the Unix domain socket; nil means auto-resolve from XDG_DATA_HOME
  socket_path = nil,
  -- Timeout for synchronous requests in milliseconds
  timeout_ms = 5000,
  -- Statusline cache refresh interval in seconds
  statusline_refresh_s = 30,
  -- Floating window dimensions (fraction of editor size)
  float_width = 0.8,
  float_height = 0.7,
  -- Telescope integration: enable if telescope.nvim is available
  telescope = true,
}

local _config = vim.deepcopy(M.defaults)

--- Return the resolved socket path.
-- Checks (in order):
--   1. config.socket_path (user override)
--   2. $XDG_DATA_HOME/taskflow/taskflow.sock
--   3. ~/.local/share/taskflow/taskflow.sock
---@return string
function M.default_socket_path()
  if _config.socket_path and _config.socket_path ~= '' then
    return _config.socket_path
  end
  local xdg = os.getenv('XDG_DATA_HOME')
  if xdg and xdg ~= '' then
    return xdg .. '/taskflow/taskflow.sock'
  end
  local home = os.getenv('HOME') or vim.fn.expand('~')
  return home .. '/.local/share/taskflow/taskflow.sock'
end

--- Merge user options into the active configuration.
---@param opts table|nil
function M.setup(opts)
  if opts then
    _config = vim.tbl_deep_extend('force', _config, opts)
  end
end

--- Return the active configuration.
---@return table
function M.get()
  return _config
end

return M
