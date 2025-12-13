//! Sidebar navigation handlers.

use crate::app::{
    FocusPane, Model, NavigationMessage, ViewId, SIDEBAR_FIRST_PROJECT_INDEX,
    SIDEBAR_PROJECTS_HEADER_INDEX, SIDEBAR_SEPARATOR_INDEX, SIDEBAR_VIEWS,
};

/// Handle sidebar-specific navigation messages.
pub fn handle_sidebar_navigation(model: &mut Model, msg: NavigationMessage) {
    match msg {
        NavigationMessage::FocusSidebar => {
            if model.show_sidebar {
                model.focus_pane = FocusPane::Sidebar;
            }
        }
        NavigationMessage::FocusTaskList => {
            model.focus_pane = FocusPane::TaskList;
        }
        NavigationMessage::SelectSidebarItem => {
            handle_sidebar_selection(model);
        }
        NavigationMessage::SidebarSelectIndex(index) => {
            // Direct sidebar selection by index (for mouse click)
            let max_index = model.sidebar_item_count().saturating_sub(1);
            if index <= max_index && index != SIDEBAR_SEPARATOR_INDEX {
                model.sidebar_selected = index;
                model.focus_pane = FocusPane::Sidebar;
                handle_sidebar_selection(model);
            }
        }
        _ => {}
    }
}

/// Skip non-selectable items when navigating up in sidebar.
pub fn skip_sidebar_non_selectable_up(model: &mut Model) {
    let projects_end = SIDEBAR_FIRST_PROJECT_INDEX + model.projects.len().max(1);
    let contexts_separator = projects_end;
    let contexts_header = projects_end + 1;
    let contexts = model.all_contexts();
    let contexts_end = model.sidebar_contexts_start() + contexts.len().max(1);
    let filters_separator = contexts_end;
    let filters_header = contexts_end + 1;

    // Skip first separator (before Projects)
    if model.sidebar_selected == SIDEBAR_SEPARATOR_INDEX {
        model.sidebar_selected = SIDEBAR_SEPARATOR_INDEX - 1;
    }
    // Skip contexts separator
    else if model.sidebar_selected == contexts_separator {
        model.sidebar_selected = contexts_separator - 1;
    }
    // Skip contexts header
    else if model.sidebar_selected == contexts_header {
        model.sidebar_selected = contexts_header - 1;
        // Also skip the separator we just landed on
        if model.sidebar_selected == contexts_separator {
            model.sidebar_selected = contexts_separator - 1;
        }
    }
    // Skip filters separator
    else if model.sidebar_selected == filters_separator {
        model.sidebar_selected = filters_separator - 1;
    }
    // Skip filters header
    else if model.sidebar_selected == filters_header {
        model.sidebar_selected = filters_header - 1;
        // Also skip the separator we just landed on
        if model.sidebar_selected == filters_separator {
            model.sidebar_selected = filters_separator - 1;
        }
    }
}

/// Skip non-selectable items when navigating down in sidebar.
pub fn skip_sidebar_non_selectable_down(model: &mut Model, max_index: usize) {
    let projects_end = SIDEBAR_FIRST_PROJECT_INDEX + model.projects.len().max(1);
    let contexts_separator = projects_end;
    let contexts_header = projects_end + 1;
    let contexts = model.all_contexts();
    let contexts_end = model.sidebar_contexts_start() + contexts.len().max(1);
    let filters_separator = contexts_end;
    let filters_header = contexts_end + 1;

    // Skip first separator (before Projects)
    if model.sidebar_selected == SIDEBAR_SEPARATOR_INDEX && model.sidebar_selected < max_index {
        model.sidebar_selected = SIDEBAR_SEPARATOR_INDEX + 1;
    }
    // Skip contexts separator
    else if model.sidebar_selected == contexts_separator && model.sidebar_selected < max_index {
        model.sidebar_selected = contexts_separator + 1;
        // Also skip the header
        if model.sidebar_selected == contexts_header && model.sidebar_selected < max_index {
            model.sidebar_selected = contexts_header + 1;
        }
    }
    // Skip contexts header
    else if model.sidebar_selected == contexts_header && model.sidebar_selected < max_index {
        model.sidebar_selected = contexts_header + 1;
    }
    // Skip filters separator
    else if model.sidebar_selected == filters_separator && model.sidebar_selected < max_index {
        model.sidebar_selected = filters_separator + 1;
        // Also skip the header
        if model.sidebar_selected == filters_header && model.sidebar_selected < max_index {
            model.sidebar_selected = filters_header + 1;
        }
    }
    // Skip filters header
    else if model.sidebar_selected == filters_header && model.sidebar_selected < max_index {
        model.sidebar_selected = filters_header + 1;
    }
}

