" TaskFlow Neovim plugin autoload shim.
" This file only guards against double-loading. Actual setup is done via
"   require('taskflow').setup(opts)
" in the user's init.lua / init.vim.
if exists('g:loaded_taskflow')
  finish
endif
let g:loaded_taskflow = 1
