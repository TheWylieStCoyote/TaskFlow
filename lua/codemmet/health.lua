---`:checkhealth codemmet` — verify the whole expansion chain: the binary is
---found and runnable, its version matches this plugin checkout, tree-sitter
---context support is compiled in, the snippet packs validate, and the language
---server is attached.
---
---Tree-sitter probe: the feature has no CLI surface (`codemmet -V` reports
---only the version, and `codemmet expand` never consults buffer context), but
---the LSP server picks its context provider at compile time. So the probe runs
---a scripted one-shot `codemmet lsp` session over stdio and previews the
---builtin `fn` trigger inside a Rust `impl` block: the tree-sitter build sees
---the `type-body` scope and expands the `&self` method variant, while the
---scope-blind null provider expands the plain top-level body.
local M = {}

local health = vim.health

---Wall-clock budget for each spawned `codemmet` process, in milliseconds.
local TIMEOUT_MS = 5000

---Run `cmd` to completion.
---@param cmd string[]
---@param opts? vim.SystemOpts
---@return vim.SystemCompleted|nil result, string|nil spawn_err
local function run(cmd, opts)
  opts = vim.tbl_extend("force", { text = true, timeout = TIMEOUT_MS }, opts or {})
  local ok, proc = pcall(vim.system, cmd, opts)
  if not ok then
    return nil, tostring(proc)
  end
  return proc:wait()
end

---Resolve the codemmet binary the plugin uses: a running client's `cmd[1]`
---when one exists (matching how capture.lua locates the CLI), else the
---configured `cmd`, else the default PATH lookup.
---@return string|nil binary, string source how the binary was resolved
local function resolve_binary()
  local client = vim.lsp.get_clients({ name = "codemmet" })[1]
  if client then
    local cmd = client.config.cmd
    if type(cmd) == "table" and type(cmd[1]) == "string" then
      return cmd[1], "running LSP client"
    end
    return nil, "the running client's cmd is not a string list"
  end
  local config = require("codemmet").config
  if config and type(config.cmd) == "table" and type(config.cmd[1]) == "string" then
    return config.cmd[1], "setup() cmd"
  end
  return "codemmet", "default cmd (PATH lookup)"
end

---Report whether `binary` exists and is executable.
---@param binary string
---@param source string how the binary was resolved (for the report)
---@return string|nil path the resolved executable path, nil when unusable
local function check_binary(binary, source)
  local path = vim.fn.exepath(binary)
  if path ~= "" then
    health.ok(("binary found: `%s` (via %s)"):format(path, source))
    return path
  end
  local advice = {
    "Install the binary (`cargo install --path crates/codemmet`) or point"
      .. " `setup({ cmd = { '/abs/path/to/codemmet', 'lsp' } })` at a built one.",
    "See docs/installation.md and docs/editors.md.",
  }
  if vim.fn.filereadable(binary) == 1 then
    health.error(("`%s` exists but is not executable (via %s)"):format(binary, source), advice)
  else
    health.error(("codemmet binary not found: `%s` (via %s)"):format(binary, source), advice)
  end
  return nil
end

---The workspace version of the checkout this plugin file lives in (the plugin
---ships inside the codemmet repository), or nil when run from elsewhere.
---@return string|nil
local function plugin_version()
  local source = debug.getinfo(1, "S").source
  if source:sub(1, 1) ~= "@" then
    return nil
  end
  -- <repo>/lua/codemmet/health.lua -> <repo>/Cargo.toml
  local root = vim.fs.dirname(vim.fs.dirname(vim.fs.dirname(source:sub(2))))
  local manifest = root and (root .. "/Cargo.toml")
  if not manifest or vim.fn.filereadable(manifest) ~= 1 then
    return nil
  end
  local text = table.concat(vim.fn.readfile(manifest), "\n")
  return text:match('%[workspace%.package%].-\nversion%s*=%s*"([^"]+)"')
end

---Report the binary's version and compare it with the plugin checkout's.
---@param binary string
local function check_version(binary)
  local result, spawn_err = run({ binary, "--version" })
  if not result or result.code ~= 0 then
    local detail = spawn_err or (result and vim.trim(result.stderr or "")) or ""
    health.error(("`codemmet --version` failed: %s"):format(detail))
    return
  end
  local output = vim.trim(result.stdout or "")
  local version = output:match("^codemmet%s+([%w%.%-%+]+)")
  if not version then
    health.warn(("could not parse the version from `%s`"):format(output))
    return
  end
  local expected = plugin_version()
  if expected and expected ~= version then
    health.warn(
      ("binary is %s but this plugin checkout is %s"):format(version, expected),
      { "The plugin and binary ship from the same repository; rebuild or update them together." }
    )
    return
  end
  local suffix = expected and " (matches the plugin checkout)" or ""
  health.ok(("`%s`%s"):format(output, suffix))
end

