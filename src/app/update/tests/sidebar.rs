//! Sidebar navigation tests.

use crate::app::{
    update::update, FocusPane, Message, Model, NavigationMessage, ViewId,
    SIDEBAR_FIRST_PROJECT_INDEX, SIDEBAR_PROJECTS_HEADER_INDEX, SIDEBAR_SEPARATOR_INDEX,
};
use crate::domain::{Project, Task};

#[test]
fn test_focus_sidebar() {
    let mut model = Model::new();
    assert_eq!(model.focus_pane, FocusPane::TaskList);

    update(
        &mut model,
        Message::Navigation(NavigationMessage::FocusSidebar),
    );

    assert_eq!(model.focus_pane, FocusPane::Sidebar);
}

#[test]
fn test_focus_task_list() {
    let mut model = Model::new();
    model.focus_pane = FocusPane::Sidebar;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::FocusTaskList),
    );

    assert_eq!(model.focus_pane, FocusPane::TaskList);
}

#[test]
fn test_sidebar_navigation_up_down() {
    let mut model = Model::new().with_sample_data();
    model.focus_pane = FocusPane::Sidebar;
    model.sidebar_selected = 0;

    // Move down
    update(&mut model, Message::Navigation(NavigationMessage::Down));
    assert_eq!(model.sidebar_selected, 1);

    // Move down again
    update(&mut model, Message::Navigation(NavigationMessage::Down));
    assert_eq!(model.sidebar_selected, 2);

    // Move up
    update(&mut model, Message::Navigation(NavigationMessage::Up));
    assert_eq!(model.sidebar_selected, 1);
}

#[test]
fn test_sidebar_navigation_skips_separator() {
    let mut model = Model::new().with_sample_data();
    model.focus_pane = FocusPane::Sidebar;
    // Position at last view item (just before separator)
    model.sidebar_selected = SIDEBAR_SEPARATOR_INDEX - 1;

    // Move down should skip separator and go to Projects header
    update(&mut model, Message::Navigation(NavigationMessage::Down));
    assert_eq!(model.sidebar_selected, SIDEBAR_PROJECTS_HEADER_INDEX);

    // Move up should skip separator and go back to last view item
    update(&mut model, Message::Navigation(NavigationMessage::Up));
    assert_eq!(model.sidebar_selected, SIDEBAR_SEPARATOR_INDEX - 1);
}

#[test]
fn test_sidebar_select_view() {
    let mut model = Model::new().with_sample_data();
    model.focus_pane = FocusPane::Sidebar;
    model.sidebar_selected = 1; // Today view

    update(
        &mut model,
        Message::Navigation(NavigationMessage::SelectSidebarItem),
    );

    assert_eq!(model.current_view, ViewId::Today);
    assert!(model.selected_project.is_none());
}

#[test]
fn test_sidebar_select_overdue_view() {
    let mut model = Model::new().with_sample_data();
    model.focus_pane = FocusPane::Sidebar;
    model.sidebar_selected = 3; // Overdue view

    update(
        &mut model,
        Message::Navigation(NavigationMessage::SelectSidebarItem),
    );

    assert_eq!(model.current_view, ViewId::Overdue);
    assert!(model.selected_project.is_none());
    assert_eq!(model.focus_pane, FocusPane::TaskList);
}

#[test]
fn test_sidebar_select_project() {
    let mut model = Model::new();
    // Add a project
    let project = Project::new("Test Project");
    let project_id = project.id;
    model.projects.insert(project.id, project);

    // Add a task with this project
    let mut task = Task::new("Task in project");
    task.project_id = Some(project_id);
    model.tasks.insert(task.id, task);

    // Add a task without project
    let task2 = Task::new("Task without project");
    model.tasks.insert(task2.id, task2);

    model.refresh_visible_tasks();
    assert_eq!(model.visible_tasks.len(), 2);

    model.focus_pane = FocusPane::Sidebar;
    model.sidebar_selected = SIDEBAR_FIRST_PROJECT_INDEX; // First project

    update(
        &mut model,
        Message::Navigation(NavigationMessage::SelectSidebarItem),
    );

    // Project should be selected
    assert_eq!(model.selected_project, Some(project_id));
    // Only project tasks should be visible
    assert_eq!(model.visible_tasks.len(), 1);
}

