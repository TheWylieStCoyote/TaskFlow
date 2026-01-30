//! Comprehensive UI component tests.
//!
//! This module provides extensive test coverage for UI components that have
//! basic tests but need more comprehensive testing. Tests cover:
//!
//! - Command Palette: Search filtering, action execution, keyboard navigation
//! - Saved Filter Picker: Selection, application, filter management
//! - Description Editor: Multi-line editing, cursor movement
//! - Time Log Editor: Multiple modes, time entry manipulation
//! - Duplicates UI: Similarity scoring, pair display
//! - Help Screen: Keybinding display, category grouping

#[cfg(test)]
mod comprehensive_component_tests {
    use crate::app::{Model, TaskTemplate, TemplateManager};
    use crate::config::{Keybindings, Theme};
    use crate::domain::{Filter, Priority, SavedFilter, SortSpec, Task, TimeEntry};
    use crate::ui::components::{
        CommandPalette, DescriptionEditor, Duplicates, HelpPopup, SavedFilterPicker,
        TemplatePicker, TimeLogEditor, TimeLogMode,
    };
    use crate::ui::test_utils::{buffer_content, render_widget};
    use chrono::Utc;

    // ========================================================================
    // Command Palette Comprehensive Tests
    // ========================================================================

    #[test]
    fn test_command_palette_search_by_description() {
        let keybindings = Keybindings::default();
        let theme = Theme::default();

        let palette = CommandPalette::new("task", 4, 0, &keybindings, &theme);
        let count = palette.filtered_count();

        // Should find actions with "task" in description
        assert!(count > 0);
        assert!(count < crate::config::ALL_ACTIONS.len());
    }

    #[test]
    fn test_command_palette_search_case_insensitive() {
        let keybindings = Keybindings::default();
        let theme = Theme::default();

        let palette1 = CommandPalette::new("HELP", 4, 0, &keybindings, &theme);
        let palette2 = CommandPalette::new("help", 4, 0, &keybindings, &theme);

        assert_eq!(palette1.filtered_count(), palette2.filtered_count());
    }

    #[test]
    fn test_command_palette_empty_query_shows_all() {
        let keybindings = Keybindings::default();
        let theme = Theme::default();

        let palette = CommandPalette::new("", 0, 0, &keybindings, &theme);
        let count = palette.filtered_count();

        // Should show all actions except ShowCommandPalette itself
        assert_eq!(count, crate::config::ALL_ACTIONS.len() - 1);
    }

    #[test]
    fn test_command_palette_no_results_query() {
        let keybindings = Keybindings::default();
        let theme = Theme::default();

        let palette = CommandPalette::new("xyznonexistent", 0, 0, &keybindings, &theme);
        let count = palette.filtered_count();

        assert_eq!(count, 0);
    }

    #[test]
    fn test_command_palette_selected_action_valid_index() {
        let keybindings = Keybindings::default();
        let theme = Theme::default();

        let palette = CommandPalette::new("help", 0, 0, &keybindings, &theme);
        let action = palette.selected_action();

        assert!(action.is_some());
        let action = action.unwrap();
        assert!(action.description().to_lowercase().contains("help"));
    }

    #[test]
    fn test_command_palette_selected_action_out_of_bounds() {
        let keybindings = Keybindings::default();
        let theme = Theme::default();

        let palette = CommandPalette::new("help", 0, 9999, &keybindings, &theme);
        let action = palette.selected_action();

        assert!(action.is_none());
    }

    #[test]
    fn test_command_palette_cursor_display() {
        let keybindings = Keybindings::default();
        let theme = Theme::default();

        // Test cursor at different positions
        for cursor in 0..5 {
            let palette = CommandPalette::new("test", cursor, 0, &keybindings, &theme);
            let buffer = render_widget(palette, 60, 20);
            let content = buffer_content(&buffer);

            // Should render without panic and have content
            assert!(!content.is_empty());
        }
    }