/// Handle sidebar item selection.
pub fn handle_sidebar_selection(model: &mut Model) {
    let selected = model.sidebar_selected;

    // Helper to activate a view
    let activate_view = |model: &mut Model, view: ViewId| {
        model.current_view = view;
        model.selected_project = None;
        model.focus_pane = FocusPane::TaskList;
        model.selected_index = 0;
        model.refresh_visible_tasks();
    };

    // Check if it's a view from SIDEBAR_VIEWS array
    if let Some(&view_id) = SIDEBAR_VIEWS.get(selected) {
        activate_view(model, view_id);
        return;
    }

    // Calculate indices for sections
    let projects_end = SIDEBAR_FIRST_PROJECT_INDEX + model.projects.len().max(1);
    let contexts_separator = projects_end;
    let contexts_header = projects_end + 1;
    let contexts_start = model.sidebar_contexts_start();
    let contexts = model.all_contexts();
    let contexts_end = contexts_start + contexts.len().max(1);
    let filters_separator = contexts_end;
    let filters_header = contexts_end + 1;
    let filters_start = model.sidebar_saved_filters_start();

    // Handle special items after the views
    match selected {
        n if n == SIDEBAR_SEPARATOR_INDEX => {} // First separator, do nothing
        n if n == SIDEBAR_PROJECTS_HEADER_INDEX => {
            // Projects header - go to Projects view showing all project tasks
            activate_view(model, ViewId::Projects);
        }
        n if n >= SIDEBAR_FIRST_PROJECT_INDEX && n < projects_end => {
            // Select a specific project
            let project_index = n - SIDEBAR_FIRST_PROJECT_INDEX;
            let project_ids: Vec<_> = model.projects.keys().copied().collect();
            if let Some(project_id) = project_ids.get(project_index) {
                model.current_view = ViewId::TaskList;
                model.selected_project = Some(*project_id);
                model.focus_pane = FocusPane::TaskList;
                model.selected_index = 0;
                model.refresh_visible_tasks();
            }
        }
        n if n == contexts_separator => {} // Contexts separator, do nothing
        n if n == contexts_header => {}    // Contexts header, do nothing
        n if n >= contexts_start && n < contexts_end => {
            // Select a specific context
            let context_index = n - contexts_start;
            if let Some(context) = contexts.get(context_index) {
                // Apply filter for this context
                model.filtering.filter.tags = Some(vec![context.clone()]);
                model.filtering.filter.tags_mode = crate::domain::TagFilterMode::Any;
                model.active_saved_filter = None; // Clear any active saved filter
                model.current_view = ViewId::TaskList;
                model.selected_project = None;
                model.focus_pane = FocusPane::TaskList;
                model.selected_index = 0;
                model.refresh_visible_tasks();
                model.alerts.status_message = Some(format!("Context: {context}"));
            }
        }
        n if n == filters_separator => {} // Filters separator, do nothing
        n if n == filters_header => {}    // Filters header, do nothing
        n if n >= filters_start => {
            // Select a saved filter
            let filter_index = n - filters_start;
            let mut filters: Vec<_> = model.saved_filters.values().collect();
            filters.sort_by(|a, b| a.name.cmp(&b.name));
            if let Some(filter) = filters.get(filter_index) {
                // Clone the filter data before modifying model
                let filter_clone = (*filter).clone();
                let filter_name = filter_clone.name.clone();
                // Apply the filter
                model.filtering.filter = filter_clone.filter;
                model.filtering.sort = filter_clone.sort;
                model.active_saved_filter = Some(filter_clone.id);
                model.focus_pane = FocusPane::TaskList;
                model.selected_index = 0;
                model.refresh_visible_tasks();
                model.alerts.status_message = Some(format!("Applied filter: {filter_name}"));
            }
        }
        _ => {}
    }
}
