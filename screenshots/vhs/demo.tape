Output screenshots/vhs/demo.gif

Set FontSize 14
Set Width 1600
Set Height 900
Set Theme "Catppuccin Mocha"
Set TypingSpeed 25ms
Set PlaybackSpeed 1

Type "./target/debug/taskflow --demo"
Enter
Sleep 2s
# Dismiss startup overdue alert (fires because demo data has overdue tasks)
Escape
Sleep 300ms

# ── Task List ────────────────────────────────────────────────────────────────
Screenshot screenshots/png/task_list.png
Sleep 1.5s

# ── Today ────────────────────────────────────────────────────────────────────
# Escape first: exits any special view mode (no-op on TaskList), then hg navigates sidebar
Escape
Sleep 200ms
Type "hg"
Type "j"
Enter
Sleep 800ms
Screenshot screenshots/png/today.png
Sleep 1.5s

# ── Upcoming ─────────────────────────────────────────────────────────────────
Escape
Sleep 200ms
Type "hg"
Type "jj"
Enter
Sleep 800ms
Screenshot screenshots/png/upcoming.png
Sleep 1.5s

# ── Overdue ──────────────────────────────────────────────────────────────────
Escape
Sleep 200ms
Type "hg"
Type "jjj"
Enter
Sleep 800ms
Screenshot screenshots/png/overdue.png
Sleep 1.5s

# ── Scheduled ────────────────────────────────────────────────────────────────
Escape
Sleep 200ms
Type "hg"
Type "jjjj"
Enter
Sleep 800ms
Screenshot screenshots/png/scheduled.png
Sleep 1.5s

# ── Calendar ─────────────────────────────────────────────────────────────────
Escape
Sleep 200ms
Type "hg"
Type "jjjjj"
Enter
Sleep 800ms
Screenshot screenshots/png/calendar.png
Sleep 1.5s

# ── Dashboard ────────────────────────────────────────────────────────────────
# Calendar intercepts h (CalendarPrevDay) — Escape exits to TaskList first
Escape
Sleep 200ms
Type "hg"
Type "jjjjjj"
Enter
Sleep 800ms
Screenshot screenshots/png/dashboard.png
Sleep 1.5s

# ── Reports ──────────────────────────────────────────────────────────────────
Escape
Sleep 200ms
Type "hg"
Type "jjjjjjj"
Enter
Sleep 800ms
Screenshot screenshots/png/reports.png
Sleep 1.5s

# ── Habits ───────────────────────────────────────────────────────────────────
# Reports Esc → GoToView(TaskList), then sidebar nav works
Escape
Sleep 200ms
Type "hg"
Type "jjjjjjjj"
Enter
Sleep 800ms
Screenshot screenshots/png/habits.png
Sleep 1.5s

# ── Goals ────────────────────────────────────────────────────────────────────
# Habits intercepts j (HabitDown) — Escape exits to TaskList first
Escape
Sleep 200ms
Type "hg"
Type "jjjjjjjjj"
Enter
Sleep 800ms
Screenshot screenshots/png/goals.png
Sleep 1.5s

# ── Blocked ──────────────────────────────────────────────────────────────────
# Goals intercepts h and j — Escape exits to TaskList first
Escape
Sleep 200ms
Type "hg"
Type "jjjjjjjjjj"
Enter
Sleep 800ms
Screenshot screenshots/png/blocked.png
Sleep 1.5s

# ── Untagged ─────────────────────────────────────────────────────────────────
Escape
Sleep 200ms
Type "hg"
Type "jjjjjjjjjjj"
Enter
Sleep 800ms
Screenshot screenshots/png/untagged.png
Sleep 1.5s

# ── Recent ───────────────────────────────────────────────────────────────────
Escape
Sleep 200ms
Type "hg"
Type "jjjjjjjjjjjjj"
Enter
Sleep 800ms
Screenshot screenshots/png/recent.png
Sleep 1.5s

# ── Kanban ───────────────────────────────────────────────────────────────────
Escape
Sleep 200ms
Type "hg"
Type "jjjjjjjjjjjjjj"
Enter
Sleep 800ms
Screenshot screenshots/png/kanban.png
Sleep 1.5s

# ── Eisenhower ───────────────────────────────────────────────────────────────
# Kanban intercepts h and j — Escape exits to TaskList first
Escape
Sleep 200ms
Type "hg"
Type "jjjjjjjjjjjjjjj"
Enter
Sleep 800ms
Screenshot screenshots/png/eisenhower.png
Sleep 1.5s

