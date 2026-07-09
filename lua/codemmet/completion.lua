---Auto-wiring for Neovim's built-in LSP completion. On LspAttach the plugin
---enables `vim.lsp.completion` for the codemmet client so expansions pop up as
---you type, unless a completion engine (nvim-cmp, blink.cmp) is already active
---or the running Neovim lacks support. `:checkhealth codemmet` explains which
---of these applies.
local M = {}

---Whether this Neovim can auto-enable built-in LSP completion.
---`vim.lsp.completion.enable(true, client_id, bufnr, { autotrigger })` landed in
---Neovim 0.11; on 0.10 (the plugin's floor) the wiring quietly no-ops and the
---health check points at the version.
---@return boolean
function M.supported()
  return vim.fn.has("nvim-0.11") == 1 and vim.lsp.completion ~= nil
end

---Name of a loaded completion engine that already consumes LSP sources, or nil.
---Detection reads `package.loaded` instead of pcall(require, ...): under lazy.nvim
---a require() force-loads an installed-but-inactive plugin (its module hook), while
---`package.loaded` only sees plugins actually running in this session — and only a
---running engine can produce a double completion menu. The check runs on every
---LspAttach, so an engine that loads before a codemmet buffer opens is caught.
---@return string|nil
function M.engine()
  if package.loaded.cmp ~= nil then
    return "nvim-cmp"
  end
  if package.loaded["blink.cmp"] ~= nil then
    return "blink.cmp"
  end
  return nil
end

---Why auto-enable would be skipped right now, or nil when it can go ahead.
---@return string|nil
function M.skip_reason()
  local engine = M.engine()
  if engine then
    return engine .. " is active and already surfaces codemmet's LSP completions"
  end
  if not M.supported() then
    return "built-in LSP completion (vim.lsp.completion) needs Neovim 0.11+"
  end
  return nil
end

---Enable built-in LSP completion for the codemmet client on this buffer, unless
---the user disabled it, an engine is active, or the Neovim version lacks support.
---@param client vim.lsp.Client
---@param bufnr integer
---@param opts codemmet.CompletionOpts resolved completion options from setup()
function M.attach(client, bufnr, opts)
  if not opts.enabled or M.skip_reason() ~= nil then
    return
  end
  vim.lsp.completion.enable(true, client.id, bufnr, { autotrigger = opts.autotrigger })
end

return M
