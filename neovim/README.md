# TaskFlow Neovim Plugin

Neovim integration for [TaskFlow](https://github.com/TheWylieStCoyote/TaskFlow) via a persistent Unix Domain Socket. Create tasks from code comments, browse tasks in a floating window, and surface a statusline component — without spawning a new process on every request.

**Requires:** Neovim 0.10 or newer (tested on 0.12), and the TaskFlow TUI running (`cargo run`) — it creates and owns the socket.

---

## Installation

### lazy.nvim

```lua
{
  dir = '/path/to/taskflow/neovim',  -- path to the neovim/ directory in this repo
  config = function()
    require('taskflow').setup({
      -- socket_path = '',  -- auto-resolves to ~/.local/share/taskflow/taskflow.sock
      timeout_ms = 5000,
    })
  end,
}
```

### packer.nvim

```lua
use {
  '/path/to/taskflow/neovim',
  config = function()
    require('taskflow').setup({})
  end,
}
```

### vim-plug

```vim
Plug '/path/to/taskflow/neovim'
```

Then in your config:

```lua
require('taskflow').setup({})
```

### Manual (no plugin manager)

```bash
mkdir -p ~/.config/nvim/pack/manual/start
ln -s /path/to/taskflow/neovim ~/.config/nvim/pack/manual/start/taskflow
```

Then add to `init.lua`:

```lua
require('taskflow').setup({})
```

---

## Commands

| Command | Mode | Description |
|---------|------|-------------|
| `:TaskFlowAdd [title]` | Normal, Visual | Create a task; visual mode uses selection; prompts if no title |
| `:TaskFlowList` | Normal | Floating task browser; `<CR>` focuses task in TUI |
| `:TaskFlowCapture` | Normal | Batch-create tasks from TODO/FIXME in current buffer |
| `:TaskFlowStatus` | Normal | Show focus task + incomplete count via `vim.notify` |
| `:TaskFlowOpen` | Normal | Open TUI in a floating terminal |

---

## Statusline

Returns a cached task count string, refreshed every 30 seconds (no per-render socket overhead):

```lua
vim.o.statusline = '%{%v:lua.require("taskflow").statusline()%}'
-- renders: " 5 tasks"  or  ""  when TUI is not running
```

Works with lualine too:

```lua
require('lualine').setup({
  sections = {
    lualine_x = { require('taskflow').statusline },
  }
})
```

---

## Telescope Extension (optional)

Requires [telescope.nvim](https://github.com/nvim-telescope/telescope.nvim).

```lua
require('telescope').load_extension('taskflow')
```

Then:

```
:Telescope taskflow
```

| Key | Action |
|-----|--------|
| `<CR>` | Focus task in TUI |
| `<C-d>` | Delete task |
| `<C-c>` | Create new task |

---

## Configuration

All options and their defaults:

```lua
require('taskflow').setup({
  -- Path to the Unix domain socket.
  -- Empty string = auto-resolve from $XDG_DATA_HOME/taskflow/taskflow.sock
  socket_path = '',

  -- Timeout for synchronous requests in milliseconds
  timeout_ms = 5000,

  -- How often to refresh the statusline cache (seconds)
  statusline_refresh_s = 30,

  -- Floating window size as a fraction of the editor
  float_width  = 0.8,
  float_height = 0.7,

  -- Enable Telescope extension (auto-detected if telescope.nvim is installed)
  telescope = true,
})
```

The socket path can also be set on the Rust side via `config.toml`:

```toml
[editor]
enabled          = true
socket_path      = ""        # auto: $XDG_DATA_HOME/taskflow/taskflow.sock
socket_permissions = 384     # 0o600 in decimal
query_timeout_ms = 5000
auto_open_tui    = false
```

---

## Verifying the Connection

Start TaskFlow first, then test the socket with `socat`:

```bash
echo '{"type":"ping"}' | socat - UNIX-CONNECT:~/.local/share/taskflow/taskflow.sock
# → {"success":true,"data":{"pong":true,"protocol_version":"1",...}}
```

Or from Neovim:

```vim
:TaskFlowStatus
```

---

## Protocol

The plugin communicates over a newline-terminated JSON protocol. One request → one response → connection closed.

```jsonc
// Ping
{ "type": "ping" }

// Status
{ "type": "status" }

// Focus a task in the TUI
{ "type": "focus_task", "task_id": "abc-123" }

// Pipe (wraps existing TaskFlow pipe protocol)
{ "type": "pipe", "operation": "create", "entity": "task", "data": { "title": "Fix bug" } }
{ "type": "pipe", "operation": "list",   "entity": "task" }
```
