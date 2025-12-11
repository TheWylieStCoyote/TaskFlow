//! Layout cache for mouse hit-testing.
//!
//! Stores cached UI layout rectangles for efficient mouse click detection.
//! Uses interior mutability so layout can be updated during rendering
//! even when the Model is borrowed immutably.

use std::cell::RefCell;
use std::time::Instant;

use ratatui::layout::Rect;

/// Internal layout data for mouse hit-testing.
#[derive(Debug, Clone, Default)]
struct LayoutData {
    /// Sidebar area (if visible)
    sidebar_area: Option<Rect>,
    /// Main content area (task list, calendar, etc.)
    main_area: Option<Rect>,
    /// Task list area within main area
    task_list_area: Option<Rect>,
    /// Calendar grid area (calendar view only)
    calendar_area: Option<Rect>,
    /// Individual kanban column areas
    kanban_columns: [Option<Rect>; 4],
    /// Eisenhower quadrant areas
    eisenhower_quadrants: [Option<Rect>; 4],
    /// Weekly planner day column areas
    weekly_planner_days: [Option<Rect>; 7],
    /// Reports tabs area (for clicking on tabs)
    reports_tabs_area: Option<Rect>,
    /// Individual report tab areas for precise click detection
    reports_tab_rects: [Option<Rect>; 7],
    /// Header height offset for task list (border + header row)
    task_list_header_offset: u16,
    /// Scroll offset for task list (how many rows scrolled)
    scroll_offset: usize,
    /// Last click position and time for double-click detection
    last_click: Option<(u16, u16, Instant)>,
}

/// Cached layout rectangles for mouse hit-testing.
///
/// Uses interior mutability so layout can be updated during rendering
/// even when the Model is borrowed immutably.
#[derive(Debug, Default, Clone)]
pub struct LayoutCache {
    data: RefCell<LayoutData>,
}

impl LayoutCache {
    /// Clear all cached layout areas.
    pub fn clear(&self) {
        let mut data = self.data.borrow_mut();
        data.sidebar_area = None;
        data.main_area = None;
        data.task_list_area = None;
        data.calendar_area = None;
        data.kanban_columns = [None; 4];
        data.eisenhower_quadrants = [None; 4];
        data.weekly_planner_days = [None; 7];
        data.reports_tabs_area = None;
        data.reports_tab_rects = [None; 7];
    }

    /// Check if a point is within a rectangle.
    #[must_use]
    pub fn is_in_rect(x: u16, y: u16, rect: Rect) -> bool {
        x >= rect.x
            && x < rect.x.saturating_add(rect.width)
            && y >= rect.y
            && y < rect.y.saturating_add(rect.height)
    }

    /// Record a click for double-click detection.
    pub fn record_click(&self, x: u16, y: u16) {
        self.data.borrow_mut().last_click = Some((x, y, Instant::now()));
    }

    /// Check if the current click is a double-click.
    #[must_use]
    pub fn is_double_click(&self, x: u16, y: u16) -> bool {
        let data = self.data.borrow();
        if let Some((last_x, last_y, last_time)) = data.last_click {
            let same_position =
                (x as i16 - last_x as i16).abs() <= 1 && (y as i16 - last_y as i16).abs() <= 1;
            let within_time = last_time.elapsed().as_millis() < 500;
            same_position && within_time
        } else {
            false
        }
    }

    // ========================================================================
    // Layout setters (for use during rendering)
    // ========================================================================

    /// Set the sidebar area.
    pub fn set_sidebar_area(&self, area: Rect) {
        self.data.borrow_mut().sidebar_area = Some(area);
    }

    /// Set the main content area.
    pub fn set_main_area(&self, area: Rect) {
        self.data.borrow_mut().main_area = Some(area);
    }

    /// Set the task list area.
    pub fn set_task_list_area(&self, area: Rect, header_offset: u16, scroll_offset: usize) {
        let mut data = self.data.borrow_mut();
        data.task_list_area = Some(area);
        data.task_list_header_offset = header_offset;
        data.scroll_offset = scroll_offset;
    }

