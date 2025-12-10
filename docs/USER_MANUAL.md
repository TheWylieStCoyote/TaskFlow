# TaskFlow User Manual

A comprehensive guide to TaskFlow, your terminal-based task management application.

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Getting Started](#2-getting-started)
3. [The Interface](#3-the-interface)
4. [Task Management](#4-task-management)
5. [Organization](#5-organization)
6. [Views & Filtering](#6-views--filtering)
7. [Time Management](#7-time-management)
8. [Analytics & Reports](#8-analytics--reports)
9. [Data Management](#9-data-management)
10. [Customization](#10-customization)
11. [Advanced Features](#11-advanced-features)
12. [Appendices](#appendices)

---

## 1. Introduction

### What is TaskFlow?

TaskFlow is a powerful, keyboard-driven task management application that runs entirely in your terminal. Inspired by tools like Taskwarrior and Todoist, TaskFlow combines the speed of a CLI with the visual clarity of a TUI (Terminal User Interface).

Whether you're managing personal todos, tracking work projects, or organizing complex workflows with dependencies, TaskFlow provides the features you need without ever leaving your terminal.

### Key Features

- **Fast keyboard navigation** - Vim-style keybindings for efficient task management
- **Project organization** - Group tasks into projects with sidebar navigation
- **Flexible tagging** - Categorize tasks with tags and filter dynamically
- **Time tracking** - Built-in timer with start/stop and detailed logging
- **Pomodoro technique** - Integrated focus timer with work/break cycles
- **Multiple views** - Today, Upcoming, Calendar, Dashboard, and more
- **Task dependencies** - Block tasks until prerequisites are complete
- **Recurring tasks** - Daily, weekly, monthly, or yearly repetition
- **Subtasks** - Break down complex tasks into hierarchies
- **Undo/redo** - Safely experiment with full history support
- **Multiple backends** - Store data as JSON, YAML, SQLite, or Markdown
- **Customizable** - Themes, keybindings, and settings via config files
- **Import/Export** - CSV and iCalendar format support

### System Requirements

- **Operating System**: Linux, macOS, or Windows (with a terminal supporting ANSI colors)
- **Terminal**: Any modern terminal with UTF-8 support
- **Rust**: Version 1.87+ (for building from source)

---

## 2. Getting Started

### Installation

#### Building from Source

```bash
# Clone the repository
git clone <repository-url>
cd taskflow

# Build the release version
cargo build --release

# The binary is at ./target/release/taskflow
# Optionally, copy it to your PATH:
sudo cp ./target/release/taskflow /usr/local/bin/
```

#### Verify Installation

```bash
taskflow --version
```

### First Launch

Run TaskFlow with default settings:

```bash
taskflow
```

On first launch, TaskFlow creates a data file in the default location (`~/.local/share/taskflow/tasks.json`) and displays an empty task list.

#### Launch with Demo Data

To explore TaskFlow with sample tasks:

```bash
taskflow --demo
```

This loads example projects, tasks, and time entries so you can immediately try the interface.

### Basic Concepts

Before diving in, understand these core concepts:

#### Tasks

The fundamental unit in TaskFlow. Each task has:
- **Title** - What needs to be done
- **Status** - Todo, In Progress, Blocked, Done, or Cancelled
- **Priority** - None, Low, Medium, High, or Urgent
- **Due date** - When it should be completed (optional)
- **Tags** - Labels for categorization (optional)
- **Description** - Additional notes (optional)

#### Projects

Containers for related tasks. Projects help you:
- Group tasks by area (Work, Personal, Home)
- Filter to see only relevant tasks
- Track progress across related items

#### Tags

Flexible labels that cut across projects:
- A task can have multiple tags
- Filter by one or more tags
- Examples: `#bug`, `#meeting`, `#research`, `#urgent`

### Quick Tutorial (5 Minutes)

Let's create your first task and explore the basics.

**Step 1: Create a Task**
1. Press `a` to add a new task
2. Type: `Buy groceries #errands due:tomorrow`
3. Press `Enter`

You've created a task with a tag and due date using Quick Add syntax!

**Step 2: Navigate**
- Press `j` to move down (or `↓`)
- Press `k` to move up (or `↑`)
- Press `g` to jump to the first task
- Press `G` to jump to the last task

**Step 3: Complete a Task**
- Select a task and press `x` or `Space` to mark it complete

**Step 4: Change Priority**
- Press `p` to cycle through priority levels
- Watch the `!` indicators change on the left

**Step 5: Create a Project**
1. Press `P` to create a project
2. Type a project name (e.g., "Home")
3. Press `Enter`

**Step 6: Assign to Project**
1. Select a task
2. Press `m` to move it
3. Select your project and press `Enter`

**Step 7: Explore Views**
1. Press `h` to focus the sidebar
2. Use `j`/`k` to navigate between views
3. Press `Enter` to select a view

**Step 8: Get Help**
- Press `?` to show the help popup with all keybindings
- Press `?` again or `Esc` to close it

**Step 9: Quit**
- Press `q` to exit TaskFlow
- Your data is saved automatically!

Congratulations! You've learned the basics. Continue reading for in-depth coverage of every feature.

---

## 3. The Interface

### Layout Overview

TaskFlow's interface is divided into distinct areas:

```
┌─────────────────────────────────────────────────────────────────┐
│ TaskFlow                                                        │ ← Header
├──────────────┬──────────────────────────────────────────────────┤
│              │                                                  │
│   Sidebar    │              Task List                           │
│              │                                                  │
│  ┌─────────┐ │  ┌─────────────────────────────────────────────┐ │
│  │ Views   │ │  │ !!!! [ ] Fix critical bug        due:today  │ │
│  │ ─────── │ │  │ !!!  [~] Implement feature       #backend   │ │
│  │ All     │ │  │ !!   [ ] Write tests             #testing   │ │
│  │ Today   │ │  │ !    [ ] Update documentation               │ │
│  │ Upcoming│ │  │      [x] Initial setup           completed  │ │
│  │ Overdue │ │  └─────────────────────────────────────────────┘ │
│  │ Calendar│ │                                                  │
│  │         │ │                                                  │
│  │ Projects│ │                                                  │
│  │ ─────── │ │                                                  │
│  │ Work    │ │                                                  │
│  │ Personal│ │                                                  │
│  └─────────┘ │                                                  │
├──────────────┴──────────────────────────────────────────────────┤
│ 5 tasks (1 completed) │ showing all │ ? for help               │ ← Footer
└─────────────────────────────────────────────────────────────────┘
```

### Header

The top bar displays:
- Application name ("TaskFlow")
- Current view or search query
- Active Pomodoro timer (when running)

### Sidebar

The left panel contains:

**Views Section**
- All Tasks - Every task regardless of status
- Today - Tasks due today
- Upcoming - Tasks with future due dates
- Overdue - Tasks past their due date
- Calendar - Monthly calendar view
- Dashboard - Statistics overview
- Reports - Detailed analytics
- Scheduled - Tasks with scheduled dates
- Blocked - Tasks waiting on dependencies
- Untagged - Tasks without tags
- No Project - Unassigned tasks
- Recent - Recently modified tasks

**Projects Section**
- Lists all your projects
- Shows task count per project
- Selecting a project filters the task list

### Task List

The main content area showing tasks with:

```
Priority  Status  Title                      Tags         Due Date
   │        │       │                          │              │
   ▼        ▼       ▼                          ▼              ▼
  !!!!    [ ]    Fix critical bug           #urgent        [12/15]
```

**Priority Indicators** (leftmost):
| Symbol | Level |
|--------|-------|
| `!!!!` | Urgent |
| `!!!` | High |
| `!!` | Medium |
| `!` | Low |
| (blank) | None |

**Status Symbols**:
| Symbol | Status |
|--------|--------|
| `[ ]` | Todo |
| `[~]` | In Progress |
| `[!]` | Blocked |
| `[x]` | Done |
| `[-]` | Cancelled |

**Additional Indicators**:
- `↻` - Recurring task
- `[B]` - Has blocking dependencies
- `▶` - Has subtasks
- `⏱` - Time is being tracked

### Footer / Status Bar

The bottom bar shows:
- Task count: "5 tasks (1 completed)"
- Filter state: "showing all" or current filter
- Mode indicators: Search, Multi-select, etc.
- Help reminder: "? for help"
- Pomodoro status when active

### Modal Dialogs

TaskFlow uses popups for:
- **Input dialogs** - Creating/editing tasks, setting dates
- **Confirmation dialogs** - Delete confirmation
- **Help popup** - Keybinding reference
- **Template picker** - Quick task creation
- **Time log editor** - Managing time entries

Press `Esc` to close any dialog without saving.

---

## 4. Task Management

### Creating Tasks

#### Basic Creation

1. Press `a` to open the task input
2. Type your task title
3. Press `Enter` to create

#### Quick Add Syntax

Create tasks with metadata in a single line:

```
Buy groceries #errands !high due:tomorrow @Personal
```

| Syntax | Meaning | Example |
|--------|---------|---------|
| `#tag` | Add a tag | `#work`, `#bug` |
| `!priority` | Set priority | `!urgent`, `!high`, `!med`, `!low` |
| `due:date` | Set due date | `due:tomorrow`, `due:2025-01-15` |
| `sched:date` | Set scheduled date | `sched:monday` |
| `@project` | Assign to project | `@Work`, `@Personal` |

**Date Keywords:**
- `today`, `tod` - Today's date
- `tomorrow`, `tom` - Tomorrow
- `monday`, `mon`, `tuesday`, `tue`, etc. - Next occurrence of weekday
- `next week` - Same day next week
- `next month` - Same day next month
- `in 3 days` - Three days from now
- `in 2 weeks` - Two weeks from now
- `eow` - End of week (Sunday)
- `eom` - End of month
- `eoy` - End of year
- `1st`, `15th`, `22nd` - Day of current month
- `YYYY-MM-DD` - Specific date (ISO format)

**Examples:**
```
Fix login bug #bug !urgent due:today @Backend
Write report due:friday #work
Call dentist sched:next monday @Personal
Review PR #code-review !high due:in 2 days
```

### Editing Tasks

#### Edit Title
1. Select a task
2. Press `e`
3. Modify the title
4. Press `Enter` to save or `Esc` to cancel

#### Edit Due Date
1. Select a task
2. Press `D`
3. Enter date (YYYY-MM-DD format or keywords)
4. Press `Enter` or leave blank to clear

#### Edit Tags
1. Select a task
2. Press `T`
3. Enter comma-separated tags: `work, urgent, meeting`
4. Press `Enter` (empty clears tags)

#### Edit Description
1. Select a task
2. Press `n`
3. Type or modify the description
4. Press `Enter` to save

### Task Properties

#### Priority

Cycle priority with `p`:
```
None → Low → Medium → High → Urgent → None
```

Priority affects:
- Visual indicator (! symbols)
- Sort order when sorting by priority
- Dashboard statistics

#### Status

The task status lifecycle:

```
       ┌─────────────────────┐
       │                     │
       ▼                     │
    ┌──────┐   toggle    ┌───────┐
    │ Todo │────────────▶│ Done  │
    └──────┘             └───────┘
       │                     ▲
       │ (manual)            │ (manual)
       ▼                     │
  ┌────────────┐    ┌────────────┐
  │In Progress │    │ Cancelled  │
  └────────────┘    └────────────┘
       │
       │ (manual)
       ▼
   ┌─────────┐
   │ Blocked │
   └─────────┘
```

- Press `x` or `Space` to toggle between Todo ↔ Done
- Other statuses can be set via editing

### Completing Tasks

#### Single Task
- Select the task and press `x` or `Space`
- The task moves to "Done" status
- If hiding completed, it disappears from view

#### Recurring Tasks
When you complete a recurring task:
1. The current task is marked Done
2. A new task is created with the next due date
3. The new task appears in your list

### Deleting Tasks

1. Select the task
2. Press `d`
3. Confirm deletion in the popup

**Note**: Deletion is permanent but can be undone with `u` or `Ctrl+z`.

**Deletion Restrictions**:
- Tasks with subtasks cannot be deleted directly
- Delete subtasks first, then the parent

### Multi-Select & Bulk Operations

For operations on multiple tasks:

1. Press `v` to enter multi-select mode
2. Navigate and press `Space` to toggle selection on each task
3. Selected tasks show a `●` indicator
4. Perform bulk operations:
   - `d` - Delete all selected
   - `m` - Move all to a project
5. Press `v` or `Ctrl+v` to exit multi-select

**Quick Select All:**
- Press `V` to select all visible tasks at once

### Task Templates

Create tasks quickly from predefined templates:

1. Press `Ctrl+n` to open template picker
2. Navigate with `j`/`k` or press the template number
3. Press `Enter` to create from template
4. Edit the generated task title as needed

**Built-in Templates:**
| Template | Priority | Tags | Due |
|----------|----------|------|-----|
| Bug Fix | High | #bug | - |
| Feature | Medium | #feature | - |
| Review | Medium | - | Tomorrow |
| Meeting Notes | Low | #meeting | - |
| Daily Task | Low | - | Today |
| Weekly Task | Low | - | +7 days |
| Urgent | Urgent | #urgent | Today |
| Research | Low | #research | - |

---

## 5. Organization

### Projects

Projects group related tasks and appear in the sidebar.

#### Create a Project
1. Press `P`
2. Enter the project name
3. Press `Enter`

#### Edit a Project
1. Focus the sidebar (`h`)
2. Select the project
3. Press `E`
4. Modify the name
5. Press `Enter`

#### Delete a Project
1. Focus the sidebar (`h`)
2. Select the project
3. Press `X`
4. Confirm deletion

**Note**: Deleting a project unassigns its tasks (tasks are not deleted).

#### Project Status
Projects have their own lifecycle:
- **Active** - Currently being worked on
- **On Hold** - Temporarily paused
- **Completed** - All work finished
- **Archived** - No longer relevant

#### Assign Task to Project
1. Select a task
2. Press `m` (move)
3. Choose the destination project
4. Press `Enter`

### Tags

Tags are flexible labels for cross-cutting concerns.

#### Add Tags to a Task
1. Select the task
2. Press `T`
3. Enter comma-separated tags: `work, urgent, q4`
4. Press `Enter`

#### Quick Add with Tags
```
New feature #frontend #priority
```

#### Filter by Tags
1. Press `#`
2. Enter tag(s) to filter by
3. Press `Enter`

Tasks matching any of the tags are shown.

#### Clear Tag Filter
Press `Ctrl+t` to remove the tag filter.

### Subtasks & Hierarchies

Break complex tasks into smaller pieces:

#### Create a Subtask
1. Select the parent task
2. Press `A` (capital A)
3. Enter the subtask title
4. Press `Enter`

The subtask appears indented under its parent.

#### Hierarchy Display
```
▶ [ ] Main project task
    [ ] Subtask 1
    [ ] Subtask 2
        [ ] Sub-subtask
```

#### Completing Parent Tasks
When you complete a parent task:
- All subtasks are also marked complete
- This cascades down the entire hierarchy

When you uncomplete a parent:
- Only the parent changes
- Subtasks remain in their current state

### Task Dependencies

Block a task until others are complete:

#### Set Dependencies
1. Select the task to block
2. Press `B`
3. Enter the IDs of blocking tasks (comma-separated)
4. Press `Enter`

Blocked tasks show `[!]` status and cannot be completed until blockers are done.

#### View Blocked Tasks
Select "Blocked" from the sidebar to see all blocked tasks.

### Task Chains

Link tasks in sequence for workflows:

#### Create a Chain
1. Select a task
2. Press `Ctrl+l` to link to next task
3. Select the task that should follow
4. Press `Enter`

Chains visualize work sequences and can be exported as diagrams.

#### Unlink from Chain
1. Select a chained task
2. Press `Ctrl+u` to unlink

### Recurring Tasks

Set up tasks that repeat automatically:

#### Configure Recurrence
1. Select a task
2. Press `R`
3. Choose pattern:
   - `d` - Daily
   - `w` - Weekly (same day of week)
   - `m` - Monthly (same day of month)
   - `y` - Yearly (same date)
   - `0` - Clear recurrence

Recurring tasks show `↻` indicator.

#### How Recurrence Works
1. You complete a recurring task
2. TaskFlow creates a new task automatically
3. The new task has the next due date based on the pattern
4. Original task stays in completed state

---

## 6. Views & Filtering

### Available Views

Access views from the sidebar (`h` to focus, `j`/`k` to navigate, `Enter` to select):

| View | Shows |
|------|-------|
| All Tasks | Every task in the system |
| Today | Tasks due today |
| Upcoming | Tasks with future due dates |
| Overdue | Tasks past their due date |
| Calendar | Monthly calendar with tasks |
| Dashboard | Statistics and overview |
| Reports | Detailed analytics |
| Scheduled | Tasks with scheduled dates |
| Blocked | Tasks waiting on dependencies |
| Untagged | Tasks without any tags |
| No Project | Tasks not assigned to a project |
| Recent | Tasks modified in last 7 days |

### Calendar View

The calendar provides a monthly overview:

```
┌─────────────────────────────────────────┐
│           December 2025                 │
├─────┬─────┬─────┬─────┬─────┬─────┬─────┤
│ Sun │ Mon │ Tue │ Wed │ Thu │ Fri │ Sat │
├─────┼─────┼─────┼─────┼─────┼─────┼─────┤
│     │  1  │  2  │  3• │  4  │  5• │  6  │
│  7  │  8• │  9  │ 10  │ 11  │ 12  │ 13  │
│ 14  │ 15  │[16]•│ 17  │ 18  │ 19  │ 20  │
│ 21  │ 22  │ 23  │ 24• │ 25  │ 26  │ 27  │
│ 28  │ 29  │ 30  │ 31  │     │     │     │
└─────┴─────┴─────┴─────┴─────┴─────┴─────┘
```

- `•` indicates days with tasks
- `[16]` is the selected day
- Overdue days are highlighted in red

#### Calendar Navigation
| Key | Action |
|-----|--------|
| `h`/`l` or `←`/`→` | Previous/next day |
| `j`/`k` or `↓`/`↑` | Previous/next week |
| `<`/`>` | Previous/next month |
| `Enter` | Focus task list for selected day |
| `Esc` | Return focus to calendar grid |

### Searching Tasks

#### Start a Search
1. Press `/`
2. Type your search query
3. Press `Enter`

The task list filters to show matching tasks.

#### Search Behavior
- Searches task titles and descriptions
- Case-insensitive matching
- Partial word matches

#### Clear Search
Press `Ctrl+l` to clear the search and show all tasks.

### Filtering by Tags

#### Apply Tag Filter
1. Press `#`
2. Enter one or more tags (comma-separated)
3. Press `Enter`

Example: `#work, urgent` shows tasks with either tag.

#### Clear Tag Filter
Press `Ctrl+t` to remove the filter.

### Sorting Options

#### Change Sort Field
Press `s` to cycle through:
```
Created → Updated → Due Date → Priority → Title → Status
```

#### Toggle Sort Order
Press `S` to switch between:
- Ascending (A→Z, oldest first, lowest priority first)
- Descending (Z→A, newest first, highest priority first)

### Show/Hide Completed

Press `c` to toggle visibility of completed tasks.

- **Showing completed**: All tasks visible, completed tasks show `[x]`
- **Hiding completed**: Only incomplete tasks shown

The footer indicates the current mode.

---

## 7. Time Management

### Time Tracking

Track how long you spend on tasks with the built-in timer.

#### Start Tracking
1. Select a task
2. Press `t` to start the timer
3. A `⏱` appears next to the task
4. The footer shows elapsed time

#### Stop Tracking
1. Press `t` again on the tracked task
2. Time is recorded and added to the task's total

#### Switch Tasks
Starting tracking on a new task automatically stops the previous timer.

#### Timer Persistence
**Important**: The timer persists across application restarts. If you close TaskFlow while tracking, the timer continues. When you reopen, it picks up where you left off.

### Time Log Editor

View and edit all time entries for a task:

1. Select a task
2. Press `L` to open the time log

The editor shows:
```
┌─────────────────────────────────────────┐
│ Time Log - Fix login bug                │
├─────────────────────────────────────────┤
│ Dec 15, 2025                            │
│   09:30 - 11:15  (1h 45m)              │
│   14:00 - 15:30  (1h 30m)              │
│                                         │
│ Dec 14, 2025                            │
│   10:00 - 12:30  (2h 30m)              │
├─────────────────────────────────────────┤
│ Total: 5h 45m                           │
└─────────────────────────────────────────┘
```

#### Time Log Actions
| Key | Action |
|-----|--------|
| `a` | Add new time entry |
| `e` | Edit selected entry |
| `d` | Delete selected entry |
| `j`/`k` | Navigate entries |
| `Esc` | Close log |

### Pomodoro Timer

The Pomodoro Technique helps maintain focus through timed work sessions.

#### How It Works
1. **Work phase**: 25 minutes of focused work
2. **Short break**: 5 minutes rest
3. **Repeat**: After 4 cycles, take a long break (15 minutes)

#### Start a Pomodoro Session
1. Select a task (optional)
2. Press `F5` to start
3. The timer appears in the header/footer

#### Timer Display
```
🍅 23:45 [2/4]
```
- 🍅 = Work phase (☕ = short break, 🌴 = long break)
- 23:45 = Time remaining
- [2/4] = Completed cycles / goal

#### Pomodoro Controls
| Key | Action |
|-----|--------|
| `F5` | Start session |
| `F6` | Pause/Resume |
| `F7` | Skip current phase |
| `F8` | Stop session |
| `+` | Increase cycle goal |
| `-` | Decrease cycle goal |

#### Configuration
Customize timings in `~/.config/taskflow/config.toml`:
```toml
[pomodoro]
work_minutes = 25
short_break_minutes = 5
long_break_minutes = 15
cycles_before_long_break = 4
```

### Focus Mode

Minimize distractions with a single-task view:

1. Select a task
2. Press `f` to enter focus mode

Focus mode displays:
- Only the selected task (large, centered)
- Active timer (if tracking time)
- Pomodoro status (if running)
- Minimal UI

Press `f` or `Esc` to exit focus mode.

---

## 8. Analytics & Reports

### Dashboard Overview

Access the Dashboard from the sidebar to see at-a-glance statistics:

```
┌────────────────────────────────────────────────────────────────┐
│                         Dashboard                              │
├─────────────────────┬─────────────────────┬────────────────────┤
│   Completion        │   Time Tracking     │    This Week       │
│   ────────────      │   ─────────────     │    ──────────      │
│   Total: 47         │   Tracked: 12h 30m  │    Created: 8      │
│   Done: 23 (49%)    │   Avg/task: 32m     │    Completed: 12   │
│   Overdue: 3        │   Currently: —      │    Active: 24      │
├─────────────────────┴─────────────────────┴────────────────────┤
│                    Status Distribution                         │
│   ─────────────────────────────────────────────────────────    │
│   Todo        ████████████████████░░░░░  18                   │
│   In Progress ██████░░░░░░░░░░░░░░░░░░░   5                   │
│   Blocked     ██░░░░░░░░░░░░░░░░░░░░░░░   1                   │
│   Done        ████████████████████████░  23                   │
├────────────────────────────────────────────────────────────────┤
│                    Projects                                    │
│   Work        ████████████░░░░  65%  (13/20)                  │
│   Personal    ██████████████░░  78%  (7/9)                    │
│   Home        ████░░░░░░░░░░░░  22%  (2/9)                    │
└────────────────────────────────────────────────────────────────┘
```

### Reports View

Access detailed analytics from the sidebar → Reports.

The Reports view has multiple panels (navigate with `Tab`/`Shift+Tab`):

#### Overview Panel
Summary statistics:
- Total tasks created
- Completion rate
- Tasks by status
- Tasks by priority

#### Velocity Panel
Track your productivity trends:
- Weekly completion rate
- Monthly trends
- Completion velocity (tasks/week)
- Trend direction (improving/declining)

#### Tags Panel
Tag usage statistics:
- Most used tags
- Completion rate per tag
- Tag frequency chart

Tags are sorted by count, with alphabetical sorting for equal counts.

#### Time Panel
Time tracking analysis:
- Total time tracked
- Time per project
- Peak productivity hours
- Most productive day of week

#### Insights Panel
Productivity patterns:
- Current streak (consecutive days completing tasks)
- Longest streak
- Best day of week
- Peak productivity hour
- Average tasks per day

#### Estimation Panel
If you use estimated times:
- Accuracy of estimates
- Over/under estimation patterns
- Variance analysis

### Exporting Reports

#### Markdown Report
Press `Ctrl+p` to export a report as Markdown (`.md` file).

#### HTML Report
Press `Ctrl+h` to export a formatted HTML report.

Both formats include:
- Summary statistics
- Charts and graphs (ASCII in MD, visual in HTML)
- Task breakdown
- Time tracking summary

---

## 9. Data Management

### Storage Backends

TaskFlow supports multiple storage formats:

| Backend | Format | File | Best For |
|---------|--------|------|----------|
| JSON | Single file | `.json` | Default, fast, compact |
| YAML | Single file | `.yaml` | Human-readable, easy editing |
| SQLite | Database | `.db` | Large datasets, queries |
| Markdown | Directory | folder | Git integration, external editing |

#### Specify Backend on Launch
```bash
# JSON (default)
taskflow --backend json --data ~/tasks.json

# YAML
taskflow --backend yaml --data ~/tasks.yaml

# SQLite
taskflow --backend sqlite --data ~/tasks.db

# Markdown
taskflow --backend markdown --data ~/tasks/
```

### Choosing a Backend

**Use JSON if:**
- You want the default, hassle-free option
- Your task count is under 1000
- You don't need to edit data externally

**Use YAML if:**
- You want human-readable data files
- You might edit tasks manually
- You're comfortable with YAML syntax

**Use SQLite if:**
- You have 1000+ tasks
- Performance is critical
- You want robust data integrity

**Use Markdown if:**
- You want to track tasks in Git
- You use tools like Obsidian
- You prefer one file per task

### Data Location

Default locations by platform:
- **Linux**: `~/.local/share/taskflow/`
- **macOS**: `~/Library/Application Support/taskflow/`
- **Windows**: `%APPDATA%\taskflow\`

Configuration is always in:
- **Linux/macOS**: `~/.config/taskflow/`
- **Windows**: `%APPDATA%\taskflow\config\`

### Import

#### Import CSV
1. Press `I`
2. Enter the file path
3. Review the import preview
4. Confirm import

CSV format expected:
```csv
title,status,priority,due_date,tags
"Buy groceries",todo,low,2025-01-15,"errands,personal"
```

#### Import iCalendar (ICS)
1. Press `Alt+I`
2. Enter the file path
3. Review and confirm

ICS import brings in VTODO items from calendar files.

### Export

#### Export to CSV
1. Press `Ctrl+e`
2. Enter the destination path
3. File is created with all tasks

#### Export to iCalendar
1. Press `Ctrl+i`
2. Enter the destination path
3. Use in calendar applications

#### Export Task Chains

**Graphviz DOT format:**
Press `Ctrl+g` to export task chains as DOT format for visualization.

**Mermaid format:**
Press `Ctrl+m` to export as Mermaid diagram syntax.

### Backup Recommendations

1. **Regular backups**: Copy your data file periodically
2. **Git tracking**: Use Markdown backend with a Git repository
3. **Cloud sync**: Store data in a synced folder (Dropbox, etc.)
4. **Export**: Periodically export to CSV as a backup

---

## 10. Customization

### Configuration File

TaskFlow's configuration lives at `~/.config/taskflow/config.toml`:

```toml
# Storage settings
backend = "json"                    # json, yaml, sqlite, markdown
data_path = "tasks.json"            # relative to config or absolute

# UI defaults
show_sidebar = true                 # Show sidebar on startup
show_completed = false              # Hide completed tasks by default
default_priority = "none"           # Default priority for new tasks

# Auto-save
auto_save_interval = 300            # Seconds between auto-saves (0 = disabled)

# Theme
theme = "default"                   # Theme name

# Pomodoro settings
[pomodoro]
work_minutes = 25
short_break_minutes = 5
long_break_minutes = 15
cycles_before_long_break = 4
```

### Keybindings Editor

Customize any keybinding:

1. Press `Ctrl+k` to open the keybindings editor
2. Navigate to the action you want to change
3. Press `Enter` to edit
4. Press the new key combination
5. Press `Enter` to confirm
6. Press `Ctrl+s` to save changes

#### Reset Keybindings
- Reset single binding: Select and press `r`
- Reset all bindings: Press `R`

### Custom Keybindings File

Keybindings are stored in `~/.config/taskflow/keybindings.toml`:

```toml
[bindings]
# Navigation
move_down = "j"
move_up = "k"
focus_sidebar = "h"
focus_list = "l"

# Task actions
toggle_complete = "x"
create_task = "a"
delete_task = "d"
edit_title = "e"

# Add your customizations here
```

### Themes & Colors

#### Theme Configuration

Create custom themes in `~/.config/taskflow/themes/`:

```toml
# ~/.config/taskflow/themes/dark.toml
[colors]
background = "#1a1a2e"
foreground = "#eaeaea"
accent = "#00d4ff"
secondary = "#7b68ee"
success = "#00ff7f"
warning = "#ffd700"
danger = "#ff4757"
muted = "#4a4a6a"
border = "#3a3a5a"

[priority]
urgent = "#ff0000"
high = "#ff6b6b"
medium = "#ffd93d"
low = "#6bcb77"
none = "#808080"

[status]
todo = "#808080"
in_progress = "#00d4ff"
blocked = "#ff4757"
done = "#00ff7f"
cancelled = "#4a4a6a"
```

#### Color Formats

Specify colors using:
- **Named**: `red`, `blue`, `cyan`, `green`, `yellow`, `magenta`, `white`, `black`
- **Hex**: `#ff5500`, `#3498db`
- **RGB**: `{ r = 100, g = 150, b = 200 }`

#### Apply Theme

In `config.toml`:
```toml
theme = "dark"  # Uses themes/dark.toml
```

### Shell Completions

Generate tab completions for your shell:

**Bash:**
```bash
taskflow completion bash > ~/.local/share/bash-completion/completions/taskflow
source ~/.local/share/bash-completion/completions/taskflow
```

**Zsh:**
```bash
mkdir -p ~/.zsh/completions
taskflow completion zsh > ~/.zsh/completions/_taskflow
# Add to ~/.zshrc: fpath=(~/.zsh/completions $fpath)
```

**Fish:**
```bash
taskflow completion fish > ~/.config/fish/completions/taskflow.fish
```

---

## 11. Advanced Features

### Macros

Record and replay action sequences:

#### Recording a Macro
1. Press `Ctrl+q` to start recording
2. Press a digit (`0`-`9`) to select the storage slot
3. Perform the actions you want to record
4. Press `Ctrl+q` again to stop recording
5. Press the same digit to save

The footer shows `[REC]` while recording.

#### Playing a Macro
Press `@` followed by the slot number (`@0`, `@1`, etc.)

#### Macro Ideas
- `@1`: Create a bug report task with standard tags
- `@2`: Move selected tasks to "Archive" project
- `@3`: Set priority to high and add #urgent tag

### Undo/Redo System

TaskFlow maintains a history of your actions:

| Key | Action |
|-----|--------|
| `u` or `Ctrl+z` | Undo last action |
| `U` or `Ctrl+r` | Redo undone action |

**Undoable actions include:**
- Creating/deleting tasks
- Editing task properties
- Creating/deleting projects
- Time tracking operations
- Moving tasks between projects

**History limit:** 50 actions (configurable)

### CLI Options

```bash
taskflow [OPTIONS]

Options:
    --data <PATH>       Path to data file/directory
    --backend <TYPE>    Storage backend (json, yaml, sqlite, markdown)
    --demo              Load sample data for exploration

Subcommands:
    completion <SHELL>  Generate shell completions (bash, zsh, fish)
```

**Examples:**
```bash
# Custom data location
taskflow --data ~/Dropbox/tasks.json

# Use SQLite for large datasets
taskflow --backend sqlite --data ~/tasks.db

# Try TaskFlow with demo data
taskflow --demo

# Generate completions
taskflow completion bash > ~/.local/share/bash-completion/completions/taskflow
```

---

## Appendices

### Appendix A: Complete Keybindings Reference

#### Navigation
| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `g` | Go to first item |
| `G` | Go to last item |
| `Ctrl+u` / `Page Up` | Page up |
| `Ctrl+d` / `Page Down` | Page down |
| `h` / `←` | Focus sidebar |
| `l` / `→` | Focus task list |
| `Enter` | Select/confirm |
| `Esc` | Cancel/close |

#### Task Operations
| Key | Action |
|-----|--------|
| `a` | Add new task |
| `A` | Add subtask |
| `e` | Edit task title |
| `d` | Delete task |
| `x` / `Space` | Toggle complete |
| `p` | Cycle priority |
| `D` | Edit due date |
| `T` | Edit tags |
| `n` | Edit description |
| `m` | Move to project |
| `B` | Edit dependencies |
| `R` | Set recurrence |

#### Project Operations
| Key | Action |
|-----|--------|
| `P` | Create project |
| `E` | Edit project (sidebar) |
| `X` | Delete project (sidebar) |

#### Time Tracking
| Key | Action |
|-----|--------|
| `t` | Toggle time tracking |
| `L` | Open time log |

#### Pomodoro
| Key | Action |
|-----|--------|
| `F5` | Start Pomodoro |
| `F6` | Pause/Resume |
| `F7` | Skip phase |
| `F8` | Stop Pomodoro |

#### Search & Filter
| Key | Action |
|-----|--------|
| `/` | Search tasks |
| `Ctrl+l` | Clear search |
| `#` | Filter by tag |
| `Ctrl+t` | Clear tag filter |
| `s` | Cycle sort field |
| `S` | Toggle sort order |

#### View Controls
| Key | Action |
|-----|--------|
| `b` | Toggle sidebar |
| `c` | Toggle completed |
| `f` | Toggle focus mode |
| `?` | Show help |

#### Multi-Select
| Key | Action |
|-----|--------|
| `v` | Toggle multi-select mode |
| `V` | Select all |
| `Space` | Toggle selection (in mode) |
| `Ctrl+v` | Clear selection |

#### Export/Import
| Key | Action |
|-----|--------|
| `Ctrl+e` | Export CSV |
| `Ctrl+i` | Export ICS |
| `I` | Import CSV |
| `Alt+I` | Import ICS |
| `Ctrl+g` | Export chains (DOT) |
| `Ctrl+m` | Export chains (Mermaid) |
| `Ctrl+p` | Export report (Markdown) |
| `Ctrl+h` | Export report (HTML) |

#### Macros
| Key | Action |
|-----|--------|
| `Ctrl+q` | Start/stop recording |
| `@0`-`@9` | Play macro |

#### System
| Key | Action |
|-----|--------|
| `u` / `Ctrl+z` | Undo |
| `U` / `Ctrl+r` | Redo |
| `Ctrl+s` | Save |
| `Ctrl+k` | Keybindings editor |
| `Ctrl+n` | Template picker |
| `q` | Quit |

### Appendix B: Quick Add Syntax Reference

| Syntax | Meaning | Example |
|--------|---------|---------|
| `#tag` | Add tag | `#work`, `#bug`, `#urgent` |
| `!priority` | Set priority | `!urgent`, `!high`, `!med`, `!low` |
| `due:date` | Set due date | `due:tomorrow`, `due:friday` |
| `sched:date` | Set scheduled date | `sched:monday` |
| `@project` | Assign project | `@Work`, `@Personal` |

**Complete Example:**
```
Fix login bug #bug #security !urgent due:today @Backend
```

### Appendix C: Date Parsing Reference

| Input | Interpretation |
|-------|----------------|
| `today`, `tod` | Today |
| `tomorrow`, `tom` | Tomorrow |
| `yesterday` | Yesterday |
| `monday`, `mon` | Next Monday |
| `tuesday`, `tue` | Next Tuesday |
| `wednesday`, `wed` | Next Wednesday |
| `thursday`, `thu` | Next Thursday |
| `friday`, `fri` | Next Friday |
| `saturday`, `sat` | Next Saturday |
| `sunday`, `sun` | Next Sunday |
| `next week` | Same day, next week |
| `next month` | Same day, next month |
| `next year` | Same day, next year |
| `in 3 days` | 3 days from now |
| `in 2 weeks` | 2 weeks from now |
| `in 1 month` | 1 month from now |
| `eow` | End of week (Sunday) |
| `eom` | End of month |
| `eoy` | End of year |
| `1st`, `15th`, `22nd` | Day of current month |
| `2025-01-15` | Specific date (ISO) |
| `01/15`, `01-15` | Month/Day current year |

### Appendix D: Troubleshooting & FAQ

#### Q: Where is my data stored?
**A:** By default:
- Linux: `~/.local/share/taskflow/tasks.json`
- macOS: `~/Library/Application Support/taskflow/tasks.json`
- Windows: `%APPDATA%\taskflow\tasks.json`

Use `--data <path>` to specify a custom location.

#### Q: How do I backup my tasks?
**A:** Options:
1. Copy your data file to a backup location
2. Export to CSV (`Ctrl+e`) periodically
3. Use the Markdown backend with Git
4. Store data in a cloud-synced folder

#### Q: Why is TaskFlow slow with many tasks?
**A:** Try:
1. Switch to SQLite backend for 1000+ tasks
2. Hide completed tasks (`c`)
3. Filter to specific projects
4. Increase auto-save interval or disable it

#### Q: Can I sync across devices?
**A:** TaskFlow doesn't have built-in sync, but you can:
1. Store data in Dropbox/Google Drive/iCloud
2. Use the Markdown backend with Git
3. Use a shared SQLite database (careful of conflicts)

#### Q: My keybindings aren't working
**A:** Check:
1. Terminal key handling (some keys may be intercepted)
2. `~/.config/taskflow/keybindings.toml` for conflicts
3. Press `Ctrl+k` to verify current bindings
4. Reset with `R` in keybindings editor

#### Q: How do I migrate to a different backend?
**A:**
1. Export all tasks to CSV: `Ctrl+e`
2. Start TaskFlow with new backend: `taskflow --backend sqlite --data tasks.db`
3. Import CSV: `I`

#### Q: The timer kept running while I was away
**A:** This is by design—the timer persists across restarts. Open the time log (`L`) and edit the entry to fix incorrect times.

#### Q: Can I use TaskFlow over SSH?
**A:** Yes! TaskFlow works in any terminal, including remote sessions. Make sure your SSH connection supports UTF-8.

#### Q: How do I report a bug?
**A:** Open an issue at: https://github.com/anthropics/claude-code/issues

---

*Last updated: December 2025*
*TaskFlow User Manual v1.0*
