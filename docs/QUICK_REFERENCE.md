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
| `Esc` | Cancel a prompt / close a popup / leave focus mode / back to sidebar (never quits) |
| `:` or `Alt+p` | Command palette (every action, searchable) |
| `?` | Help (`j`/`k` scroll, any other key closes) |
| `q` | Quit |

---

## Task Operations

| Key | Action |
|-----|--------|
| `a` | Add task |
| `Alt+a` | Quick capture (with syntax hints) |
| `A` | Add subtask |
| `e` | Edit title |
| `i` | View task details |
| `Enter` | View task details / Select |
| `d` | Delete |
| `x` | Toggle complete |
| `p` | Cycle priority |
| `D` | Set due date |
| `T` | Edit tags |
| `n` / `N` | Edit description (one line / multi-line editor) |
| `S` / `Alt+s` | Edit scheduled date / time block |
| `Alt+t` | Edit estimate (`1h30m`, `90m`) |
| `m` | Move to project |
| `Y` | Duplicate task |
| `B` | Edit dependencies (task numbers) |
| `R` | Set recurrence (`d`, `w2`, `m15`, `y`, `0` clears) |
| `]` / `[` / `M` | Reschedule: tomorrow / next week / next Monday |
| `z` / `Z` | Snooze until a date / clear snooze |
| `Ctrl+↑` / `Ctrl+↓` | Move task up / down |
| `Alt+l` / `Alt+u` | Link to next task in chain / unlink |

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
| `sched:date` | `sched:monday`, `sched:next week` |
| `time:range` | `time:9:00-11:00`, `time:9am-11am` |

**Date shortcuts:** `today`, `tomorrow`, `monday`-`sunday`, `next week`, `in 3 days`, `eom`

---

## Search & Filter

| Key | Action |
|-----|--------|
| `/` | Search |
| `#` | Filter by tag |
| `Ctrl+l` | Clear search |
| `Ctrl+t` | Clear tag filter |
| `s` / `Ctrl+o` | Cycle sort field / toggle order |
| `c` | Toggle completed |

---

## Time & Focus

| Key | Action |
|-----|--------|
| `t` | Start/stop timer |
| `L` | Time log (`a` add, `s`/`e` start/end, `n` note, `t` date, `+`/`-` 5 min, `d` delete) |
| `W` | Work log |
| `Alt+t` | Edit estimate |
| `f` | Focus mode |
| `F` | Full-screen focus |
| `Q` | Add to focus queue |
| `F5` | Start Pomodoro |
| `F6` | Pause/Resume |
| `F7` | Skip phase |
| `F8` | Stop Pomodoro |
| `+` / `-` | Adjust cycle goal (focus mode, session running) |

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

Every view lives in the sidebar: press `h` to focus it, `j`/`k` to move, `Enter` to open (there are no number-key shortcuts). In order:

| Sidebar entry | Shows |
|---------------|-------|
| All Tasks, Today, Upcoming, Overdue, Scheduled | Task lists filtered by due/scheduled date |
| Calendar | Monthly grid with the selected day's tasks |
| Dashboard | Completion, time tracking and project stats |
| Reports | Analytics panels (see below) |
| Habits, Goals | Habit tracker, OKR goals and key results |
| Blocked, Untagged, No Project, Recent | Unmet dependencies, no tags, no project, modified in 7 days |
| Kanban, Eisenhower, Weekly Planner, Timeline | Board, matrix, week and Gantt layouts |
| Snoozed | Tasks hidden until their snooze date |
| Heatmap, Forecast, Network, Burndown | Completion heatmap, workload forecast, dependency graph, burndown chart |
| Duplicates | Likely duplicate pairs |
| Git TODOs | Tasks extracted from code comments |
| *Projects* | One entry per project |

In any of the specialised views `Esc` returns focus to the sidebar. Focus mode (`f`/`F`) is an overlay, not a sidebar view.

---

## Kanban Board

| Key | Action |
|-----|--------|
| `h` / `l` | Move between columns |
| `j` / `k` | Move between tasks in column |
| `Enter` | View task details |
| `x` | Toggle complete |

Columns: Todo → In Progress → Done

---

## Eisenhower Matrix

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

## Weekly Planner

| Key | Action |
|-----|--------|
| `h` / `l` | Move between days |
| `j` / `k` | Move between tasks |
| `Enter` | View task details |
| `D` | Set due date for selected task |

Shows tasks organized by day of the week.

---

## Timeline

| Key | Action |
|-----|--------|
| `h` / `l` | Scroll left / right |
| `j` / `k` | Select previous / next task |
| `<` / `>` (or `,` / `.`) | Zoom out / in |
| `t` | Jump to today |
| `d` | Toggle dependency lines |
| `Enter` | View task details |

Visual timeline of tasks with due dates.

---

## Habits

| Key | Action |
|-----|--------|
| `n` | Create new habit |
| `e` | Edit habit |
| `d` | Delete habit |
| `Space` / `x` | Toggle today's check-in |
| `a` | Habit analytics (`a`, `q` or `Esc` closes) |
| `A` | Archive habit |
| `H` | Show/hide archived habits |
| `j` / `k` | Navigate habits |

Track daily/weekly habits with streak counters.

---

## Goals

| Key | Action |
|-----|--------|
| `n` | Create goal |
| `N` | Create key result under the selected goal |
| `Enter` / `l` | Expand goal / move into its key results |
| `h` | Collapse / back to goals |
| `e` | Edit goal |
| `d` | Delete goal (and its key results) |
| `a` | Show/hide archived goals |
| `j` / `k` | Navigate |

