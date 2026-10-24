-- Minimal Neovim init for running plenary.nvim tests.
-- Usage:
--   nvim --headless -c "PlenaryBustedDirectory neovim/tests/ {minimal_init = 'neovim/tests/minimal_init.lua'}"

-- Add the plugin root to runtimepath so `require('taskflow.*')` works.
local root = vim.fn.fnamemodify(debug.getinfo(1).source:sub(2), ':p:h:h:h')
vim.opt.rtp:prepend(root .. '/neovim')

-- Add plenary: $PLENARY_DIR first (CI checks it out there), then the usual
-- plugin-manager locations.
local plenary_env = vim.env.PLENARY_DIR
local plenary_path = vim.fn.stdpath('data') .. '/lazy/plenary.nvim'
if plenary_env and plenary_env ~= '' and vim.fn.isdirectory(plenary_env) == 1 then
  vim.opt.rtp:prepend(plenary_env)
elseif vim.fn.isdirectory(plenary_path) == 1 then
  vim.opt.rtp:prepend(plenary_path)
else
  -- Try common plugin manager locations
  for _, path in ipairs({
    vim.fn.expand('~/.local/share/nvim/site/pack/*/start/plenary.nvim'),
    vim.fn.expand('~/.vim/plugged/plenary.nvim'),
  }) do
    local expanded = vim.fn.glob(path)
    if expanded ~= '' then
      vim.opt.rtp:prepend(expanded)
      break
    end
  end
end
