---`:checkhealth codemmet` — reports the server binary, whether this Neovim can
---auto-enable built-in LSP completion, and whether the plugin's completion
---wiring is active or skipped (and why).
local M = {}

function M.check()
  vim.health.start("codemmet")

  local config = require("codemmet")._config
  if not config then
    vim.health.warn('require("codemmet").setup() has not been called', {
      "call setup() (your plugin manager's `opts` table does this) and re-run checkhealth",
    })
  end

  -- Server binary.
  local bin = (config and config.cmd or { "codemmet" })[1]
  if vim.fn.executable(bin) == 1 then
    local version = vim.trim(vim.fn.system({ bin, "--version" }))
    if vim.v.shell_error == 0 then
      vim.health.ok(("`%s` found (%s)"):format(bin, version))
    else
      vim.health.warn(("`%s` found but `%s --version` failed"):format(bin, bin))
    end
  else
    vim.health.error(("`%s` is not executable"):format(bin), {
      "run `cargo install --path crates/codemmet`, or point `cmd` at the built binary",
    })
  end

  -- Built-in completion support in this Neovim.
  local completion = require("codemmet.completion")
  local nvim = tostring(vim.version())
  if completion.supported() then
    vim.health.ok("Neovim " .. nvim .. " supports built-in LSP completion (vim.lsp.completion)")
  else
    vim.health.warn("Neovim " .. nvim .. " cannot auto-enable built-in LSP completion", {
      "upgrade to Neovim 0.11+, or use a completion plugin (nvim-cmp, blink.cmp)",
    })
  end

  -- Completion wiring status.
  if not config then
    return
  end
  if not config.completion.enabled then
    vim.health.ok("completion wiring disabled (`completion.enabled = false`)")
  else
    local reason = completion.skip_reason()
    if reason then
      vim.health.ok("completion auto-enable skipped: " .. reason)
    else
      vim.health.ok(
        ("built-in LSP completion enabled on attach (autotrigger = %s)"):format(
          tostring(config.completion.autotrigger)
        )
      )
    end
  end
  local mode = config.init_options.completion or "always (server default)"
  vim.health.info("server completion gate: " .. mode)
end

return M
