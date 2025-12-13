# UI/UX Improvement Features - Analysis

## Overview

Analysis of UI/UX improvement features for TaskFlow's Ratatui-based terminal interface.

---

## Current Architecture

| Component | Location | Description |
|-----------|----------|-------------|
| Layout System | `src/ui/view/layout.rs` | 3-tier structure (header/content/footer) |
| Theme System | `src/config/theme.rs` | TOML-based with color specs |
| Input Handling | `src/bin/taskflow/input/` | Keybinding-based, 70+ actions |
| Dashboard | `src/ui/components/dashboard/` | 7 fixed panels |
| Network View | `src/ui/components/network/` | ASCII dependency graph |

---

## Feature 1: Split Panes

**Goal**: View multiple lists side-by-side (e.g., Today + Calendar)

### What Exists
- `model.focus_pane` enum (TaskList, Sidebar only)
- Fixed 25-column sidebar width
- `LayoutCache` for mouse events

### What Needs to Be Added

```rust
pub enum PaneLayout {
    Single,
    HorizontalSplit(f32),  // ratio
    VerticalSplit(f32),
    Grid(u8, u8),          // rows x cols
}

pub struct PaneView {
    pub view_id: ViewId,
    pub filter_state: FilterState,
    pub sort_state: SortState,
    pub scroll_position: u16,
}

// In Model:
pub active_panes: Vec<PaneView>,
pub pane_layout: PaneLayout,
pub focused_pane: usize,
```

### New Messages
```rust
NavigationMessage::NextPane
NavigationMessage::PreviousPane
NavigationMessage::SelectPane(index)
```

### Key Changes
- Redesign `render_main_content()` for multiple panes
- Coordinate layout cache for multiple areas
- Input routing based on active pane

### Complexity: Medium-High (80-120 hours)

---

## Feature 2: Theming Engine

**Goal**: User-created themes with more control

### What Exists
- `Theme` struct with `ThemeColors`, `PriorityColors`, `StatusColors`
- 3 color formats: named, hex, RGB tuples
- TOML serialization
- 15+ color fields

### What Needs to Be Added

```rust
// Extended color categories
pub struct ThemeColors {
    // ... existing fields ...

    // NEW:
    pub border_inactive: ColorSpec,
    pub border_active: ColorSpec,
    pub highlight_subtle: ColorSpec,
    pub tag_colors: HashMap<String, ColorSpec>,
    pub gradient_start: ColorSpec,
    pub gradient_end: ColorSpec,
}

// Component-level customization
pub struct ComponentTheme {
    pub borders: BorderStyle,  // Rounded, Double, etc.
    pub shadows: bool,
    pub animations: bool,
}
```

### Theme Editor UI
- Interactive color picker
- Live preview
- Export/import themes
- Palette-based generation (Gruvbox, Solarized, Dracula, Nord)

### Complexity: Medium (40-60 hours)

---

## Feature 3: Vim Mode

**Goal**: Full vim-style modal editing with motions like `ciw`, `dd`, `yy`

### What Exists
- 70+ pre-defined actions
- Keybinding lookup system
- Command palette
- Undo/redo system

### What Needs to Be Added

```rust
pub enum VimMode {
    Normal,
    Insert,
    Visual,
    Command,
    Replace,
}

// Motion system
pub enum Motion {
    Left, Right, Up, Down,       // h, j, k, l
    Word, BackWord, EndWord,     // w, b, e
    LineStart, LineEnd,          // 0, $
    FirstNonBlank,               // ^
    Paragraph { forward: bool }, // {, }
    Search { pattern: String },  // /, ?
}

// Operators
pub enum Operator {
    Delete,   // d
    Change,   // c
    Yank,     // y
    Paste,    // p
}

// Pending state for operator+motion
pub struct PendingVimState {
    pub operator: Option<Operator>,
    pub count: Option<u32>,
}
```

### Key Changes
- New module: `src/app/vim/`
- Modal state machine
- Motion parser
- Operator + motion composition
- Command mode (`:w`, `:q`, `:set`)
- Visual selection tracking

### Complexity: High (100-150 hours)

---

## Feature 4: Progress Visualizations

**Goal**: Animated celebrations on milestone completions

### What Exists
- Ratatui supports per-frame rendering
- BurnChart, Sparkline components
- Dashboard stats display

### What Needs to Be Added

