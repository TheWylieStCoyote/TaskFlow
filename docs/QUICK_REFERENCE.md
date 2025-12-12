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
| `i` | View task details |
| `Enter` | View task details / Select |
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
| `F` | Full-screen focus |
| `Q` | Add to focus queue |
| `F5` | Start Pomodoro |
| `F6` | Pause/Resume |
| `F8` | Stop Pomodoro |

---

## Reviews

| Key | Action |
|-----|--------|
| `Alt+d` | Daily Review |
| `Alt+w` | Weekly Review |
| `Alt+e` | Evening Review |

---

## Projects

| Key | Action |
|-----|--------|
| `P` | Create project |
| `E` | Edit project (sidebar) |
| `X` | Delete project (sidebar) |

---

## Views

Switch between views using number keys:

| Key | View |
|-----|------|
| `1` | Task List |
| `2` | Calendar |
| `3` | Dashboard |
| `4` | Kanban |
| `5` | Eisenhower Matrix |
| `6` | Weekly Planner |
| `7` | Timeline |
| `8` | Habits |
| `9` | Heatmap |
| `0` | Forecast |

Additional views (access via search/command):
- **Burndown** - Sprint progress chart
- **Network** - Task dependency graph
- **Snoozed** - Hidden tasks until snooze expires
- **Focus Mode** - Distraction-free single-task view

---

## Kanban Board (4)

| Key | Action |
|-----|--------|
| `h` / `l` | Move between columns |
| `j` / `k` | Move between tasks in column |
| `Enter` | View task details |
| `x` | Toggle complete |

Columns: Todo → In Progress → Done

---

## Eisenhower Matrix (5)

| Key | Action |
|-----|--------|
| `h` / `l` | Move left / right quadrants |
| `j` / `k` | Move up / down quadrants |
| `Enter` | View task details |

Quadrants based on priority and due date:
- **Urgent + Important** (top-left)
- **Not Urgent + Important** (top-right)
- **Urgent + Not Important** (bottom-left)
- **Not Urgent + Not Important** (bottom-right)

---

## Weekly Planner (6)

| Key | Action |
|-----|--------|
| `h` / `l` | Move between days |
| `j` / `k` | Move between tasks |
| `Enter` | View task details |
| `D` | Set due date for selected task |

Shows tasks organized by day of the week.

---

## Timeline (7)

| Key | Action |
|-----|--------|
| `h` / `l` | Scroll left / right |
| `j` / `k` | Select previous / next task |
| `<` / `>` | Zoom out / in |
| `t` | Jump to today |
| `Enter` | View task details |

Visual timeline of tasks with due dates.

---

## Habits (8)

| Key | Action |
|-----|--------|
| `n` | Create new habit |
| `e` | Edit habit |
| `d` | Delete habit |
| `Space` | Check in today |
| `j` / `k` | Navigate habits |

Track daily/weekly habits with streak counters.

---

## Heatmap (9)

View-only calendar showing task completion intensity over time.
- Darker cells = more tasks completed
- Navigate with standard calendar controls

---

## Forecast (0)

View-only workload projection:
- Weekly task distribution chart
- Daily capacity breakdown
- Upcoming deadlines
- ⚠ Overload warnings when capacity exceeded

---

## Network Graph

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate between nodes |
| `h` / `l` | Navigate between nodes |
| `Enter` | View task details |

Visual graph of task dependencies and chains.

---

## Focus Mode (f/F)

| Key | Action |
|-----|--------|
| `f` | Toggle focus mode |
| `F` | Toggle full-screen focus |
| `Q` | Add to focus queue |
| `Alt+Q` | Clear focus queue |
| `Alt+J` | Advance focus queue |
| `[` / `]` | Navigate chain (prev/next) |
| `t` | Start/stop timer |
| `x` | Toggle complete |
| `Esc` | Exit focus mode |

Distraction-free view for working on a single task. Full-screen focus removes all UI chrome.

---

## Git Integration

| Key | Action |
|-----|--------|
| `Alt+g` | View Git TODOs |
| `O` | Open file in editor |

---

## Snoozed Tasks

| Key | Action |
|-----|--------|
| `z` | Snooze selected task |
| Standard navigation | Same as task list |

Snoozed tasks are hidden until their snooze time expires.

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
