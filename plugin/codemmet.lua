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