    /// Set the calendar grid area.
    pub fn set_calendar_area(&self, area: Rect) {
        self.data.borrow_mut().calendar_area = Some(area);
    }

    /// Set a kanban column area.
    pub fn set_kanban_column(&self, index: usize, area: Rect) {
        if index < 4 {
            self.data.borrow_mut().kanban_columns[index] = Some(area);
        }
    }

    /// Set an eisenhower quadrant area.
    pub fn set_eisenhower_quadrant(&self, index: usize, area: Rect) {
        if index < 4 {
            self.data.borrow_mut().eisenhower_quadrants[index] = Some(area);
        }
    }

    /// Set a weekly planner day column area.
    pub fn set_weekly_planner_day(&self, index: usize, area: Rect) {
        if index < 7 {
            self.data.borrow_mut().weekly_planner_days[index] = Some(area);
        }
    }

    /// Set the reports tabs area.
    pub fn set_reports_tabs_area(&self, area: Rect) {
        self.data.borrow_mut().reports_tabs_area = Some(area);
    }

    /// Set an individual report tab area.
    pub fn set_reports_tab_rect(&self, index: usize, area: Rect) {
        if index < 7 {
            self.data.borrow_mut().reports_tab_rects[index] = Some(area);
        }
    }

    // ========================================================================
    // Layout getters (for use in mouse handling)
    // ========================================================================

    /// Get the sidebar area.
    #[must_use]
    pub fn sidebar_area(&self) -> Option<Rect> {
        self.data.borrow().sidebar_area
    }

    /// Get the main content area.
    #[must_use]
    pub fn main_area(&self) -> Option<Rect> {
        self.data.borrow().main_area
    }

    /// Get the task list area.
    #[must_use]
    pub fn task_list_area(&self) -> Option<Rect> {
        self.data.borrow().task_list_area
    }

    /// Get the task list header offset.
    #[must_use]
    pub fn task_list_header_offset(&self) -> u16 {
        self.data.borrow().task_list_header_offset
    }

    /// Get the task list scroll offset.
    #[must_use]
    pub fn scroll_offset(&self) -> usize {
        self.data.borrow().scroll_offset
    }

    /// Get the calendar grid area.
    #[must_use]
    pub fn calendar_area(&self) -> Option<Rect> {
        self.data.borrow().calendar_area
    }

    /// Get a kanban column area.
    #[must_use]
    pub fn kanban_column(&self, index: usize) -> Option<Rect> {
        if index < 4 {
            self.data.borrow().kanban_columns[index]
        } else {
            None
        }
    }

    /// Get all kanban column areas.
    #[must_use]
    pub fn kanban_columns(&self) -> [Option<Rect>; 4] {
        self.data.borrow().kanban_columns
    }

    /// Get an eisenhower quadrant area.
    #[must_use]
    pub fn eisenhower_quadrant(&self, index: usize) -> Option<Rect> {
        if index < 4 {
            self.data.borrow().eisenhower_quadrants[index]
        } else {
            None
        }
    }

    /// Get all eisenhower quadrant areas.
    #[must_use]
    pub fn eisenhower_quadrants(&self) -> [Option<Rect>; 4] {
        self.data.borrow().eisenhower_quadrants
    }

    /// Get a weekly planner day column area.
    #[must_use]
    pub fn weekly_planner_day(&self, index: usize) -> Option<Rect> {
        if index < 7 {
            self.data.borrow().weekly_planner_days[index]
        } else {
            None
        }
    }

    /// Get all weekly planner day column areas.
    #[must_use]
    pub fn weekly_planner_days(&self) -> [Option<Rect>; 7] {
        self.data.borrow().weekly_planner_days
    }