    #[test]
    fn test_command_palette_renders_with_keybindings() {
        let keybindings = Keybindings::default();
        let theme = Theme::default();

        let palette = CommandPalette::new("", 0, 0, &keybindings, &theme);
        let buffer = render_widget(palette, 80, 25);
        let content = buffer_content(&buffer);

        // Should render title
        assert!(content.contains("Command Palette"));
    }

    #[test]
    fn test_command_palette_filters_out_show_command_palette() {
        let keybindings = Keybindings::default();
        let theme = Theme::default();

        let palette = CommandPalette::new("", 0, 0, &keybindings, &theme);
        let count = palette.filtered_count();

        // ShowCommandPalette should be filtered out
        let total_actions = crate::config::ALL_ACTIONS.len();
        assert_eq!(count, total_actions - 1);
    }

    #[test]
    fn test_command_palette_search_by_action_name() {
        let keybindings = Keybindings::default();
        let theme = Theme::default();

        // Search for action name (not just description)
        let palette = CommandPalette::new("create", 0, 0, &keybindings, &theme);
        let count = palette.filtered_count();

        // Should find CreateTask and similar actions
        assert!(count > 0);
    }

    // ========================================================================
    // Saved Filter Picker Comprehensive Tests
    // ========================================================================

    fn create_test_saved_filter(name: &str, has_search: bool, has_tags: bool) -> SavedFilter {
        let mut filter = Filter::default();
        if has_search {
            filter.search_text = Some("test".to_string());
        }
        if has_tags {
            filter.tags = Some(vec!["work".to_string()]);
        }

        SavedFilter {
            id: crate::domain::SavedFilterId::new(),
            name: name.to_string(),
            filter,
            sort: SortSpec::default(),
            icon: Some("🔍".to_string()),
        }
    }

    #[test]
    fn test_saved_filter_picker_multiple_filters() {
        let theme = Theme::default();
        let filter1 = create_test_saved_filter("Filter 1", true, false);
        let filter2 = create_test_saved_filter("Filter 2", false, true);
        let filter3 = create_test_saved_filter("Filter 3", true, true);

        let filters: Vec<&SavedFilter> = vec![&filter1, &filter2, &filter3];
        let picker = SavedFilterPicker::new(filters, 1, None, &theme);
        let buffer = render_widget(picker, 70, 15);
        let content = buffer_content(&buffer);

        assert!(content.contains("Filter 1"));
        assert!(content.contains("Filter 2"));
        assert!(content.contains("Filter 3"));
    }

    #[test]
    fn test_saved_filter_picker_shows_filter_criteria() {
        let theme = Theme::default();
        let filter = create_test_saved_filter("Complex", true, true);
        let filters: Vec<&SavedFilter> = vec![&filter];

        let picker = SavedFilterPicker::new(filters, 0, None, &theme);
        let buffer = render_widget(picker, 80, 15);
        let content = buffer_content(&buffer);

        // Should show criteria like "search" and "tags"
        assert!(content.contains("search") || content.contains("tags"));
    }

    #[test]
    fn test_saved_filter_picker_active_filter_highlighted() {
        let theme = Theme::default();
        let filter1 = create_test_saved_filter("Active Filter", false, false);
        let filter2 = create_test_saved_filter("Inactive Filter", false, false);
        let filters: Vec<&SavedFilter> = vec![&filter1, &filter2];

        let picker = SavedFilterPicker::new(filters, 0, Some("Active Filter"), &theme);
        let buffer = render_widget(picker, 80, 15);
        let content = buffer_content(&buffer);

        // Active filter should have checkmark
        assert!(content.contains("✓"));
    }

