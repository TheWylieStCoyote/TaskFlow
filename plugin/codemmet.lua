if vim.g.loaded_codemmet then
  return
end
vim.g.loaded_codemmet = true

-- `.codemmet` source files (file mode) are a first-class authoring format.
vim.filetype.add({ extension = { codemmet = "codemmet" } })

vim.api.nvim_create_user_command("CodemmetExpand", function()
  require("codemmet").expand()
end, { desc = "Expand the codemmet abbreviation before the cursor" })

vim.api.nvim_create_user_command("CodemmetWrap", function(cmd)
  require("codemmet").wrap({ line1 = cmd.line1, line2 = cmd.line2 })
end, { range = true, desc = "Wrap the given lines in a codemmet abbreviation" })

vim.api.nvim_create_user_command("CodemmetRenderBuffer", function(cmd)
  require("codemmet").render({ line1 = cmd.line1, line2 = cmd.line2 })
end, {
  range = "%",
  desc = "Render the buffer (or the given range) as a .codemmet source into code",
})

vim.api.nvim_create_user_command("CodemmetCapture", function(cmd)
  require("codemmet").capture({
    name = vim.trim(cmd.args) ~= "" and vim.trim(cmd.args) or nil,
    line1 = cmd.line1,
    line2 = cmd.line2,
  })
end, {
  range = true,
  nargs = "?",
  desc = "Capture the selected lines into the project snippet pack",
})

vim.api.nvim_create_user_command("CodemmetSnippets", function()
  require("codemmet").snippets()
end, { desc = "Browse codemmet snippets for this buffer and expand the picked one" })

vim.api.nvim_create_user_command("CodemmetContext", function()
  require("codemmet").context()
end, { desc = "Show the codemmet context (scopes, collections) at the cursor" })
vim.api.nvim_create_user_command("CodemmetRenderTo", function(cmd)
  local file = vim.trim(cmd.args)
  require("codemmet").render_to({ file = file ~= "" and file or nil })
end, {
  nargs = "?",
  complete = "file",
  desc = "Render the .codemmet buffer to a file (default: the sidecar name)",
})

vim.api.nvim_create_user_command("CodemmetPreviewToggle", function()
  require("codemmet").toggle_preview()
end, { desc = "Toggle the codemmet live ghost-line preview in this buffer" })

vim.api.nvim_create_user_command("CodemmetTrain", function(cmd)
  require("codemmet").train({ args = cmd.fargs })
end, {
  nargs = "*",
  desc = "Open the codemmet syntax trainer in a terminal split",
})

vim.api.nvim_create_user_command("CodemmetRenderPreview", function()
  require("codemmet").toggle_render_preview()
end, {
  desc = "Toggle a live split rendering this buffer as a .codemmet source file",
})

vim.api.nvim_create_user_command("CodemmetNote", function(cmd)
  if vim.trim(cmd.args) ~= "" then
    require("codemmet").note(cmd.args)
    return
  end
  vim.ui.input({ prompt = "codemmet note: " }, function(input)
    if input then
      require("codemmet").note(input)
    end
  end)
end, { nargs = "*", desc = "Append a timestamped codemmet dogfooding note" })
