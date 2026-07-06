---codemmet.nvim — Emmet-style abbreviation expansion for code, backed by the
---`codemmet lsp` language server. Requires Neovim >= 0.10.
local M = {}

---@class codemmet.KeymapOpts
---@field expand string|false insert- and normal-mode mapping for expand(); false disables
---@field wrap string|false visual-mode mapping for wrap(); false disables

---@class codemmet.SetupOpts
---@field cmd? string[] command used to start the language server
---@field filetypes? string[] filetypes the server is started for
---@field keymaps? codemmet.KeymapOpts
---@field init_options? table LSP initializationOptions ({ snippets_dir?, completion? })

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
  init_options = {},
}

---Expand the abbreviation immediately before the cursor.
function M.expand()
  require("codemmet.lsp").expand()
end

---Wrap the visual selection (or `opts.line1..opts.line2`) in an abbreviation.
---@param opts? codemmet.WrapOpts
function M.wrap(opts)
  require("codemmet.wrap").wrap(opts)
end

---Configure codemmet: start the language server for the configured filetypes and
---install buffer-local keymaps when it attaches.
---@param opts? codemmet.SetupOpts
function M.setup(opts)
  local config = vim.tbl_deep_extend("force", vim.deepcopy(defaults), opts or {})
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
    desc = "Buffer-local codemmet keymaps",
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
    end,
  })
end

return M