#[test]
fn test_sidebar_select_all_tasks_clears_project_filter() {
    let mut model = Model::new();
    let project = Project::new("Test Project");
    let project_id = project.id;
    model.projects.insert(project.id, project);
    model.selected_project = Some(project_id);

    model.focus_pane = FocusPane::Sidebar;
    model.sidebar_selected = 0; // All Tasks

    update(
        &mut model,
        Message::Navigation(NavigationMessage::SelectSidebarItem),
    );

    assert!(model.selected_project.is_none());
    assert_eq!(model.current_view, ViewId::TaskList);
}

// ============================================================================
// Sidebar sections beyond projects: contexts, saved filters, direct selection
// ============================================================================

mod sections {
    use crate::app::{
        update::update, FocusPane, Message, Model, NavigationMessage, ViewId,
        SIDEBAR_FIRST_PROJECT_INDEX, SIDEBAR_PROJECTS_HEADER_INDEX, SIDEBAR_SEPARATOR_INDEX,
    };
    use crate::domain::{Project, SavedFilter, SortField, SortOrder, Task};

    fn nav(model: &mut Model, msg: NavigationMessage) {
        update(model, Message::Navigation(msg));
    }

    /// Model with one project, two contexts (@home, @work) and two saved filters.
    fn sectioned_model() -> Model {
        let mut model = Model::new();
        let project = Project::new("Proj");
        model.projects.insert(project.id, project);

        let mut home = Task::new("Home chore");
        home.tags = vec!["@home".to_string()];
        let mut work = Task::new("Work item");
        work.tags = vec!["@work".to_string(), "deep".to_string()];
        let plain = Task::new("Plain");
        model.tasks.insert(home.id, home);
        model.tasks.insert(work.id, work);
        model.tasks.insert(plain.id, plain);

        let mut sort = model.filtering.sort.clone();
        sort.field = SortField::Title;
        sort.order = SortOrder::Ascending;
        let mut filter = model.filtering.filter.clone();
        filter.search_text = Some("Work".to_string());
        let alpha = SavedFilter::new("Alpha filter", filter, sort);
        let zeta = SavedFilter::new(
            "Zeta filter",
            model.filtering.filter.clone(),
            model.filtering.sort.clone(),
        );
        model.saved_filters.insert(alpha.id.clone(), alpha);
        model.saved_filters.insert(zeta.id.clone(), zeta);

        model.refresh_visible_tasks();
        model.focus_pane = FocusPane::Sidebar;
        model
    }

    #[test]
    fn test_projects_header_activates_projects_view() {
        let mut model = sectioned_model();
        model.sidebar_selected = SIDEBAR_PROJECTS_HEADER_INDEX;
        nav(&mut model, NavigationMessage::SelectSidebarItem);
        assert_eq!(model.current_view, ViewId::Projects);
        assert_eq!(model.focus_pane, FocusPane::TaskList);
        assert!(model.selected_project.is_none());
    }

    #[test]
    fn test_selecting_context_applies_tag_filter() {
        let mut model = sectioned_model();
        let contexts = model.all_contexts();
        assert_eq!(contexts, vec!["@home".to_string(), "@work".to_string()]);
        model.sidebar_selected = model.sidebar_contexts_start() + 1;

        nav(&mut model, NavigationMessage::SelectSidebarItem);

        assert_eq!(model.filtering.filter.tags, Some(vec!["@work".to_string()]));
        assert_eq!(model.current_view, ViewId::TaskList);
        assert_eq!(model.focus_pane, FocusPane::TaskList);
        assert_eq!(model.visible_tasks.len(), 1);
        assert_eq!(
            model.alerts.status_message.as_deref(),
            Some("Context: @work")
        );
    }

    #[test]
    fn test_selecting_context_clears_active_saved_filter() {
        let mut model = sectioned_model();
        let some_filter_id = model.saved_filters.keys().next().cloned();
        model.active_saved_filter = some_filter_id;
        model.sidebar_selected = model.sidebar_contexts_start();

        nav(&mut model, NavigationMessage::SelectSidebarItem);

        assert!(model.active_saved_filter.is_none());
        assert_eq!(model.filtering.filter.tags, Some(vec!["@home".to_string()]));
    }