```rust
pub struct CelebrationAnimation {
    pub achievement: Achievement,
    pub start_time: Instant,
    pub duration_ms: u64,
    pub animation_type: CelebrationType,
}

pub enum CelebrationType {
    TaskCompletion,       // "Task Complete!"
    StreakMilestone(u32), // "7-Day Streak!"
    ChallengeComplete,
    AchievementUnlock,
}

impl CelebrationAnimation {
    pub fn progress(&self) -> f64 {
        let elapsed = Instant::now() - self.start_time;
        (elapsed.as_millis() as f64 / self.duration_ms as f64).min(1.0)
    }
}
```

### Animation Types
- Confetti particles (Unicode: `*`, `+`, sparkles)
- Screen flash (brief background color change)
- Progress bar fill animation
- Toast notifications (slide in/out)

### No External Dependencies
- Uses only Ratatui's built-in styling
- Pure Rust timing and state management

### Complexity: Medium (50-80 hours)

---

## Feature 5: Custom Dashboard Widgets

**Goal**: Configurable dashboard with draggable widgets

### What Exists
- 7 fixed panels in dashboard
- Stats calculation in `dashboard/stats.rs`
- Theme-aware rendering

### What Needs to Be Added

```rust
pub enum DashboardWidget {
    CompletionRate,
    TimeTracking,
    Projects,
    StatusDistribution,
    EstimationAccuracy,
    FocusSessions,
    RecentActivity,
    Streaks,           // NEW
    DailyChallenge,    // NEW
    Achievements,      // NEW
}

pub struct DashboardLayout {
    pub grid: (u8, u8),  // e.g., 4x3
    pub widgets: Vec<WidgetPlacement>,
}

pub struct WidgetPlacement {
    pub widget: DashboardWidget,
    pub position: (u8, u8),
    pub size: (u8, u8),
}
```

### Configuration (TOML)
```toml
[dashboard]
grid = [4, 3]

[[dashboard.widgets]]
name = "completion_rate"
position = [0, 0]
size = [2, 1]

[[dashboard.widgets]]
name = "time_tracking"
position = [2, 0]
size = [2, 1]
```

### Interactive Editor
- Edit mode: press `E` in dashboard
- Move widgets with arrow keys
- Resize with Alt+arrows
- Add/remove from picker menu

### Complexity: Medium (60-100 hours)

---

## Feature 6: Cross-Project Dependency Visualization

**Goal**: Highlight critical paths across all projects

### What Exists
- Network view with ASCII tree
- `Task.dependencies: Vec<TaskId>`
- Dependency queries in `network/queries.rs`

### What Needs to Be Added

```rust
// Critical path analysis
pub struct CriticalPathAnalysis {
    pub critical_tasks: Vec<TaskId>,
    pub critical_edges: Vec<(TaskId, TaskId)>,
    pub bottleneck_tasks: Vec<TaskId>,  // Many dependents
    pub total_path_length: u32,
}

pub fn find_critical_path(tasks: &[Task]) -> CriticalPathAnalysis {
    // DAG longest path algorithm
}

pub fn calculate_task_impact(task_id: TaskId) -> u32 {
    // How many downstream tasks affected?
}
```

### Visualization
- Color coding: critical path in red, non-critical in muted
- Edge styling: `═══` for critical, `───` for normal
- Icons: `★` for critical, `⚠` for bottleneck
- Impact heatmap

### Complexity: Medium-High (70-120 hours)

---

## Implementation Priority

| Feature | Effort | Value | Risk | Priority |
|---------|--------|-------|------|----------|
| Progress Visualizations | 50-80h | Medium | Low | 1st (polish) |
| Theming Engine | 40-60h | Medium | Low | 2nd |
| Dashboard Widgets | 60-100h | Medium | Low | 3rd |
| Split Panes | 80-120h | High | Medium | 4th |
| Critical Path Viz | 70-120h | Medium | Medium | 5th |
| Vim Mode | 100-150h | Medium | High | 6th |

**Total Estimated Effort**: 400-630 hours (full implementation)

---

## Key Files to Modify/Create

```
src/ui/view/layout.rs           # Split panes
src/config/theme.rs             # Extended theming
src/app/vim/                    # NEW: Vim mode module
src/ui/animation.rs             # NEW: Animation framework
src/ui/components/dashboard/    # Widget system
src/ui/components/celebration.rs # NEW: Celebration overlays
src/app/analytics/critical_path.rs # NEW: Critical path algorithm
```

---

## See Also

- [FEATURE_IDEAS.md](FEATURE_IDEAS.md) - Feature overview
- [GAMIFICATION_FEATURES.md](GAMIFICATION_FEATURES.md) - Related: Achievement celebrations
