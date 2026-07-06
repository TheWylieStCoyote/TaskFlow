if vim.g.loaded_codemmet then
  return
end
vim.g.loaded_codemmet = true

vim.api.nvim_create_user_command("CodemmetExpand", function()
  require("codemmet").expand()
end, { desc = "Expand the codemmet abbreviation before the cursor" })

vim.api.nvim_create_user_command("CodemmetWrap", function(cmd)
  require("codemmet").wrap({ line1 = cmd.line1, line2 = cmd.line2 })
end, { range = true, desc = "Wrap the given lines in a codemmet abbreviation" })

vim.api.nvim_create_user_command("CodemmetPreviewToggle", function()
  require("codemmet").toggle_preview()
end, { desc = "Toggle the codemmet live ghost-line preview in this buffer" })

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