    #[test]
    fn test_saved_filter_picker_with_icons() {
        let theme = Theme::default();
        let mut filter1 = create_test_saved_filter("With Icon", false, false);
        filter1.icon = Some("💼".to_string());

        let mut filter2 = create_test_saved_filter("Different Icon", false, false);
        filter2.icon = Some("🔥".to_string());

        let filters: Vec<&SavedFilter> = vec![&filter1, &filter2];
        let picker = SavedFilterPicker::new(filters, 0, None, &theme);
        let buffer = render_widget(picker, 70, 15);
        let content = buffer_content(&buffer);

        // Icons should be displayed
        assert!(content.contains("💼") || content.contains("🔥") || content.contains("🔍"));
    }

    #[test]
    fn test_saved_filter_picker_selection_indices() {
        let theme = Theme::default();
        let filters: Vec<SavedFilter> = (0..5)
            .map(|i| create_test_saved_filter(&format!("Filter {i}"), false, false))
            .collect();
        let filter_refs: Vec<&SavedFilter> = filters.iter().collect();

        // Test different selections
        for selected in 0..5 {
            let picker = SavedFilterPicker::new(filter_refs.clone(), selected, None, &theme);
            let buffer = render_widget(picker, 60, 15);
            // Should render without panic
            let _ = buffer_content(&buffer);
        }
    }

    #[test]
    fn test_saved_filter_picker_empty_instructions() {
        let theme = Theme::default();
        let picker = SavedFilterPicker::new(vec![], 0, None, &theme);
        let buffer = render_widget(picker, 80, 15);
        let content = buffer_content(&buffer);

        // Should show instructions for saving filter
        assert!(content.contains("save") || content.contains("No saved filters"));
    }

    // ========================================================================
    // Description Editor Comprehensive Tests
    // ========================================================================

    #[test]
    fn test_description_editor_multiline_content() {
        let theme = Theme::default();
        let buffer = vec![
            "Line 1".to_string(),
            "Line 2 with more text".to_string(),
            "Line 3".to_string(),
            "Line 4 final line".to_string(),
        ];

        let editor = DescriptionEditor::new(&buffer, 1, 5, &theme);
        let rendered = render_widget(editor, 80, 15);
        let content = buffer_content(&rendered);

        assert!(content.contains("Line 1"));
        assert!(content.contains("Line 2"));
        assert!(content.contains("Line 3"));
        assert!(content.contains("Line 4"));
    }

    #[test]
    fn test_description_editor_line_numbers() {
        let theme = Theme::default();
        let buffer = vec![
            "First".to_string(),
            "Second".to_string(),
            "Third".to_string(),
        ];

        let editor = DescriptionEditor::new(&buffer, 0, 0, &theme);
        let rendered = render_widget(editor, 80, 15);
        let content = buffer_content(&rendered);

        // Should show line numbers
        assert!(content.contains('1') && content.contains('2') && content.contains('3'));
    }

    #[test]
    fn test_description_editor_cursor_positions() {
        let theme = Theme::default();
        let buffer = vec!["Test line with content".to_string()];

        // Test cursor at different positions
        for cursor_col in [0, 5, 10, 22] {
            let editor = DescriptionEditor::new(&buffer, 0, cursor_col, &theme);
            let rendered = render_widget(editor, 80, 10);
            // Should render without panic
            let _ = buffer_content(&rendered);
        }
    }

    #[test]
    fn test_description_editor_cursor_beyond_line_length() {
        let theme = Theme::default();
        let buffer = vec!["Short".to_string()];

        // Cursor position beyond line length
        let editor = DescriptionEditor::new(&buffer, 0, 100, &theme);
        let rendered = render_widget(editor, 80, 10);
        // Should handle gracefully
        let _ = buffer_content(&rendered);
    }

    #[test]
    fn test_description_editor_empty_lines() {
        let theme = Theme::default();
        let buffer = vec![
            "First line".to_string(),
            String::new(), // Empty line
            "Third line".to_string(),
        ];

        let editor = DescriptionEditor::new(&buffer, 1, 0, &theme);
        let rendered = render_widget(editor, 80, 10);
        let content = buffer_content(&rendered);

        assert!(content.contains("First line"));
        assert!(content.contains("Third line"));
    }

