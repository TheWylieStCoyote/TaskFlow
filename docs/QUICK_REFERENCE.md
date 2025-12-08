# TaskFlow Quick Reference

A one-page cheat sheet for TaskFlow. For detailed documentation, see [USER_MANUAL.md](USER_MANUAL.md).

---

## Essential Navigation

| Key | Action |
|-----|--------|
| `j` / `k` | Move down / up |
| `g` / `G` | First / last item |
| `h` / `l` | Sidebar / task list |
| `Enter` | Select |
| `Esc` | Cancel / close |
| `?` | Help |
| `q` | Quit |

---

## Task Operations

| Key | Action |
|-----|--------|
| `a` | Add task |
| `A` | Add subtask |
| `e` | Edit title |
| `d` | Delete |
| `x` | Toggle complete |
| `p` | Cycle priority |
| `D` | Set due date |
| `T` | Edit tags |
| `m` | Move to project |

---

## Quick Add Syntax

Create tasks with metadata in one line:

```
Fix bug #backend !high due:tomorrow @Work
```

| Syntax | Example |
|--------|---------|
| `#tag` | `#work`, `#urgent` |
| `!priority` | `!urgent`, `!high`, `!med`, `!low` |
| `due:date` | `due:tomorrow`, `due:friday`, `due:2025-01-15` |
| `@project` | `@Work`, `@Personal` |

**Date shortcuts:** `today`, `tomorrow`, `monday`-`sunday`, `next week`, `in 3 days`, `eom`

---

## Search & Filter

| Key | Action |
|-----|--------|
| `/` | Search |
| `#` | Filter by tag |
| `Ctrl+l` | Clear search |
| `Ctrl+t` | Clear tag filter |
| `s` / `S` | Sort field / order |
| `c` | Toggle completed |

---

## Time & Focus

| Key | Action |
|-----|--------|
| `t` | Start/stop timer |
| `L` | Time log |
| `f` | Focus mode |
| `F5` | Start Pomodoro |
| `F6` | Pause/Resume |
| `F8` | Stop Pomodoro |

---

## Projects

| Key | Action |
|-----|--------|
| `P` | Create project |
| `E` | Edit project (sidebar) |
| `X` | Delete project (sidebar) |

---

## Multi-Select

| Key | Action |
|-----|--------|
| `v` | Enter multi-select |
| `V` | Select all |
| `Space` | Toggle selection |
| `d` | Delete selected |

---

## Undo & Save

| Key | Action |
|-----|--------|
| `u` | Undo |
| `U` | Redo |
| `Ctrl+s` | Save |

---

## Export

| Key | Format |
|-----|--------|
| `Ctrl+e` | CSV |
| `Ctrl+i` | iCalendar |
| `Ctrl+p` | Markdown report |
| `Ctrl+h` | HTML report |

---

## Priority Symbols

| Symbol | Level |
|--------|-------|
| `!!!!` | Urgent |
| `!!!` | High |
| `!!` | Medium |
| `!` | Low |
| (blank) | None |

## Status Symbols

| Symbol | Status |
|--------|--------|
| `[ ]` | Todo |
| `[~]` | In Progress |
| `[!]` | Blocked |
| `[x]` | Done |
| `[-]` | Cancelled |

---

## Common Workflows

**Create a task with due date:**
```
a → "Buy groceries due:tomorrow #errands" → Enter
```

**Complete multiple tasks:**
```
v → j/k and Space to select → d to delete (or x to complete)
```

**Track time on a task:**
```
Select task → t (start) → work → t (stop)
```

**Start a Pomodoro session:**
```
Select task → F5 → work for 25min → break → repeat
```

---

*For full documentation: [USER_MANUAL.md](USER_MANUAL.md)*