# ── Weekly Planner ───────────────────────────────────────────────────────────
# Eisenhower intercepts h and j — Escape exits to TaskList first
Escape
Sleep 200ms
Type "hg"
Type "jjjjjjjjjjjjjjjj"
Enter
Sleep 800ms
Screenshot screenshots/png/weekly_planner.png
Sleep 1.5s

# ── Timeline ─────────────────────────────────────────────────────────────────
# WeeklyPlanner intercepts h and j — Escape exits to TaskList first
Escape
Sleep 200ms
Type "hg"
Type "jjjjjjjjjjjjjjjjj"
Enter
Sleep 800ms
Screenshot screenshots/png/timeline.png
Sleep 1.5s

# ── Snoozed ──────────────────────────────────────────────────────────────────
# Timeline intercepts h and j — Escape exits to TaskList first
Escape
Sleep 200ms
Type "hg"
Type "jjjjjjjjjjjjjjjjjj"
Enter
Sleep 800ms
Screenshot screenshots/png/snoozed.png
Sleep 1.5s

# ── Heatmap ──────────────────────────────────────────────────────────────────
Escape
Sleep 200ms
Type "hg"
Type "jjjjjjjjjjjjjjjjjjj"
Enter
Sleep 800ms
Screenshot screenshots/png/heatmap.png
Sleep 1.5s

# ── Forecast ─────────────────────────────────────────────────────────────────
Escape
Sleep 200ms
Type "hg"
Type "jjjjjjjjjjjjjjjjjjjj"
Enter
Sleep 800ms
Screenshot screenshots/png/forecast.png
Sleep 1.5s

# ── Network ──────────────────────────────────────────────────────────────────
Escape
Sleep 200ms
Type "hg"
Type "jjjjjjjjjjjjjjjjjjjjj"
Enter
Sleep 800ms
Screenshot screenshots/png/network.png
Sleep 1.5s

# ── Burndown ─────────────────────────────────────────────────────────────────
# Network intercepts h and j — Escape exits to TaskList first
Escape
Sleep 200ms
Type "hg"
Type "jjjjjjjjjjjjjjjjjjjjjj"
Enter
Sleep 800ms
Screenshot screenshots/png/burndown.png
Sleep 1.5s

# ── Duplicates ───────────────────────────────────────────────────────────────
Escape
Sleep 200ms
Type "hg"
Type "jjjjjjjjjjjjjjjjjjjjjjj"
Enter
Sleep 800ms
Screenshot screenshots/png/duplicates.png
Sleep 1.5s

# ── Git TODOs ────────────────────────────────────────────────────────────────
Escape
Sleep 200ms
Type "hg"
Type "jjjjjjjjjjjjjjjjjjjjjjjj"
Enter
Sleep 800ms
Screenshot screenshots/png/git_todos.png
Sleep 1.5s

# ── Focus Mode ───────────────────────────────────────────────────────────────
# GitTodos only intercepts 'r', so Escape is a no-op here
Escape
Sleep 200ms
Type "hg"
Enter
Sleep 300ms
Type "f"
Sleep 800ms
Screenshot screenshots/png/focus.png
Sleep 1.5s
Type "f"
Sleep 300ms

# ── Task Detail popup ────────────────────────────────────────────────────────
# focus_pane=TaskList after focus mode exit, 'i' opens task detail
Type "l"
Sleep 300ms
Type "i"
Sleep 800ms
Screenshot screenshots/png/task_detail.png
Sleep 1.5s
Type "q"
Sleep 300ms

# ── Create Task ──────────────────────────────────────────────────────────────
Type "a"
Sleep 500ms
Type "Fix login bug"
Sleep 800ms
Screenshot screenshots/png/create_task.png
Sleep 1.5s
Escape
Sleep 300ms

# ── Search ───────────────────────────────────────────────────────────────────
Type "/"
Sleep 300ms
Type "auth"
Sleep 800ms
Screenshot screenshots/png/search.png
Sleep 1.5s
Escape
Sleep 300ms

# ── Command Palette ──────────────────────────────────────────────────────────
Type ":"
Sleep 500ms
Type "toggle"
Sleep 800ms
Screenshot screenshots/png/command_palette.png
Sleep 1.5s
Escape
Sleep 300ms

# ── Help ─────────────────────────────────────────────────────────────────────
Type "?"
Sleep 500ms
Screenshot screenshots/png/help.png
Sleep 1.5s
Type "q"
Sleep 300ms

Type "q"
