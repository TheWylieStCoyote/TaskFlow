--- Telescope extension entry point.
-- Lets users run `require('telescope').load_extension('taskflow')` and then
-- `:Telescope taskflow`, in addition to the registration done by setup().
return require('telescope').register_extension({
  exports = { taskflow = require('taskflow.telescope').taskflow },
})