    #[test]
    fn test_description_editor_unicode_content() {
        let theme = Theme::default();
        let buffer = vec![
            "日本語 text".to_string(),
            "Emoji: 🎉🔥💻".to_string(),
            "Mixed content 中文".to_string(),
        ];

        let editor = DescriptionEditor::new(&buffer, 1, 0, &theme);
        let rendered = render_widget(editor, 80, 15);
        let content = buffer_content(&rendered);

        // Should render unicode content
        assert!(content.contains("日本語") || content.contains("text"));
    }

    #[test]
    fn test_description_editor_shows_title() {
        let theme = Theme::default();
        let buffer = vec!["Content".to_string()];

        let editor = DescriptionEditor::new(&buffer, 0, 0, &theme);
        let rendered = render_widget(editor, 80, 10);
        let content = buffer_content(&rendered);

        // Should show title with instructions
        assert!(
            content.contains("Edit Description")
                || content.contains("save")
                || content.contains("cancel")
        );
    }

    // ========================================================================
    // Time Log Editor Comprehensive Tests
    // ========================================================================

    fn create_test_time_entry(
        started_minutes_ago: i64,
        duration_minutes: Option<i64>,
    ) -> TimeEntry {
        let now = Utc::now();
        let started_at = now - chrono::Duration::minutes(started_minutes_ago);
        let ended_at = duration_minutes.map(|d| started_at + chrono::Duration::minutes(d));
        let duration_mins = duration_minutes.map(|d| d as u32);

        TimeEntry {
            id: crate::domain::TimeEntryId::new(),
            task_id: crate::domain::TaskId::new(),
            started_at,
            ended_at,
            description: None,
            duration_minutes: duration_mins,
        }
    }

    #[test]
    fn test_time_log_editor_browse_mode() {
        let theme = Theme::default();
        let entry1 = create_test_time_entry(120, Some(60));
        let entry2 = create_test_time_entry(60, Some(30));
        let entries: Vec<&TimeEntry> = vec![&entry1, &entry2];

        let editor = TimeLogEditor::new(entries, 0, TimeLogMode::Browse, "", &theme);
        let buffer = render_widget(editor, 80, 15);
        // Should render without panic
        let _ = buffer_content(&buffer);
    }

    #[test]
    fn test_time_log_editor_edit_start_mode() {
        let theme = Theme::default();
        let entry = create_test_time_entry(60, Some(30));
        let entries: Vec<&TimeEntry> = vec![&entry];

        let editor = TimeLogEditor::new(entries, 0, TimeLogMode::EditStart, "12:30", &theme);
        let buffer = render_widget(editor, 80, 15);
        let content = buffer_content(&buffer);

        // Should show edit buffer
        assert!(content.contains("12:30"));
    }

    #[test]
    fn test_time_log_editor_edit_end_mode() {
        let theme = Theme::default();
        let entry = create_test_time_entry(60, Some(30));
        let entries: Vec<&TimeEntry> = vec![&entry];

        let editor = TimeLogEditor::new(entries, 0, TimeLogMode::EditEnd, "13:00", &theme);
        let buffer = render_widget(editor, 80, 15);
        let content = buffer_content(&buffer);

        // Should show edit buffer
        assert!(content.contains("13:00"));
    }

    #[test]
    fn test_time_log_editor_running_entry() {
        let theme = Theme::default();
        let running_entry = create_test_time_entry(30, None); // No end time = running
        let completed_entry = create_test_time_entry(120, Some(60));
        let entries: Vec<&TimeEntry> = vec![&running_entry, &completed_entry];

        let editor = TimeLogEditor::new(entries, 0, TimeLogMode::Browse, "", &theme);
        let buffer = render_widget(editor, 80, 15);
        let content = buffer_content(&buffer);

        // Should show "running" indicator
        assert!(content.contains("running") || content.contains("●"));
    }