    #[test]
    fn test_selecting_saved_filter_applies_it_in_name_order() {
        let mut model = sectioned_model();
        model.sidebar_selected = model.sidebar_saved_filters_start();

        nav(&mut model, NavigationMessage::SelectSidebarItem);

        assert_eq!(
            model.alerts.status_message.as_deref(),
            Some("Applied filter: Alpha filter")
        );
        assert_eq!(model.filtering.filter.search_text.as_deref(), Some("Work"));
        assert!(matches!(model.filtering.sort.field, SortField::Title));
        let active = model.active_saved_filter.as_ref().expect("active filter");
        assert_eq!(model.saved_filters[active].name, "Alpha filter");
        assert_eq!(model.visible_tasks.len(), 1);
        assert_eq!(model.focus_pane, FocusPane::TaskList);
    }

    #[test]
    fn test_selecting_second_saved_filter() {
        let mut model = sectioned_model();
        model.sidebar_selected = model.sidebar_saved_filters_start() + 1;
        nav(&mut model, NavigationMessage::SelectSidebarItem);
        assert_eq!(
            model.alerts.status_message.as_deref(),
            Some("Applied filter: Zeta filter")
        );
    }

    #[test]
    fn test_selecting_past_last_saved_filter_is_noop() {
        let mut model = sectioned_model();
        model.sidebar_selected = model.sidebar_saved_filters_start() + 5;
        nav(&mut model, NavigationMessage::SelectSidebarItem);
        assert!(model.active_saved_filter.is_none());
        assert_eq!(model.focus_pane, FocusPane::Sidebar);
    }

    #[test]
    fn test_separators_and_headers_are_not_selectable() {
        let mut model = sectioned_model();
        let projects_end = SIDEBAR_FIRST_PROJECT_INDEX + model.projects.len();
        let contexts_end = model.sidebar_contexts_start() + model.all_contexts().len();
        for index in [
            SIDEBAR_SEPARATOR_INDEX,
            projects_end,     // contexts separator
            projects_end + 1, // contexts header
            contexts_end,     // filters separator
            contexts_end + 1, // filters header
        ] {
            model.sidebar_selected = index;
            nav(&mut model, NavigationMessage::SelectSidebarItem);
            assert_eq!(model.focus_pane, FocusPane::Sidebar, "index {index}");
            assert!(model.alerts.status_message.is_none(), "index {index}");
        }
    }

    #[test]
    fn test_sidebar_select_index_jumps_and_activates() {
        let mut model = sectioned_model();
        model.focus_pane = FocusPane::TaskList;
        let context_index = model.sidebar_contexts_start();

        nav(
            &mut model,
            NavigationMessage::SidebarSelectIndex(context_index),
        );

        assert_eq!(model.sidebar_selected, context_index);
        assert_eq!(model.filtering.filter.tags, Some(vec!["@home".to_string()]));
        assert_eq!(model.focus_pane, FocusPane::TaskList);
    }

    #[test]
    fn test_sidebar_select_index_ignores_separator_and_out_of_range() {
        let mut model = sectioned_model();
        model.sidebar_selected = 0;
        model.focus_pane = FocusPane::TaskList;

        nav(
            &mut model,
            NavigationMessage::SidebarSelectIndex(SIDEBAR_SEPARATOR_INDEX),
        );
        assert_eq!(model.sidebar_selected, 0);
        assert_eq!(model.focus_pane, FocusPane::TaskList);

        let too_far = model.sidebar_item_count() + 3;
        nav(&mut model, NavigationMessage::SidebarSelectIndex(too_far));
        assert_eq!(model.sidebar_selected, 0);
    }

    #[test]
    fn test_sidebar_select_index_project_sets_selected_project() {
        let mut model = sectioned_model();
        let project_id = *model.projects.keys().next().unwrap();
        nav(
            &mut model,
            NavigationMessage::SidebarSelectIndex(SIDEBAR_FIRST_PROJECT_INDEX),
        );
        assert_eq!(model.selected_project, Some(project_id));
        assert_eq!(model.current_view, ViewId::TaskList);
    }

    #[test]
    fn test_navigation_down_skips_contexts_separator_and_header() {
        let mut model = sectioned_model();
        let last_project = SIDEBAR_FIRST_PROJECT_INDEX + model.projects.len() - 1;
        model.sidebar_selected = last_project;

        nav(&mut model, NavigationMessage::Down);

        assert_eq!(model.sidebar_selected, model.sidebar_contexts_start());
    }

