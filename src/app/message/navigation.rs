//! Navigation messages for movement and view switching.

/// Navigation messages for movement and view switching.
///
/// These messages handle cursor movement within lists, switching
/// between views, and calendar navigation.
///
/// # Examples
///
/// ```
/// use taskflow::app::{Model, Message, NavigationMessage, ViewId, update};
///
/// let mut model = Model::new().with_sample_data();
///
/// // Move selection down
/// update(&mut model, NavigationMessage::Down.into());
///
/// // Switch to a different view
/// update(&mut model, NavigationMessage::GoToView(ViewId::Today).into());
/// ```
#[derive(Debug, Clone)]
pub enum NavigationMessage {
    /// Move selection up in the current list
    Up,
    /// Move selection down in the current list
    Down,
    /// Jump to the first item
    First,
    /// Jump to the last item
    Last,
    /// Move up by a page (10 items)
    PageUp,
    /// Move down by a page (10 items)
    PageDown,
    /// Select a specific item by index
    Select(usize),
    /// Switch to a different view
    GoToView(ViewId),
    /// Move focus to the sidebar
    FocusSidebar,
    /// Move focus to the task list
    FocusTaskList,
    /// Activate the selected sidebar item
    SelectSidebarItem,
    /// Navigate to previous month in calendar
    CalendarPrevMonth,
    /// Navigate to next month in calendar
    CalendarNextMonth,
    /// Select a specific day in calendar
    CalendarSelectDay(u32),
    /// Focus the task list panel in calendar view
    CalendarFocusTaskList,
    /// Focus the calendar grid in calendar view
    CalendarFocusGrid,
    /// Navigate to next panel in reports view
    ReportsNextPanel,
    /// Navigate to previous panel in reports view
    ReportsPrevPanel,
    /// Scroll timeline viewport left (earlier dates)
    TimelineScrollLeft,
    /// Scroll timeline viewport right (later dates)
    TimelineScrollRight,
    /// Zoom in on timeline (Week → Day)
    TimelineZoomIn,
    /// Zoom out on timeline (Day → Week)
    TimelineZoomOut,
    /// Jump to today in timeline
    TimelineGoToday,
    /// Navigate up in timeline task list
    TimelineUp,
    /// Navigate down in timeline task list
    TimelineDown,
    /// Navigate left in Kanban view (previous column)
    KanbanLeft,
    /// Navigate right in Kanban view (next column)
    KanbanRight,
    /// Navigate up in Kanban view (previous task in column)
    KanbanUp,
    /// Navigate down in Kanban view (next task in column)
    KanbanDown,
    /// Navigate up in Eisenhower view (to upper quadrant)
    EisenhowerUp,
    /// Navigate down in Eisenhower view (to lower quadrant)
    EisenhowerDown,
    /// Navigate left in Eisenhower view (to left quadrant)
    EisenhowerLeft,
    /// Navigate right in Eisenhower view (to right quadrant)
    EisenhowerRight,
    /// Navigate left in WeeklyPlanner view (previous day)
    WeeklyPlannerLeft,
    /// Navigate right in WeeklyPlanner view (next day)
    WeeklyPlannerRight,
    /// Navigate up in WeeklyPlanner view (previous task in day)
    WeeklyPlannerUp,
    /// Navigate down in WeeklyPlanner view (next task in day)
    WeeklyPlannerDown,
    /// Select a specific sidebar item by index (for mouse click)
    SidebarSelectIndex(usize),
    /// Select a specific Kanban column by index (for mouse click)
    KanbanSelectColumn(usize),
    /// Select a specific Eisenhower quadrant by index (for mouse click)
    EisenhowerSelectQuadrant(usize),
    /// Select a specific WeeklyPlanner day by index (for mouse click)
    WeeklyPlannerSelectDay(usize),
    /// Select a specific Reports panel by index (for mouse click)
    ReportsSelectPanel(usize),
    /// Navigate up in Network view (previous task)
    NetworkUp,
    /// Navigate down in Network view (next task)
    NetworkDown,
}

/// View identifiers for different application screens.
///
/// Each view shows tasks filtered and presented differently.
///
/// # Examples
///
/// ```
/// use taskflow::app::ViewId;
///
/// let view = ViewId::default();
/// assert_eq!(view, ViewId::TaskList);
///
/// // Compare views
/// let today = ViewId::Today;
/// let upcoming = ViewId::Upcoming;
/// assert_ne!(today, upcoming);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ViewId {
    /// All tasks view (default)
    #[default]
    TaskList,
    /// Tasks due today
    Today,
    /// Tasks with future due dates
    Upcoming,
    /// Tasks past their due date
    Overdue,
    /// Tasks with scheduled dates, sorted by scheduled date
    Scheduled,
    /// Monthly calendar view
    Calendar,
    /// Statistics and overview dashboard
    Dashboard,
    /// Tasks grouped by project
    Projects,
    /// Tasks with incomplete dependencies (blocked)
    Blocked,
    /// Tasks without any tags
    Untagged,
    /// Tasks not assigned to any project
    NoProject,
    /// Tasks modified in the last 7 days
    RecentlyModified,
    /// Analytics and reports view
    Reports,
    /// Kanban board view with columns for each status
    Kanban,
    /// Eisenhower matrix (urgent/important quadrants)
    Eisenhower,
    /// Weekly planner view with day columns
    WeeklyPlanner,
    /// Timeline/Gantt view showing tasks as bars on time axis
    Timeline,
    /// Snoozed tasks (hidden until snooze date)
    Snoozed,
    /// Habit tracking view
    Habits,
    /// Goal/OKR tracking view
    Goals,
    /// Heatmap view (GitHub-style contribution graph)
    Heatmap,
    /// Forecast view (workload projection into future)
    Forecast,
    /// Network graph view (dependency visualization)
    Network,
    /// Burndown chart view (progress toward completion)
    Burndown,
    /// Duplicate task detection view
    Duplicates,
}