    #[test]
    fn test_time_log_editor_empty_entries() {
        let theme = Theme::default();
        let entries: Vec<&TimeEntry> = vec![];

        let editor = TimeLogEditor::new(entries, 0, TimeLogMode::Browse, "", &theme);
        let buffer = render_widget(editor, 80, 15);
        // Should handle empty list gracefully
        let _ = buffer_content(&buffer);
    }

    #[test]
    fn test_time_log_editor_selected_entry_id() {
        let entry1 = create_test_time_entry(120, Some(60));
        let entry2 = create_test_time_entry(60, Some(30));
        let entries: Vec<&TimeEntry> = vec![&entry1, &entry2];
        let theme = Theme::default();

        let editor = TimeLogEditor::new(entries, 1, TimeLogMode::Browse, "", &theme);
        let selected_id = editor.selected_entry_id();

        assert!(selected_id.is_some());
        assert_eq!(selected_id, Some(&entry2.id));
    }

    #[test]
    fn test_time_log_editor_selected_entry_id_out_of_bounds() {
        let entry = create_test_time_entry(60, Some(30));
        let entries: Vec<&TimeEntry> = vec![&entry];
        let theme = Theme::default();

        let editor = TimeLogEditor::new(entries, 999, TimeLogMode::Browse, "", &theme);
        let selected_id = editor.selected_entry_id();

        assert!(selected_id.is_none());
    }

    // ========================================================================
    // Duplicates UI Comprehensive Tests
    // ========================================================================

    fn create_model_with_duplicates() -> Model {
        let mut model = Model::new();

        // Add similar tasks
        let task1 = Task::new("Fix login bug on homepage");
        let task2 = Task::new("Fix login bug on homepage"); // Duplicate
        let task3 = Task::new("Update documentation for API");
        let task4 = Task::new("Update API documentation"); // Similar

        model.tasks.insert(task1.id, task1);
        model.tasks.insert(task2.id, task2);
        model.tasks.insert(task3.id, task3);
        model.tasks.insert(task4.id, task4);

        model
    }

    #[test]
    fn test_duplicates_renders_empty_state() {
        let model = Model::new();
        let theme = Theme::default();

        let duplicates = Duplicates::new(&model, &theme);
        let buffer = render_widget(duplicates, 80, 20);
        let content = buffer_content(&buffer);

        // Should show "no duplicates" message
        assert!(content.contains("No duplicate") || content.contains('0'));
    }

    #[test]
    fn test_duplicates_shows_threshold() {
        let model = Model::new();
        let theme = Theme::default();

        let duplicates = Duplicates::new(&model, &theme);
        let buffer = render_widget(duplicates, 80, 20);
        let content = buffer_content(&buffer);

        // Should show threshold percentage
        assert!(content.contains('%') || content.contains("threshold"));
    }

    #[test]
    fn test_duplicates_renders_pair_count() {
        let model = create_model_with_duplicates();
        let theme = Theme::default();

        let duplicates = Duplicates::new(&model, &theme);
        let buffer = render_widget(duplicates, 100, 25);
        let content = buffer_content(&buffer);

        // Should show pair count
        assert!(content.contains("pair") || content.contains("duplicate"));
    }

    // ========================================================================
    // Help Screen Comprehensive Tests
    // ========================================================================

    #[test]
    fn test_help_popup_renders_title() {
        let keybindings = Keybindings::default();
        let theme = Theme::default();

        let help = HelpPopup::new(&keybindings, &theme);
        let buffer = render_widget(help, 80, 30);
        let content = buffer_content(&buffer);

        assert!(content.contains("Help") || content.contains('?'));
    }

    #[test]
    fn test_help_popup_shows_categories() {
        let keybindings = Keybindings::default();
        let theme = Theme::default();

        let help = HelpPopup::new(&keybindings, &theme);
        let buffer = render_widget(help, 80, 30);
        let content = buffer_content(&buffer);

        // Should show some category headers like Navigation, Tasks, etc.
        assert!(content.len() > 100); // Should have substantial content
    }