    #[test]
    fn test_navigation_up_skips_contexts_header_and_separator() {
        let mut model = sectioned_model();
        model.sidebar_selected = model.sidebar_contexts_start();

        nav(&mut model, NavigationMessage::Up);

        let last_project = SIDEBAR_FIRST_PROJECT_INDEX + model.projects.len() - 1;
        assert_eq!(model.sidebar_selected, last_project);
    }

    #[test]
    fn test_navigation_down_skips_filters_separator_and_header() {
        let mut model = sectioned_model();
        let last_context = model.sidebar_contexts_start() + model.all_contexts().len() - 1;
        model.sidebar_selected = last_context;

        nav(&mut model, NavigationMessage::Down);

        assert_eq!(model.sidebar_selected, model.sidebar_saved_filters_start());
    }

    #[test]
    fn test_navigation_up_skips_filters_header_and_separator() {
        let mut model = sectioned_model();
        model.sidebar_selected = model.sidebar_saved_filters_start();

        nav(&mut model, NavigationMessage::Up);

        let last_context = model.sidebar_contexts_start() + model.all_contexts().len() - 1;
        assert_eq!(model.sidebar_selected, last_context);
    }

    #[test]
    fn test_navigation_down_stops_at_last_item() {
        let mut model = sectioned_model();
        let last = model.sidebar_item_count() - 1;
        model.sidebar_selected = last;
        nav(&mut model, NavigationMessage::Down);
        assert_eq!(model.sidebar_selected, last);
    }

    #[test]
    fn test_navigation_walks_every_selectable_item_down_then_up() {
        let mut model = sectioned_model();
        let total = model.sidebar_item_count();
        let projects_end = SIDEBAR_FIRST_PROJECT_INDEX + model.projects.len();
        let contexts_end = model.sidebar_contexts_start() + model.all_contexts().len();
        let skipped = [
            SIDEBAR_SEPARATOR_INDEX,
            projects_end,
            projects_end + 1,
            contexts_end,
            contexts_end + 1,
        ];

        model.sidebar_selected = 0;
        let mut visited = vec![0];
        for _ in 0..total {
            let before = model.sidebar_selected;
            nav(&mut model, NavigationMessage::Down);
            if model.sidebar_selected == before {
                break;
            }
            visited.push(model.sidebar_selected);
        }
        assert_eq!(*visited.last().unwrap(), total - 1);
        for index in &skipped {
            assert!(!visited.contains(index), "down skipped {index}");
        }

        let mut visited_up = vec![model.sidebar_selected];
        for _ in 0..total {
            let before = model.sidebar_selected;
            nav(&mut model, NavigationMessage::Up);
            if model.sidebar_selected == before {
                break;
            }
            visited_up.push(model.sidebar_selected);
        }
        assert_eq!(*visited_up.last().unwrap(), 0);
        for index in &skipped {
            assert!(!visited_up.contains(index), "up skipped {index}");
        }
    }

    #[test]
    fn test_navigation_with_no_projects_or_contexts_uses_placeholder_rows() {
        let mut model = Model::new();
        model.focus_pane = FocusPane::Sidebar;
        // Placeholder rows ("No projects", "Use @tag", "Press F") are selectable
        model.sidebar_selected = SIDEBAR_FIRST_PROJECT_INDEX;
        nav(&mut model, NavigationMessage::Down);
        assert_eq!(model.sidebar_selected, model.sidebar_contexts_start());
        nav(&mut model, NavigationMessage::Down);
        assert_eq!(model.sidebar_selected, model.sidebar_saved_filters_start());
        nav(&mut model, NavigationMessage::Up);
        assert_eq!(model.sidebar_selected, model.sidebar_contexts_start());

        // Selecting the placeholder rows does nothing
        nav(&mut model, NavigationMessage::SelectSidebarItem);
        assert!(model.filtering.filter.tags.is_none());
        model.sidebar_selected = model.sidebar_saved_filters_start();
        nav(&mut model, NavigationMessage::SelectSidebarItem);
        assert!(model.active_saved_filter.is_none());
        model.sidebar_selected = SIDEBAR_FIRST_PROJECT_INDEX;
        nav(&mut model, NavigationMessage::SelectSidebarItem);
        assert!(model.selected_project.is_none());
    }

    #[test]
    fn test_focus_sidebar_requires_visible_sidebar() {
        let mut model = Model::new();
        model.show_sidebar = false;
        nav(&mut model, NavigationMessage::FocusSidebar);
        assert_eq!(model.focus_pane, FocusPane::TaskList);
    }
}