    /// Get the reports tabs area.
    #[must_use]
    pub fn reports_tabs_area(&self) -> Option<Rect> {
        self.data.borrow().reports_tabs_area
    }

    /// Get all report tab areas.
    #[must_use]
    pub fn reports_tab_rects(&self) -> [Option<Rect>; 7] {
        self.data.borrow().reports_tab_rects
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_cache_default() {
        let cache = LayoutCache::default();
        assert!(cache.sidebar_area().is_none());
        assert!(cache.main_area().is_none());
        assert!(cache.task_list_area().is_none());
        assert!(cache.calendar_area().is_none());
        assert_eq!(cache.task_list_header_offset(), 0);
        assert_eq!(cache.scroll_offset(), 0);
    }

    #[test]
    fn test_layout_cache_setters_and_getters() {
        let cache = LayoutCache::default();

        let sidebar = Rect::new(0, 0, 20, 50);
        let main = Rect::new(20, 0, 60, 50);
        let task_list = Rect::new(20, 0, 60, 40);

        cache.set_sidebar_area(sidebar);
        cache.set_main_area(main);
        cache.set_task_list_area(task_list, 2, 5);

        assert_eq!(cache.sidebar_area(), Some(sidebar));
        assert_eq!(cache.main_area(), Some(main));
        assert_eq!(cache.task_list_area(), Some(task_list));
        assert_eq!(cache.task_list_header_offset(), 2);
        assert_eq!(cache.scroll_offset(), 5);
    }

    #[test]
    fn test_layout_cache_kanban_columns() {
        let cache = LayoutCache::default();

        let col0 = Rect::new(0, 0, 25, 30);
        let col1 = Rect::new(25, 0, 25, 30);
        let col2 = Rect::new(50, 0, 25, 30);

        cache.set_kanban_column(0, col0);
        cache.set_kanban_column(1, col1);
        cache.set_kanban_column(2, col2);
        // Setting out of bounds should be ignored
        cache.set_kanban_column(10, Rect::default());

        assert_eq!(cache.kanban_column(0), Some(col0));
        assert_eq!(cache.kanban_column(1), Some(col1));
        assert_eq!(cache.kanban_column(2), Some(col2));
        assert!(cache.kanban_column(3).is_none());
        assert!(cache.kanban_column(10).is_none());

        let all_columns = cache.kanban_columns();
        assert_eq!(all_columns[0], Some(col0));
        assert_eq!(all_columns[1], Some(col1));
        assert_eq!(all_columns[2], Some(col2));
    }

    #[test]
    fn test_layout_cache_eisenhower_quadrants() {
        let cache = LayoutCache::default();

        let quad0 = Rect::new(0, 0, 40, 20);
        let quad1 = Rect::new(40, 0, 40, 20);
        let quad2 = Rect::new(0, 20, 40, 20);
        let quad3 = Rect::new(40, 20, 40, 20);

        cache.set_eisenhower_quadrant(0, quad0);
        cache.set_eisenhower_quadrant(1, quad1);
        cache.set_eisenhower_quadrant(2, quad2);
        cache.set_eisenhower_quadrant(3, quad3);
        cache.set_eisenhower_quadrant(5, Rect::default()); // Out of bounds

        assert_eq!(cache.eisenhower_quadrant(0), Some(quad0));
        assert_eq!(cache.eisenhower_quadrant(1), Some(quad1));
        assert_eq!(cache.eisenhower_quadrant(2), Some(quad2));
        assert_eq!(cache.eisenhower_quadrant(3), Some(quad3));
        assert!(cache.eisenhower_quadrant(5).is_none());

        let all_quads = cache.eisenhower_quadrants();
        assert_eq!(all_quads[0], Some(quad0));
    }

    #[test]
    fn test_layout_cache_weekly_planner_days() {
        let cache = LayoutCache::default();

        for i in 0..7 {
            cache.set_weekly_planner_day(i, Rect::new(i as u16 * 10, 0, 10, 30));
        }
        cache.set_weekly_planner_day(10, Rect::default()); // Out of bounds

        for i in 0..7 {
            assert_eq!(
                cache.weekly_planner_day(i),
                Some(Rect::new(i as u16 * 10, 0, 10, 30))
            );
        }
        assert!(cache.weekly_planner_day(10).is_none());

        let all_days = cache.weekly_planner_days();
        assert_eq!(all_days[0], Some(Rect::new(0, 0, 10, 30)));
    }

    #[test]
    fn test_layout_cache_reports_tabs() {
        let cache = LayoutCache::default();

        let tabs_area = Rect::new(0, 0, 80, 3);
        cache.set_reports_tabs_area(tabs_area);

        for i in 0..7 {
            cache.set_reports_tab_rect(i, Rect::new(i as u16 * 10, 0, 10, 3));
        }
        cache.set_reports_tab_rect(10, Rect::default()); // Out of bounds

        assert_eq!(cache.reports_tabs_area(), Some(tabs_area));

        let all_tabs = cache.reports_tab_rects();
        assert_eq!(all_tabs[0], Some(Rect::new(0, 0, 10, 3)));
    }

    #[test]
    fn test_layout_cache_clear() {
        let cache = LayoutCache::default();

        cache.set_sidebar_area(Rect::new(0, 0, 20, 50));
        cache.set_main_area(Rect::new(20, 0, 60, 50));
        cache.set_kanban_column(0, Rect::new(0, 0, 20, 30));
        cache.set_eisenhower_quadrant(0, Rect::new(0, 0, 40, 20));
        cache.set_weekly_planner_day(0, Rect::new(0, 0, 10, 30));

        assert!(cache.sidebar_area().is_some());

        cache.clear();

        assert!(cache.sidebar_area().is_none());
        assert!(cache.main_area().is_none());
        assert!(cache.kanban_column(0).is_none());
        assert!(cache.eisenhower_quadrant(0).is_none());
        assert!(cache.weekly_planner_day(0).is_none());
    }

    #[test]
    fn test_layout_cache_clone() {
        let cache1 = LayoutCache::default();
        cache1.set_sidebar_area(Rect::new(0, 0, 20, 50));

        let cache2 = cache1.clone();

        assert_eq!(cache2.sidebar_area(), Some(Rect::new(0, 0, 20, 50)));
    }

    #[test]
    fn test_is_in_rect() {
        let rect = Rect::new(10, 10, 20, 20);

        // Inside
        assert!(LayoutCache::is_in_rect(15, 15, rect));
        assert!(LayoutCache::is_in_rect(10, 10, rect)); // Edge top-left
        assert!(LayoutCache::is_in_rect(29, 29, rect)); // Edge bottom-right (exclusive)

        // Outside
        assert!(!LayoutCache::is_in_rect(9, 15, rect)); // Left
        assert!(!LayoutCache::is_in_rect(30, 15, rect)); // Right
        assert!(!LayoutCache::is_in_rect(15, 9, rect)); // Above
        assert!(!LayoutCache::is_in_rect(15, 30, rect)); // Below
    }

    #[test]
    fn test_double_click_detection() {
        let cache = LayoutCache::default();

        // Before any click, is_double_click should return false
        assert!(!cache.is_double_click(10, 10));

        // First click - record it
        cache.record_click(10, 10);

        // After recording, a subsequent check at same position is a double-click
        assert!(cache.is_double_click(10, 10));
    }

    #[test]
    fn test_double_click_same_position() {
        let cache = LayoutCache::default();

        // Record first click
        cache.record_click(10, 10);

        // Check for double click at same position
        // Note: since very little time has passed, this should be true
        assert!(cache.is_double_click(10, 10));
    }

    #[test]
    fn test_double_click_different_position() {
        let cache = LayoutCache::default();

        cache.record_click(10, 10);

        // Far from original click position
        assert!(!cache.is_double_click(100, 100));
    }
}