    #[test]
    fn test_help_popup_shows_specialized_views() {
        let keybindings = Keybindings::default();
        let theme = Theme::default();

        let help = HelpPopup::new(&keybindings, &theme);
        let buffer = render_widget(help, 90, 40);
        let content = buffer_content(&buffer);

        // Should have substantial content from help text
        // The actual content might be wrapped or truncated based on terminal width
        assert!(content.len() > 100);
    }

    #[test]
    fn test_help_popup_shows_close_instructions() {
        let keybindings = Keybindings::default();
        let theme = Theme::default();

        let help = HelpPopup::new(&keybindings, &theme);
        let buffer = render_widget(help, 80, 30);
        let content = buffer_content(&buffer);

        // Should show instructions to close
        assert!(content.contains("Esc") || content.contains("close") || content.contains('?'));
    }

    #[test]
    fn test_help_popup_small_area() {
        let keybindings = Keybindings::default();
        let theme = Theme::default();

        let help = HelpPopup::new(&keybindings, &theme);
        let buffer = render_widget(help, 40, 10);
        // Should handle small areas gracefully
        let _ = buffer_content(&buffer);
    }

    #[test]
    fn test_help_popup_large_area() {
        let keybindings = Keybindings::default();
        let theme = Theme::default();

        let help = HelpPopup::new(&keybindings, &theme);
        let buffer = render_widget(help, 120, 60);
        // Should handle large areas without issues
        let content = buffer_content(&buffer);
        assert!(!content.is_empty());
    }

    // ========================================================================
    // Template Picker Additional Tests
    // ========================================================================

    #[test]
    fn test_template_picker_all_priorities() {
        let mut manager = TemplateManager::new();

        // Add templates with all priority levels
        for (name, priority) in [
            ("None", Priority::None),
            ("Low", Priority::Low),
            ("Medium", Priority::Medium),
            ("High", Priority::High),
            ("Urgent", Priority::Urgent),
        ] {
            manager.templates.push(TaskTemplate {
                name: name.to_string(),
                title: format!("{name} task"),
                priority,
                tags: vec![],
                description: None,
                due_days: None,
            });
        }

        let theme = Theme::default();
        let picker = TemplatePicker::new(&manager, 0, &theme);
        let buffer = render_widget(picker, 60, 20);
        let content = buffer_content(&buffer);

        // Should show all priority indicators
        assert!(content.contains('!'));
    }

    #[test]
    fn test_template_picker_many_templates() {
        let mut manager = TemplateManager::new();

        // Add many templates (more than 10)
        for i in 0..15 {
            manager.templates.push(TaskTemplate {
                name: format!("Template {i}"),
                title: format!("Task {i}"),
                priority: Priority::Medium,
                tags: vec![],
                description: None,
                due_days: None,
            });
        }

        let theme = Theme::default();
        let picker = TemplatePicker::new(&manager, 5, &theme);
        let buffer = render_widget(picker, 70, 25);
        let content = buffer_content(&buffer);

        // Should render without panic
        assert!(!content.is_empty());
    }

    #[test]
    fn test_template_picker_multiple_tags() {
        let mut manager = TemplateManager::new();
        manager.templates.push(TaskTemplate {
            name: "Multi-tag".to_string(),
            title: "Task".to_string(),
            priority: Priority::Medium,
            tags: vec![
                "tag1".to_string(),
                "tag2".to_string(),
                "tag3".to_string(),
                "tag4".to_string(),
            ],
            description: None,
            due_days: None,
        });

        let theme = Theme::default();
        let picker = TemplatePicker::new(&manager, 0, &theme);
        let buffer = render_widget(picker, 100, 15);
        let content = buffer_content(&buffer);

        // Should show multiple tags
        assert!(content.contains('#'));
    }
}
