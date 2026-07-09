---codemmet.nvim — Emmet-style abbreviation expansion for code, backed by the
---`codemmet lsp` language server. Requires Neovim >= 0.10.
local M = {}

---@class codemmet.KeymapOpts
---@field expand string|false insert- and normal-mode mapping for expand(); false disables
---@field wrap string|false visual-mode mapping for wrap(); false disables

---@class codemmet.PreviewOpts
---@field enabled? boolean turn the preview on when the server attaches (default true for the table form)
---@field debounce_ms? integer typing-pause before the server is queried, in ms (default 60)

---@class codemmet.CompletionOpts
---@field enabled? boolean auto-enable built-in LSP completion on attach (default true)
---@field autotrigger? boolean open the completion menu while typing (default true)
---@field mode? "always"|"operators-only"|"off" server-side completion gate; nil keeps the server default

---@class codemmet.SetupOpts
---@field cmd? string[] command used to start the language server
---@field filetypes? string[] filetypes the server is started for
---@field keymaps? codemmet.KeymapOpts
---@field completion? boolean|codemmet.CompletionOpts built-in completion wiring; a boolean is shorthand for `{ enabled = ... }`
---@field init_options? table LSP initializationOptions ({ snippets_dir?, completion? })
---@field preview? boolean|codemmet.PreviewOpts live ghost-line preview while typing (default false); `true` ≡ `{ enabled = true, debounce_ms = 60 }`
---@field notes_file? string where `:CodemmetNote` appends (default `<data>/codemmet/notes.md`)

local defaults = {
  cmd = { "codemmet", "lsp" },
  filetypes = {
    "rust",
    "python",
    "c",
    "cpp",
    "javascript",
    "javascriptreact",
    "typescript",
    "typescriptreact",
    "go",
  },
  keymaps = { expand = "<A-e>", wrap = "<leader>cw" },
  completion = { enabled = true, autotrigger = true },
  init_options = {},
  preview = false,
}

---The active configuration: `defaults` merged with the last `setup()` opts.
local config = vim.deepcopy(defaults)

---Expand the abbreviation immediately before the cursor.
function M.expand()
  require("codemmet.lsp").expand()
end

---Wrap the visual selection (or `opts.line1..opts.line2`) in an abbreviation.
---@param opts? codemmet.WrapOpts
function M.wrap(opts)
  require("codemmet.wrap").wrap(opts)
end

---Capture the visual selection (or `opts.line1..opts.line2`) into the project
---snippet pack as a reusable, parameterized snippet.
---@param opts? codemmet.CaptureOpts
function M.capture(opts)
  require("codemmet.capture").capture(opts)
end

local PREVIEW_DEBOUNCE_DEFAULT = 60

---Normalize the `preview` option into its table form, warning (once, at setup)
---about invalid values. Booleans are shorthand: `true` ≡ `{ enabled = true }`,
---`false`/nil ≡ `{ enabled = false }`. In the table form `enabled` defaults to
---true (passing a table opts in), and a `debounce_ms` that is not a positive
---integer falls back to the default of 60 ms.
---@param value boolean|codemmet.PreviewOpts|nil
---@return { enabled: boolean, debounce_ms: integer }
local function normalize_preview(value)
  if type(value) ~= "table" then
    if value ~= nil and type(value) ~= "boolean" then
      vim.notify(
        "codemmet: `preview` must be a boolean or a table (got "
          .. type(value)
          .. "); preview disabled",
        vim.log.levels.WARN
      )
    end
    return { enabled = value == true, debounce_ms = PREVIEW_DEBOUNCE_DEFAULT }
  end
  local ms = value.debounce_ms
  if ms == nil then
    ms = PREVIEW_DEBOUNCE_DEFAULT
  elseif type(ms) ~= "number" or ms <= 0 or ms % 1 ~= 0 then
    vim.notify(
      ("codemmet: `preview.debounce_ms` must be a positive integer (got %s); using %d"):format(
        vim.inspect(value.debounce_ms),
        PREVIEW_DEBOUNCE_DEFAULT
      ),
      vim.log.levels.WARN
    )
    ms = PREVIEW_DEBOUNCE_DEFAULT
  end
  return { enabled = value.enabled ~= false, debounce_ms = ms }
end

---Resolved setup() configuration; read by `:checkhealth codemmet`. Nil until setup runs.
---@type table|nil
M._config = nil

---Configure codemmet: start the language server for the configured filetypes,
---install buffer-local keymaps when it attaches, and (by default) enable
---Neovim's built-in LSP completion for it.
---@param opts? codemmet.SetupOpts
function M.setup(opts)
  opts = opts and vim.deepcopy(opts) or {}
  -- Back-compat shorthand: `completion = true/false` means enabled/disabled with
  -- the remaining fields (autotrigger, mode) at their defaults.
  if type(opts.completion) == "boolean" then
    opts.completion = { enabled = opts.completion }
  end
  config = vim.tbl_deep_extend("force", vim.deepcopy(defaults), opts)
  -- `completion.mode` is the first-class spelling of the server's completion gate.
  -- An explicit `init_options.completion` is the verbatim escape hatch and wins.
  if config.init_options.completion == nil and config.completion.mode ~= nil then
    config.init_options.completion = config.completion.mode
  end
  M._config = config
  config.preview = normalize_preview(config.preview)
  -- Always forward the debounce so `:CodemmetPreviewToggle` honors it even
  -- when the preview is not enabled at attach time.
  require("codemmet.preview").set_debounce(config.preview.debounce_ms)
  require("codemmet.notes").set_file(config.notes_file)
  local group = vim.api.nvim_create_augroup("codemmet", { clear = true })

  vim.api.nvim_create_autocmd("FileType", {
    group = group,
    pattern = config.filetypes,
    desc = "Start the codemmet language server",
    callback = function(ev)
      vim.lsp.start({
        name = "codemmet",
        cmd = config.cmd,
        root_dir = vim.fs.root(ev.buf, { ".git" }) or vim.uv.cwd(),
        init_options = config.init_options,
      }, { bufnr = ev.buf })
    end,
  })

  vim.api.nvim_create_autocmd("LspAttach", {
    group = group,
    desc = "Buffer-local codemmet keymaps and completion",
    callback = function(ev)
      local client = vim.lsp.get_client_by_id(ev.data.client_id)
      if not client or client.name ~= "codemmet" then
        return
      end
      local keymaps = config.keymaps or {}
      if keymaps.expand then
        vim.keymap.set({ "i", "n" }, keymaps.expand, M.expand, {
          buffer = ev.buf,
          desc = "codemmet: expand abbreviation",
        })
      end
      if keymaps.wrap then
        vim.keymap.set("x", keymaps.wrap, function()
          M.wrap()
        end, { buffer = ev.buf, desc = "codemmet: wrap selection" })
      end
      if config.preview.enabled then
        require("codemmet.preview").attach(ev.buf)
      end
      require("codemmet.completion").attach(client, ev.buf, config.completion)
    end,
  })
end

---Open the interactive syntax trainer (`codemmet train`) in a terminal split,
---training the current buffer's language when the plugin supports it.
---@param opts? codemmet.TrainOpts
function M.train(opts)
  require("codemmet.train").train(vim.tbl_extend("keep", opts or {}, {
    cmd = config.cmd,
    filetypes = config.filetypes,
  }))
end

---Toggle the live ghost-line preview for the current buffer.
function M.toggle_preview()
  require("codemmet.preview").toggle()
end

---Append a dogfooding note (with the current buffer's context) to the notes file.
---@param text string
function M.note(text)
  require("codemmet.notes").note(text)
end

return M
