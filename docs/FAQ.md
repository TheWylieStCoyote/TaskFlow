# TaskFlow FAQ & Troubleshooting

Common questions and solutions for TaskFlow users.

---

## Table of Contents

- [General Questions](#general-questions)
- [Data & Storage](#data--storage)
- [Performance](#performance)
- [Keybindings & Input](#keybindings--input)
- [Time Tracking](#time-tracking)
- [Sync & Backup](#sync--backup)
- [Troubleshooting](#troubleshooting)

---

## General Questions

### Where is TaskFlow's configuration stored?

Configuration files are located at:
- **Linux/macOS:** `~/.config/taskflow/`
- **Windows:** `%APPDATA%\taskflow\config\`

Key files:
- `config.toml` - General settings
- `keybindings.toml` - Custom key mappings
- `themes/` - Color theme files

### Can I use TaskFlow over SSH?

Yes! TaskFlow works in any terminal that supports ANSI colors and UTF-8, including:
- SSH sessions
- tmux/screen
- Remote development environments
- Docker containers

Just ensure your terminal is configured for UTF-8.

### Does TaskFlow support multiple users?

TaskFlow is designed for single-user use. For multi-user scenarios, each user should have their own data file. The application doesn't include user authentication or access control.

### What's the difference between due date and scheduled date?

- **Due date:** When the task should be completed
- **Scheduled date:** When you plan to work on the task

Example: A report due Friday could be scheduled for Wednesday.

---

## Data & Storage

### Where is my data stored?

Default data locations:
- **Linux:** `~/.local/share/taskflow/tasks.json`
- **macOS:** `~/Library/Application Support/taskflow/tasks.json`
- **Windows:** `%APPDATA%\taskflow\tasks.json`

Override with: `taskflow --data /path/to/your/data.json`

### Which storage backend should I use?

| Backend | Best For |
|---------|----------|
| **JSON** (default) | Most users, < 1000 tasks |
| **YAML** | Manual editing, human-readable |
| **SQLite** | Large datasets (1000+ tasks), performance |
| **Markdown** | Git integration, external editing |

### How do I migrate to a different backend?

1. Export current tasks: Press `Ctrl+e` to export CSV
2. Start with new backend:
   ```bash
   taskflow --backend sqlite --data ~/tasks.db
   ```
3. Import CSV: Press `I` and select your file

### Can I edit data files directly?

- **JSON/YAML:** Yes, but close TaskFlow first to avoid conflicts
- **SQLite:** Use a SQLite browser, but be careful with schema
- **Markdown:** Yes, each task is a separate `.md` file

After external edits, restart TaskFlow to reload data.

### Is my data encrypted?

No, TaskFlow stores data in plain text. For sensitive tasks:
- Store data on an encrypted drive
- Use encrypted cloud storage
- Consider encrypting the data file with external tools

---

## Performance

### TaskFlow is slow with many tasks. What can I do?

1. **Switch to SQLite backend** - Best for 1000+ tasks:
   ```bash
   taskflow --backend sqlite --data ~/tasks.db
   ```

2. **Hide completed tasks** - Press `c` to toggle

3. **Filter by project** - View one project at a time

4. **Increase/disable auto-save:**
   ```toml
   # In config.toml
   auto_save_interval = 600  # 10 minutes, or 0 to disable
   ```

5. **Use project filters** instead of viewing all tasks

### What are the performance limits?

Tested performance thresholds:
- **JSON:** Good up to ~5,000 tasks
- **YAML:** Good up to ~3,000 tasks
- **SQLite:** Good up to 100,000+ tasks
- **Markdown:** Good up to ~500 tasks (file I/O overhead)

### Memory usage seems high

TaskFlow keeps all tasks in memory. Approximate usage:
- ~1KB per task (base)
- +100 bytes per tag
- +500 bytes per time entry
- +2KB for tasks with subtasks

For 10,000 tasks: expect ~15-20MB.

---

## Keybindings & Input

### Some keybindings don't work in my terminal

Common causes:
1. **Terminal intercepts the key** - Some terminals capture `Ctrl+` combinations
2. **SSH/tmux issues** - May require configuration
3. **Conflicting software** - Desktop shortcuts may override

**Solutions:**
- Check terminal settings for key handling
- Use alternative bindings (customize with `Ctrl+k`)
- In tmux, ensure `set -g mouse off` or prefix keys properly

### How do I reset keybindings to default?

1. Press `Ctrl+k` to open keybindings editor
2. Press `R` (capital) to reset all bindings
3. Press `Ctrl+s` to save

Or delete `~/.config/taskflow/keybindings.toml` and restart.

### Can I use vim-style keybindings?

TaskFlow uses vim-style navigation by default:
- `j`/`k` for up/down
- `h`/`l` for sidebar/list
- `g`/`G` for first/last

Customize additional mappings in the keybindings editor.

---

## Time Tracking

### The timer kept running while I was away

This is intentional—time tracking persists across restarts. To fix:
1. Press `L` to open time log
2. Select the incorrect entry
3. Press `e` to edit times
4. Adjust start/end times

### How do I track time for completed tasks?

1. Select the completed task
2. Press `L` for time log
3. Press `a` to add a manual entry
4. Enter start time, end time, or duration

### Can I export time tracking data?

Time data is included in:
- CSV exports (`Ctrl+e`)
- Reports view (Dashboard → Time panel)
- Markdown/HTML reports (`Ctrl+p`, `Ctrl+h`)

---

## Sync & Backup

### How do I sync TaskFlow across devices?

TaskFlow doesn't have built-in sync. Options:

1. **Cloud storage sync:**
   ```bash
   taskflow --data ~/Dropbox/taskflow/tasks.json
   ```
   Works with Dropbox, Google Drive, iCloud Drive, OneDrive.

2. **Git with Markdown backend:**
   ```bash
   taskflow --backend markdown --data ~/git/tasks/
   cd ~/git/tasks && git add -A && git commit -m "sync"
   ```

3. **Manual sync:** Copy data file between devices.

**Warning:** Avoid opening TaskFlow on multiple devices simultaneously with shared storage—data conflicts may occur.

### How should I backup my data?

Recommended backup strategies:

1. **Automated file backup:**
   ```bash
   cp ~/.local/share/taskflow/tasks.json ~/backups/tasks-$(date +%Y%m%d).json
   ```

2. **Periodic CSV export:** Press `Ctrl+e` weekly

3. **Version control:** Use Markdown backend with Git

4. **Cloud storage:** Store in synced folder

### I lost my data. Can I recover it?

If you have:
- A backup file → Restore it to the data location
- A CSV export → Start fresh and import with `I`
- Markdown files → Point TaskFlow to the directory

Without backups, data cannot be recovered.

---

## Troubleshooting

### TaskFlow won't start

**Check Rust version:**
```bash
rustc --version  # Needs 1.87+
```

**Check for data corruption:**
```bash
# Backup and try with fresh data
mv ~/.local/share/taskflow ~/.local/share/taskflow.bak
taskflow
```

**Check terminal compatibility:**
- Ensure UTF-8 support
- Try a different terminal emulator

### Display looks corrupted

1. **Resize terminal:** Sometimes fixes rendering
2. **Check font:** Use a monospace font with Unicode support
3. **Check terminal size:** Minimum ~80x24 recommended
4. **Try different terminal:** Some have better rendering

### Changes aren't being saved

1. **Check write permissions** on data file/directory
2. **Verify disk space** is available
3. **Check for file locks** (close other processes)
4. **Manual save:** Press `Ctrl+s`

### Import isn't working

**For CSV:**
- Ensure header row: `title,status,priority,due_date,tags`
- Use quotes for fields with commas
- Date format: `YYYY-MM-DD`

**For ICS:**
- Must be valid iCalendar format
- Only VTODO items are imported

### Tags with special characters cause issues

Avoid these in tags:
- Commas (used as separator)
- Spaces (use hyphens: `#my-tag`)
- Quotes

Valid: `#work`, `#high-priority`, `#2025`

### Recurring tasks not creating new instances

Check that:
1. Recurrence is set (shows `↻`)
2. Original task has a due date
3. You're completing (not deleting) the task

### Undo isn't working as expected

Undo limitations:
- Maximum 50 actions in history
- Some operations are combined (completing parent + children)
- External changes (file edits) aren't tracked

---

## Getting Help

### Where can I report bugs?

Open an issue at: https://github.com/anthropics/claude-code/issues

Include:
- TaskFlow version
- Operating system
- Steps to reproduce
- Error messages (if any)

### Where can I request features?

Use the same issue tracker with a "feature request" label.

### Is there a community forum?

Check the GitHub Discussions tab for community Q&A.

---

*For complete documentation, see [USER_MANUAL.md](USER_MANUAL.md)*