---One LSP stdio message with its `Content-Length` framing header.
---@param message table
---@return string
local function frame(message)
  local body = vim.json.encode(message)
  return ("Content-Length: %d\r\n\r\n%s"):format(#body, body)
end

---The scripted LSP session for the tree-sitter probe (see the module doc):
---open a Rust document with `fn` inside an `impl` block and preview it.
---@return string payload the concatenated client -> server byte stream
local function probe_payload()
  local uri = "file:///codemmet-health-probe.rs"
  local messages = {
    {
      jsonrpc = "2.0",
      id = 1,
      method = "initialize",
      params = { capabilities = vim.empty_dict() },
    },
    { jsonrpc = "2.0", method = "initialized", params = vim.empty_dict() },
    {
      jsonrpc = "2.0",
      method = "textDocument/didOpen",
      params = {
        textDocument = {
          uri = uri,
          languageId = "rust",
          version = 1,
          text = "impl Foo {\n    fn\n}\n",
        },
      },
    },
    {
      jsonrpc = "2.0",
      id = 2,
      method = "workspace/executeCommand",
      params = {
        command = "codemmet.preview",
        -- Cursor right after the "fn" on line 2 ("    fn" = 6 columns).
        arguments = { { uri = uri, position = { line = 1, character = 6 } } },
      },
    },
    { jsonrpc = "2.0", id = 3, method = "shutdown", params = vim.NIL },
    { jsonrpc = "2.0", method = "exit", params = vim.NIL },
  }
  local parts = {}
  for _, message in ipairs(messages) do
    parts[#parts + 1] = frame(message)
  end
  return table.concat(parts)
end

---Report whether the binary was built with the `tree-sitter` feature.
---@param binary string
local function check_tree_sitter(binary)
  -- `--config` points at a directory that does not exist, so a user override
  -- of `fn` cannot change the probe's outcome.
  local result = run({ binary, "--config", vim.fn.tempname(), "lsp" }, {
    stdin = probe_payload(),
  })
  local stdout = (result and result.stdout) or ""
  if stdout:find("(&self", 1, true) then
    health.ok("tree-sitter context support is compiled in")
  elseif stdout:find('"text"', 1, true) then
    health.warn("the binary is built without the `tree-sitter` feature", {
      "Context awareness degrades silently: scope variants (`&self`/`self`) never fire and"
        .. " completion is not suppressed inside strings and comments.",
      "Rebuild with default features (`cargo install --path crates/codemmet`)."
        .. " See docs/troubleshooting.md.",
    })
  else
    health.warn("could not determine tree-sitter support (the probe returned no preview)")
  end
end

---Validate the snippet packs with `codemmet check`.
---@param binary string
local function check_packs(binary)
  local result, spawn_err = run({ binary, "check" })
  if not result then
    health.error(("could not run `codemmet check`: %s"):format(spawn_err))
    return
  end
  local stdout = vim.trim(result.stdout or "")
  local stderr = vim.trim(result.stderr or "")
  if result.code == 0 then
    health.ok("`codemmet check`: " .. (stdout ~= "" and stdout or "ok"))
    return
  end
  health.error(
    ("`codemmet check` failed (exit %d): %s"):format(result.code, stderr ~= "" and stderr or stdout),
    {
      "Fix the reported pack, or move it aside; broken packs are skipped but their"
        .. " triggers are missing until then.",
    }
  )
end

---Report setup state, the configured filetypes, and client attach status.
local function check_lsp()
  local config = require("codemmet").config
  if config then
    health.ok("setup() called; filetypes: " .. table.concat(config.filetypes or {}, ", "))
  else
    health.warn("require('codemmet').setup() has not been called", {
      "The language server only starts (and keymaps are only installed) after setup().",
    })
  end

  local clients = vim.lsp.get_clients({ name = "codemmet" })
  if #clients == 0 then
    health.info(
      "no codemmet LSP client is running"
        .. " (one starts when a buffer with a configured filetype opens)"
    )
    return
  end
  for _, client in ipairs(clients) do
    local buffers = {}
    for bufnr in pairs(client.attached_buffers) do
      local name = vim.fn.fnamemodify(vim.api.nvim_buf_get_name(bufnr), ":~:.")
      buffers[#buffers + 1] = ("%s [%s]"):format(name ~= "" and name or "?", vim.bo[bufnr].filetype)
    end
    table.sort(buffers)
    health.ok(
      ("client #%d is running, attached to %d buffer(s): %s"):format(
        client.id,
        #buffers,
        table.concat(buffers, ", ")
      )
    )
  end
end

---Report whether built-in LSP completion can be — and is — wired up for the
---codemmet client, and the effective server-side completion gate.
local function check_completion()
  local completion = require("codemmet.completion")
  local nvim = tostring(vim.version())
  if completion.supported() then
    health.ok("Neovim " .. nvim .. " supports built-in LSP completion (vim.lsp.completion)")
  else
    health.warn("Neovim " .. nvim .. " cannot auto-enable built-in LSP completion", {
      "upgrade to Neovim 0.11+, or use a completion plugin (nvim-cmp, blink.cmp)",
    })
  end
  local config = require("codemmet").config
  if not config then
    return
  end
  if not config.completion.enabled then
    health.ok("completion wiring disabled (`completion.enabled = false`)")
  else
    local reason = completion.skip_reason()
    if reason then
      health.ok("completion auto-enable skipped: " .. reason)
    else
      health.ok(
        ("built-in LSP completion enabled on attach (autotrigger = %s)"):format(
          tostring(config.completion.autotrigger)
        )
      )
    end
  end
  local mode = config.init_options.completion or "always (server default)"
  health.info("server completion gate: " .. mode)
end

---The health-check entry point `:checkhealth codemmet` calls.
function M.check()
  health.start("codemmet binary")
  local binary, source = resolve_binary()
  local path = binary and check_binary(binary, source)
  if binary and not path then
    health.info("skipping the version, tree-sitter, and snippet pack checks")
  elseif not binary then
    health.error("cannot determine the codemmet binary: " .. source)
  else
    check_version(path)
    check_tree_sitter(path)
    check_packs(path)
  end

  health.start("codemmet completion")
  check_completion()

  health.start("codemmet language server")
  check_lsp()
end

return M