OKR tracking: objectives with measurable key results.

---

## Heatmap

View-only calendar showing task completion intensity over time.
- Darker cells = more tasks completed
- Navigate with standard calendar controls

---

## Forecast

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
| `[` / `]` | Navigate chain (prev/next) |
| `t` | Start/stop timer |
| `+` / `-` | Adjust Pomodoro cycle goal (while a session runs) |
| `x` | Toggle complete |
| `Esc` or `f` | Exit focus mode |

Distraction-free view for working on a single task. Full-screen focus removes all UI chrome. "Clear focus queue" and "Next in focus queue" have no default key; run them from the command palette (`:`) or bind them in the keybindings editor (`Ctrl+k`).

---

## Git Integration

| Key | Action |
|-----|--------|
| `Alt+g` | View Git TODOs |
| `r` | Rescan the repository (in the Git TODOs view) |
| `O` | Open the TODO's source file in `$EDITOR` |

---

## Snoozed Tasks

| Key | Action |
|-----|--------|
| `z` | Snooze selected task until a date (`tomorrow`, `in 3 days`, `2026-11-01`; empty clears) |
| `Z` | Clear snooze |
| Standard navigation | Same as task list |

Snoozed tasks are hidden until their snooze time expires.

---

## Multi-Select

| Key | Action |
|-----|--------|
| `v` | Enter/exit multi-select |
| `V` | Select all visible |
| `Space` | Toggle selection |
| `Ctrl+v` | Clear selection and exit |
| `p` / `T` / `D` / `z` | Set priority / add tags / set due date / snooze **all selected** |

Bulk delete, bulk move and bulk status have no default key (use the command palette `:`); `d`, `m` and `x` still act on the highlighted task.

---

## Undo & Save

| Key | Action |
|-----|--------|
| `u` / `Ctrl+z` | Undo |
| `U` / `Ctrl+r` | Redo |
| `Ctrl+s` | Save |
| `Alt+r` | Reload from storage |

---

## Export

| Key | Format |
|-----|--------|
| `Ctrl+e` | CSV |
| `Ctrl+i` | iCalendar |
| `Ctrl+p` | Markdown report |
| `Ctrl+h` | HTML report |
| `Ctrl+g` / `Ctrl+m` | Task chains as DOT / Mermaid |
| `I` / `Alt+i` | Import CSV / ICS |

---

## Reports

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Next / previous panel (Overview, Velocity, Tags, Time, Tasks, Focus, Insights, Estimation) |
| `w` | Cycle the time window (7 / 30 / 90 days) |
| `j` / `k`, `PgUp` / `PgDn`, `g` / `G` | Move in the Tasks table |

---

## Duplicates & Burndown

| Key | Action |
|-----|--------|
| `D` / `M` / `r` | Duplicates view: dismiss pair / merge / refresh |
| `w` / `o` / `+` | Burndown view: cycle window / toggle mode / toggle scope creep |

---

## Macros & Tools

| Key | Action |
|-----|--------|
| `Ctrl+q` then `0`-`9` | Start (or stop and save) recording into that slot |
| `@0`-`@9` | Play macro |
| `Ctrl+n` | Template picker (`j`/`k` or digit, `Enter`) |
| `Ctrl+k` | Keybindings editor (`Enter` rebind, `r`/`R` reset one/all, `s` save) |
| `i` / `Enter` | Task detail (`j`/`k`, `g`/`G`, `PgUp`/`PgDn`; `Esc`/`q` close) |

---

## Command Line

| Command | Does |
|---------|------|
| `taskflow add "Task #tag !high due:fri"` | Quick-add a task (alias `a`) |
| `taskflow list --view today` | List tasks (`ls`; views: all, today, overdue, upcoming, blocked, untagged, no-project, scheduled) |
| `taskflow list -p api -t bug,urgent --priority high --sort priority -n 50` | Filter by project, tags, priority, status, search; sort and limit |
| `taskflow list -f "priority:high AND !status:done"` | Filter with the DSL |
| `taskflow today` / `taskflow next` | Today's tasks (`t`) / the next task to work on (`n`) |
| `taskflow done "query" [-p project] [-t tags]` | Complete a task by title match (`d`) |
| `taskflow stats` | Quick statistics (`s`) |
| `taskflow git-todos [--repo .] [--patterns TODO,FIXME] [--dry-run]` | Create tasks from code comments (`gt`) |
| `taskflow git <link\|unlink\|commits\|status\|sync\|check-merged>` | Link tasks to branches, auto-complete on merge (`g`) |
| `echo '{"operation":"list","entity":"task"}' \| taskflow pipe [-f json\|yaml\|csv]` | Scripting interface (`p`) |
| `taskflow completion <bash\|zsh\|fish>` | Shell completions |
| `taskflow time` | Time tracked today (+ this week) |
| `taskflow time --week --by day` | This week, per day (`--by project\|day\|task\|tag`) |
| `taskflow time --days 30 --project api` | Last 30 days, one project |
| `taskflow time --from 2026-01-05 --to 2026-01-09 --json` | Custom window as JSON |

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

**Change several tasks at once:**
```
v → j/k and Space to select → p (priority), T (tags), D (due date) or z (snooze)
```

**Track time on a task:**
```
Select task → t (start) → work → t (stop)
taskflow time --week          # then see where the week went
```

**Start a Pomodoro session:**
```
Select task → F5 → work for 25min → break → repeat
```

---

*For full documentation: [USER_MANUAL.md](USER_MANUAL.md)*
