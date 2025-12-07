use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::Settings;

/// Action that can be triggered by a keybinding
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    // Navigation
    MoveUp,
    MoveDown,
    MoveFirst,
    MoveLast,
    PageUp,
    PageDown,

    // Task actions
    ToggleComplete,
    CreateTask,
    CreateSubtask,
    CreateProject,
    EditProject,
    DeleteProject,
    EditTask,
    EditDueDate,
    EditScheduledDate,
    EditTags,
    EditDescription,
    DeleteTask,
    CyclePriority,
    MoveToProject,

    // Time tracking
    ToggleTimeTracking,
    ShowTimeLog,

    // UI actions
    ToggleSidebar,
    ToggleShowCompleted,
    ShowHelp,
    ToggleFocusMode,
    FocusSidebar,
    FocusTaskList,
    Select,
    Search,
    ClearSearch,
    FilterByTag,
    ClearTagFilter,
    CycleSortField,
    ToggleSortOrder,

    // Multi-select / Bulk operations
    ToggleMultiSelect,
    ToggleTaskSelection,
    SelectAll,
    ClearSelection,
    BulkDelete,
    BulkMoveToProject,
    BulkSetStatus,

    // Dependencies
    EditDependencies,

    // Recurrence
    EditRecurrence,

    // Manual ordering
    MoveTaskUp,
    MoveTaskDown,

    // Task chains
    LinkTask,
    UnlinkTask,

    // Calendar navigation
    CalendarPrevMonth,
    CalendarNextMonth,
    CalendarPrevDay,
    CalendarNextDay,

    // Reports navigation
    ReportsNextPanel,
    ReportsPrevPanel,

    // System
    Save,
    Undo,
    Redo,
    Quit,

    // Export
    ExportCsv,
    ExportIcs,
    ExportChainsDot,
    ExportChainsMermaid,
    ExportReportMarkdown,
    ExportReportHtml,

    // Import
    ImportCsv,
    ImportIcs,

    // Macros
    RecordMacro,
    StopRecordMacro,
    PlayMacro0,
    PlayMacro1,
    PlayMacro2,
    PlayMacro3,
    PlayMacro4,
    PlayMacro5,
    PlayMacro6,
    PlayMacro7,
    PlayMacro8,
    PlayMacro9,

    // Templates
    ShowTemplates,

    // Keybindings editor
    ShowKeybindingsEditor,

    // Pomodoro timer
    PomodoroStart,
    PomodoroPause,
    PomodoroResume,
    PomodoroTogglePause,
    PomodoroSkip,
    PomodoroStop,
}

/// Category for grouping actions in help display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionCategory {
    Navigation,
    Tasks,
    Projects,
    TimeTracking,
    ViewFilter,
    MultiSelect,
    Dependencies,
    Recurrence,
    TaskChains,
    Calendar,
    Reports,
    Export,
    Import,
    Macros,
    Templates,
    Pomodoro,
    System,
}

impl ActionCategory {
    /// Get display name for the category
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Navigation => "Navigation",
            Self::Tasks => "Tasks",
            Self::Projects => "Projects",
            Self::TimeTracking => "Time Tracking",
            Self::ViewFilter => "View & Filter",
            Self::MultiSelect => "Multi-Select",
            Self::Dependencies => "Dependencies",
            Self::Recurrence => "Recurrence",
            Self::TaskChains => "Task Chains",
            Self::Calendar => "Calendar",
            Self::Reports => "Reports",
            Self::Export => "Export",
            Self::Import => "Import",
            Self::Macros => "Macros",
            Self::Templates => "Templates",
            Self::Pomodoro => "Pomodoro Timer",
            Self::System => "System",
        }
    }

    /// Get display order for sorting categories
    #[must_use]
    pub const fn display_order(&self) -> u8 {
        match self {
            Self::Navigation => 0,
            Self::Tasks => 1,
            Self::Projects => 2,
            Self::TimeTracking => 3,
            Self::ViewFilter => 4,
            Self::MultiSelect => 5,
            Self::Dependencies => 6,
            Self::Recurrence => 7,
            Self::TaskChains => 8,
            Self::Calendar => 9,
            Self::Reports => 10,
            Self::Export => 11,
            Self::Import => 12,
            Self::Macros => 13,
            Self::Templates => 14,
            Self::Pomodoro => 15,
            Self::System => 16,
        }
    }
}

impl Action {
    /// Get a human-readable description of the action
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            // Navigation
            Self::MoveUp => "Move up",
            Self::MoveDown => "Move down",
            Self::MoveFirst => "Go to first",
            Self::MoveLast => "Go to last",
            Self::PageUp => "Page up",
            Self::PageDown => "Page down",
            // Task actions
            Self::ToggleComplete => "Toggle complete",
            Self::CreateTask => "Create task",
            Self::CreateSubtask => "Create subtask",
            Self::EditTask => "Edit task title",
            Self::EditDueDate => "Edit due date",
            Self::EditScheduledDate => "Edit scheduled date",
            Self::EditTags => "Edit tags",
            Self::EditDescription => "Edit description",
            Self::DeleteTask => "Delete task",
            Self::CyclePriority => "Cycle priority",
            Self::MoveToProject => "Move to project",
            // Project actions
            Self::CreateProject => "Create project",
            Self::EditProject => "Edit project",
            Self::DeleteProject => "Delete project",
            // Time tracking
            Self::ToggleTimeTracking => "Toggle time tracking",
            Self::ShowTimeLog => "Show time log",
            // UI actions
            Self::ToggleSidebar => "Toggle sidebar",
            Self::ToggleShowCompleted => "Toggle show completed",
            Self::ShowHelp => "Show help",
            Self::ToggleFocusMode => "Toggle focus mode",
            Self::FocusSidebar => "Focus sidebar",
            Self::FocusTaskList => "Focus task list",
            Self::Select => "Select item",
            Self::Search => "Search tasks",
            Self::ClearSearch => "Clear search",
            Self::FilterByTag => "Filter by tag",
            Self::ClearTagFilter => "Clear tag filter",
            Self::CycleSortField => "Cycle sort field",
            Self::ToggleSortOrder => "Toggle sort order",
            // Multi-select
            Self::ToggleMultiSelect => "Toggle multi-select",
            Self::ToggleTaskSelection => "Toggle task selection",
            Self::SelectAll => "Select all",
            Self::ClearSelection => "Clear selection",
            Self::BulkDelete => "Bulk delete",
            Self::BulkMoveToProject => "Bulk move to project",
            Self::BulkSetStatus => "Bulk set status",
            // Dependencies
            Self::EditDependencies => "Edit dependencies",
            // Recurrence
            Self::EditRecurrence => "Edit recurrence",
            // Manual ordering
            Self::MoveTaskUp => "Move task up",
            Self::MoveTaskDown => "Move task down",
            // Task chains
            Self::LinkTask => "Link to next task",
            Self::UnlinkTask => "Unlink from chain",
            // Calendar
            Self::CalendarPrevMonth => "Previous month",
            Self::CalendarNextMonth => "Next month",
            Self::CalendarPrevDay => "Previous day",
            Self::CalendarNextDay => "Next day",
            // Reports
            Self::ReportsNextPanel => "Next panel",
            Self::ReportsPrevPanel => "Previous panel",
            // System
            Self::Save => "Save",
            Self::Undo => "Undo",
            Self::Redo => "Redo",
            Self::Quit => "Quit",
            // Export
            Self::ExportCsv => "Export to CSV",
            Self::ExportIcs => "Export to ICS",
            Self::ExportChainsDot => "Export chains (DOT)",
            Self::ExportChainsMermaid => "Export chains (Mermaid)",
            Self::ExportReportMarkdown => "Export report (Markdown)",
            Self::ExportReportHtml => "Export report (HTML)",
            // Import
            Self::ImportCsv => "Import from CSV",
            Self::ImportIcs => "Import from ICS",
            // Macros
            Self::RecordMacro => "Record macro",
            Self::StopRecordMacro => "Stop recording",
            Self::PlayMacro0 => "Play macro 0",
            Self::PlayMacro1 => "Play macro 1",
            Self::PlayMacro2 => "Play macro 2",
            Self::PlayMacro3 => "Play macro 3",
            Self::PlayMacro4 => "Play macro 4",
            Self::PlayMacro5 => "Play macro 5",
            Self::PlayMacro6 => "Play macro 6",
            Self::PlayMacro7 => "Play macro 7",
            Self::PlayMacro8 => "Play macro 8",
            Self::PlayMacro9 => "Play macro 9",
            // Templates
            Self::ShowTemplates => "Show templates",
            // Keybindings
            Self::ShowKeybindingsEditor => "Edit keybindings",
            // Pomodoro
            Self::PomodoroStart => "Start Pomodoro",
            Self::PomodoroPause => "Pause timer",
            Self::PomodoroResume => "Resume timer",
            Self::PomodoroTogglePause => "Toggle pause",
            Self::PomodoroSkip => "Skip phase",
            Self::PomodoroStop => "Stop Pomodoro",
        }
    }

    /// Get the category this action belongs to
    #[must_use]
    pub const fn category(&self) -> ActionCategory {
        match self {
            Self::MoveUp
            | Self::MoveDown
            | Self::MoveFirst
            | Self::MoveLast
            | Self::PageUp
            | Self::PageDown => ActionCategory::Navigation,

            Self::ToggleComplete
            | Self::CreateTask
            | Self::CreateSubtask
            | Self::EditTask
            | Self::EditDueDate
            | Self::EditScheduledDate
            | Self::EditTags
            | Self::EditDescription
            | Self::DeleteTask
            | Self::CyclePriority
            | Self::MoveToProject
            | Self::MoveTaskUp
            | Self::MoveTaskDown => ActionCategory::Tasks,

            Self::CreateProject | Self::EditProject | Self::DeleteProject => {
                ActionCategory::Projects
            }

            Self::ToggleTimeTracking | Self::ShowTimeLog => ActionCategory::TimeTracking,

            Self::ToggleSidebar
            | Self::ToggleShowCompleted
            | Self::ShowHelp
            | Self::ToggleFocusMode
            | Self::FocusSidebar
            | Self::FocusTaskList
            | Self::Select
            | Self::Search
            | Self::ClearSearch
            | Self::FilterByTag
            | Self::ClearTagFilter
            | Self::CycleSortField
            | Self::ToggleSortOrder => ActionCategory::ViewFilter,

            Self::ToggleMultiSelect
            | Self::ToggleTaskSelection
            | Self::SelectAll
            | Self::ClearSelection
            | Self::BulkDelete
            | Self::BulkMoveToProject
            | Self::BulkSetStatus => ActionCategory::MultiSelect,

            Self::EditDependencies => ActionCategory::Dependencies,
            Self::EditRecurrence => ActionCategory::Recurrence,
            Self::LinkTask | Self::UnlinkTask => ActionCategory::TaskChains,

            Self::CalendarPrevMonth
            | Self::CalendarNextMonth
            | Self::CalendarPrevDay
            | Self::CalendarNextDay => ActionCategory::Calendar,

            Self::ReportsNextPanel | Self::ReportsPrevPanel => ActionCategory::Reports,

            Self::ExportCsv
            | Self::ExportIcs
            | Self::ExportChainsDot
            | Self::ExportChainsMermaid
            | Self::ExportReportMarkdown
            | Self::ExportReportHtml => ActionCategory::Export,

            Self::ImportCsv | Self::ImportIcs => ActionCategory::Import,

            Self::RecordMacro
            | Self::StopRecordMacro
            | Self::PlayMacro0
            | Self::PlayMacro1
            | Self::PlayMacro2
            | Self::PlayMacro3
            | Self::PlayMacro4
            | Self::PlayMacro5
            | Self::PlayMacro6
            | Self::PlayMacro7
            | Self::PlayMacro8
            | Self::PlayMacro9 => ActionCategory::Macros,

            Self::ShowTemplates => ActionCategory::Templates,
            Self::ShowKeybindingsEditor => ActionCategory::System,

            Self::PomodoroStart
            | Self::PomodoroPause
            | Self::PomodoroResume
            | Self::PomodoroTogglePause
            | Self::PomodoroSkip
            | Self::PomodoroStop => ActionCategory::Pomodoro,

            Self::Save | Self::Undo | Self::Redo | Self::Quit => ActionCategory::System,
        }
    }
}

/// Key modifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Modifier {
    #[default]
    None,
    Ctrl,
    Alt,
    Shift,
}

/// A key combination
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyBinding {
    /// The key character or name
    pub key: String,

    /// Modifier key
    #[serde(default)]
    pub modifier: Modifier,
}

impl KeyBinding {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            modifier: Modifier::None,
        }
    }

    pub fn with_ctrl(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            modifier: Modifier::Ctrl,
        }
    }

    pub fn with_shift(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            modifier: Modifier::Shift,
        }
    }
}

/// Keybindings configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Keybindings {
    /// Map of keybindings to actions
    pub bindings: HashMap<String, Action>,
}

impl Default for Keybindings {
    fn default() -> Self {
        let mut bindings = HashMap::new();

        // Navigation
        bindings.insert("j".to_string(), Action::MoveDown);
        bindings.insert("k".to_string(), Action::MoveUp);
        bindings.insert("down".to_string(), Action::MoveDown);
        bindings.insert("up".to_string(), Action::MoveUp);
        bindings.insert("g".to_string(), Action::MoveFirst);
        bindings.insert("G".to_string(), Action::MoveLast);
        bindings.insert("ctrl+u".to_string(), Action::PageUp);
        bindings.insert("ctrl+d".to_string(), Action::PageDown);
        bindings.insert("pageup".to_string(), Action::PageUp);
        bindings.insert("pagedown".to_string(), Action::PageDown);

        // Task actions
        bindings.insert("x".to_string(), Action::ToggleComplete);
        bindings.insert("space".to_string(), Action::ToggleComplete);
        bindings.insert("a".to_string(), Action::CreateTask);
        bindings.insert("A".to_string(), Action::CreateSubtask);
        bindings.insert("P".to_string(), Action::CreateProject);
        bindings.insert("E".to_string(), Action::EditProject);
        bindings.insert("X".to_string(), Action::DeleteProject);
        bindings.insert("e".to_string(), Action::EditTask);
        bindings.insert("D".to_string(), Action::EditDueDate);
        bindings.insert("S".to_string(), Action::EditScheduledDate);
        bindings.insert("T".to_string(), Action::EditTags);
        bindings.insert("n".to_string(), Action::EditDescription);
        bindings.insert("d".to_string(), Action::DeleteTask);
        bindings.insert("p".to_string(), Action::CyclePriority);
        bindings.insert("m".to_string(), Action::MoveToProject);

        // Time tracking
        bindings.insert("t".to_string(), Action::ToggleTimeTracking);
        bindings.insert("L".to_string(), Action::ShowTimeLog);

        // UI actions
        bindings.insert("b".to_string(), Action::ToggleSidebar);
        bindings.insert("c".to_string(), Action::ToggleShowCompleted);
        bindings.insert("?".to_string(), Action::ShowHelp);
        bindings.insert("f".to_string(), Action::ToggleFocusMode);
        bindings.insert("h".to_string(), Action::FocusSidebar);
        bindings.insert("l".to_string(), Action::FocusTaskList);
        bindings.insert("left".to_string(), Action::FocusSidebar);
        bindings.insert("right".to_string(), Action::FocusTaskList);
        bindings.insert("enter".to_string(), Action::Select);
        bindings.insert("/".to_string(), Action::Search);
        bindings.insert("ctrl+l".to_string(), Action::ClearSearch);
        bindings.insert("#".to_string(), Action::FilterByTag);
        bindings.insert("ctrl+t".to_string(), Action::ClearTagFilter);
        bindings.insert("s".to_string(), Action::CycleSortField);
        bindings.insert("ctrl+s".to_string(), Action::ToggleSortOrder);

        // Multi-select / Bulk operations
        bindings.insert("v".to_string(), Action::ToggleMultiSelect);
        bindings.insert("V".to_string(), Action::SelectAll);
        bindings.insert("ctrl+v".to_string(), Action::ClearSelection);

        // Dependencies
        bindings.insert("B".to_string(), Action::EditDependencies);

        // Recurrence
        bindings.insert("R".to_string(), Action::EditRecurrence);

        // Manual ordering
        bindings.insert("ctrl+up".to_string(), Action::MoveTaskUp);
        bindings.insert("ctrl+down".to_string(), Action::MoveTaskDown);

        // Task chains
        bindings.insert("ctrl+l".to_string(), Action::LinkTask);
        bindings.insert("ctrl+shift+l".to_string(), Action::UnlinkTask);

        // Calendar navigation
        bindings.insert("<".to_string(), Action::CalendarPrevMonth);
        bindings.insert(">".to_string(), Action::CalendarNextMonth);

        // System
        bindings.insert("ctrl+s".to_string(), Action::Save);
        bindings.insert("u".to_string(), Action::Undo);
        bindings.insert("ctrl+z".to_string(), Action::Undo);
        bindings.insert("ctrl+r".to_string(), Action::Redo);
        bindings.insert("U".to_string(), Action::Redo);
        bindings.insert("q".to_string(), Action::Quit);
        bindings.insert("esc".to_string(), Action::Quit);

        // Export
        bindings.insert("ctrl+e".to_string(), Action::ExportCsv);
        bindings.insert("ctrl+i".to_string(), Action::ExportIcs);
        bindings.insert("ctrl+g".to_string(), Action::ExportChainsDot);
        bindings.insert("ctrl+m".to_string(), Action::ExportChainsMermaid);
        bindings.insert("ctrl+p".to_string(), Action::ExportReportMarkdown);
        bindings.insert("ctrl+h".to_string(), Action::ExportReportHtml);

        // Import
        bindings.insert("I".to_string(), Action::ImportCsv); // Shift+I for CSV import
        bindings.insert("alt+i".to_string(), Action::ImportIcs); // Alt+I for ICS import

        // Reports navigation (Tab/Shift+Tab or l/h when in reports view)
        bindings.insert("tab".to_string(), Action::ReportsNextPanel);
        bindings.insert("shift+tab".to_string(), Action::ReportsPrevPanel);

        // Macros - q to record, Q to stop, @0-9 to play
        bindings.insert("ctrl+q".to_string(), Action::RecordMacro);
        bindings.insert("ctrl+Q".to_string(), Action::StopRecordMacro);
        bindings.insert("@0".to_string(), Action::PlayMacro0);
        bindings.insert("@1".to_string(), Action::PlayMacro1);
        bindings.insert("@2".to_string(), Action::PlayMacro2);
        bindings.insert("@3".to_string(), Action::PlayMacro3);
        bindings.insert("@4".to_string(), Action::PlayMacro4);
        bindings.insert("@5".to_string(), Action::PlayMacro5);
        bindings.insert("@6".to_string(), Action::PlayMacro6);
        bindings.insert("@7".to_string(), Action::PlayMacro7);
        bindings.insert("@8".to_string(), Action::PlayMacro8);
        bindings.insert("@9".to_string(), Action::PlayMacro9);

        // Templates
        bindings.insert("ctrl+n".to_string(), Action::ShowTemplates);

        // Keybindings editor
        bindings.insert("ctrl+k".to_string(), Action::ShowKeybindingsEditor);

        // Pomodoro timer
        bindings.insert("f5".to_string(), Action::PomodoroStart);
        bindings.insert("f6".to_string(), Action::PomodoroTogglePause);
        bindings.insert("f7".to_string(), Action::PomodoroSkip);
        bindings.insert("f8".to_string(), Action::PomodoroStop);

        Self { bindings }
    }
}

impl Keybindings {
    /// Load keybindings from the default config path
    #[must_use]
    pub fn load() -> Self {
        Self::load_from_path(Self::config_path())
    }

    /// Load keybindings from a specific path
    #[must_use]
    pub fn load_from_path(path: PathBuf) -> Self {
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match toml::from_str(&content) {
                    Ok(keybindings) => return keybindings,
                    Err(e) => eprintln!("Warning: Failed to parse keybindings: {e}"),
                },
                Err(e) => eprintln!("Warning: Failed to read keybindings: {e}"),
            }
        }
        Self::default()
    }

    /// Get the default keybindings file path
    #[must_use]
    pub fn config_path() -> PathBuf {
        Settings::config_dir().join("keybindings.toml")
    }

    /// Look up action for a key
    #[must_use]
    pub fn get_action(&self, key: &str) -> Option<&Action> {
        self.bindings.get(key)
    }

    /// Returns a sorted list of (key, action) pairs for display
    #[must_use]
    pub fn sorted_bindings(&self) -> Vec<(String, Action)> {
        let mut pairs: Vec<_> = self
            .bindings
            .iter()
            .map(|(k, a)| (k.clone(), a.clone()))
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        pairs
    }

    /// Set a keybinding for an action
    pub fn set_binding(&mut self, key: String, action: Action) {
        // Remove any existing binding for this action
        self.bindings.retain(|_, a| a != &action);
        // Add the new binding
        self.bindings.insert(key, action);
    }

    /// Find the key bound to an action
    #[must_use]
    pub fn key_for_action(&self, action: &Action) -> Option<&String> {
        self.bindings
            .iter()
            .find(|(_, a)| *a == action)
            .map(|(k, _)| k)
    }

    /// Get all bindings grouped by category, sorted for display in help
    ///
    /// Returns a Vec of (category, Vec<(key, action, description)>) sorted by category order
    #[must_use]
    #[allow(clippy::type_complexity)]
    pub fn bindings_by_category(
        &self,
    ) -> Vec<(ActionCategory, Vec<(String, &Action, &'static str)>)> {
        use std::collections::BTreeMap;

        // Type alias for the grouped bindings
        type GroupedBindings<'a> = (ActionCategory, Vec<(String, &'a Action, &'static str)>);

        // Group bindings by category
        let mut groups: BTreeMap<u8, GroupedBindings<'_>> = BTreeMap::new();

        for (key, action) in &self.bindings {
            let category = action.category();
            let order = category.display_order();
            let description = action.description();

            groups
                .entry(order)
                .or_insert_with(|| (category, Vec::new()))
                .1
                .push((key.clone(), action, description));
        }

        // Sort each group's bindings alphabetically by key
        for (_, (_, bindings)) in groups.iter_mut() {
            bindings.sort_by(|a, b| a.0.cmp(&b.0));
        }

        // Convert to Vec, already sorted by category order due to BTreeMap
        groups.into_values().collect()
    }

    /// Save keybindings to the config file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, content)
    }

    /// Check if a key is already bound to an action
    ///
    /// Returns the conflicting action if the key is already bound.
    #[must_use]
    pub fn find_conflict(&self, key: &str) -> Option<&Action> {
        self.bindings.get(key)
    }

    /// Set a binding with conflict detection
    ///
    /// Returns the previous action if the key was already bound (conflict).
    /// The binding is still set - caller should handle the conflict.
    pub fn set_binding_checked(&mut self, key: String, action: Action) -> Option<Action> {
        // First, check if this key is already bound to something else
        let previous = self.bindings.get(&key).cloned();

        // Remove any existing binding for this action (an action can only have one key)
        self.bindings.retain(|_, a| a != &action);

        // Add the new binding
        self.bindings.insert(key, action);

        // Return the previous action if there was one (and it was different)
        previous
    }

    /// Swap bindings between two keys
    ///
    /// If key1 is bound to action1 and key2 is bound to action2,
    /// after swap: key1 -> action2, key2 -> action1
    pub fn swap_bindings(&mut self, key1: &str, key2: &str) {
        let action1 = self.bindings.get(key1).cloned();
        let action2 = self.bindings.get(key2).cloned();

        if let Some(a1) = action1 {
            if let Some(a2) = action2 {
                self.bindings.insert(key1.to_string(), a2);
                self.bindings.insert(key2.to_string(), a1);
            }
        }
    }

    /// Remove binding for a specific key
    pub fn remove_binding(&mut self, key: &str) -> Option<Action> {
        self.bindings.remove(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_keybinding_new() {
        let kb = KeyBinding::new("j");
        assert_eq!(kb.key, "j");
        assert_eq!(kb.modifier, Modifier::None);
    }

    #[test]
    fn test_keybinding_with_ctrl() {
        let kb = KeyBinding::with_ctrl("s");
        assert_eq!(kb.key, "s");
        assert_eq!(kb.modifier, Modifier::Ctrl);
    }

    #[test]
    fn test_keybinding_with_shift() {
        let kb = KeyBinding::with_shift("G");
        assert_eq!(kb.key, "G");
        assert_eq!(kb.modifier, Modifier::Shift);
    }

    #[test]
    fn test_keybindings_default_navigation() {
        let kb = Keybindings::default();

        assert_eq!(kb.get_action("j"), Some(&Action::MoveDown));
        assert_eq!(kb.get_action("k"), Some(&Action::MoveUp));
        assert_eq!(kb.get_action("up"), Some(&Action::MoveUp));
        assert_eq!(kb.get_action("down"), Some(&Action::MoveDown));
        assert_eq!(kb.get_action("g"), Some(&Action::MoveFirst));
        assert_eq!(kb.get_action("G"), Some(&Action::MoveLast));
    }

    #[test]
    fn test_keybindings_default_tasks() {
        let kb = Keybindings::default();

        assert_eq!(kb.get_action("x"), Some(&Action::ToggleComplete));
        assert_eq!(kb.get_action("space"), Some(&Action::ToggleComplete));
        assert_eq!(kb.get_action("a"), Some(&Action::CreateTask));
        assert_eq!(kb.get_action("d"), Some(&Action::DeleteTask));
        assert_eq!(kb.get_action("t"), Some(&Action::ToggleTimeTracking));
    }

    #[test]
    fn test_keybindings_default_system() {
        let kb = Keybindings::default();

        assert_eq!(kb.get_action("q"), Some(&Action::Quit));
        assert_eq!(kb.get_action("esc"), Some(&Action::Quit));
        assert_eq!(kb.get_action("ctrl+s"), Some(&Action::Save));
    }

    #[test]
    fn test_keybindings_get_action() {
        let kb = Keybindings::default();

        assert_eq!(kb.get_action("j"), Some(&Action::MoveDown));
        assert_eq!(kb.get_action("?"), Some(&Action::ShowHelp));
    }

    #[test]
    fn test_keybindings_get_action_unknown() {
        let kb = Keybindings::default();

        assert_eq!(kb.get_action("unknown_key"), None);
    }

    #[test]
    fn test_keybindings_load_missing_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.toml");

        let kb = Keybindings::load_from_path(path);

        // Should return defaults
        assert_eq!(kb.get_action("j"), Some(&Action::MoveDown));
    }

    #[test]
    fn test_keybindings_load_custom() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("keybindings.toml");

        let content = r#"
[bindings]
j = "move_down"
k = "move_up"
z = "quit"
"#;
        std::fs::write(&path, content).unwrap();

        let kb = Keybindings::load_from_path(path);

        assert_eq!(kb.get_action("j"), Some(&Action::MoveDown));
        assert_eq!(kb.get_action("k"), Some(&Action::MoveUp));
        assert_eq!(kb.get_action("z"), Some(&Action::Quit));
    }

    #[test]
    fn test_modifier_default() {
        let modifier = Modifier::default();
        assert_eq!(modifier, Modifier::None);
    }

    #[test]
    fn test_find_conflict_exists() {
        let kb = Keybindings::default();

        // "j" is bound to MoveDown by default
        assert_eq!(kb.find_conflict("j"), Some(&Action::MoveDown));
    }

    #[test]
    fn test_find_conflict_none() {
        let kb = Keybindings::default();

        // "z" is not bound by default
        assert_eq!(kb.find_conflict("z"), None);
    }

    #[test]
    fn test_set_binding_checked_no_conflict() {
        let mut kb = Keybindings::default();

        // "z" is not bound, so no conflict
        let previous = kb.set_binding_checked("z".to_string(), Action::Quit);
        assert!(previous.is_none());
        assert_eq!(kb.get_action("z"), Some(&Action::Quit));
    }

    #[test]
    fn test_set_binding_checked_with_conflict() {
        let mut kb = Keybindings::default();

        // "j" is already bound to MoveDown
        let previous = kb.set_binding_checked("j".to_string(), Action::Search);
        assert_eq!(previous, Some(Action::MoveDown));
        assert_eq!(kb.get_action("j"), Some(&Action::Search));
    }

    #[test]
    fn test_set_binding_checked_removes_old_action_binding() {
        let mut kb = Keybindings::default();

        // "j" is bound to MoveDown, now bind "z" to MoveDown
        let previous = kb.set_binding_checked("z".to_string(), Action::MoveDown);
        assert!(previous.is_none()); // "z" wasn't bound before

        // "j" should no longer be bound to MoveDown (action can only have one key)
        assert_eq!(kb.get_action("j"), None);
        assert_eq!(kb.get_action("z"), Some(&Action::MoveDown));
    }

    #[test]
    fn test_swap_bindings() {
        let mut kb = Keybindings::default();

        // "j" = MoveDown, "k" = MoveUp
        assert_eq!(kb.get_action("j"), Some(&Action::MoveDown));
        assert_eq!(kb.get_action("k"), Some(&Action::MoveUp));

        kb.swap_bindings("j", "k");

        // After swap: "j" = MoveUp, "k" = MoveDown
        assert_eq!(kb.get_action("j"), Some(&Action::MoveUp));
        assert_eq!(kb.get_action("k"), Some(&Action::MoveDown));
    }

    #[test]
    fn test_remove_binding() {
        let mut kb = Keybindings::default();

        assert_eq!(kb.get_action("j"), Some(&Action::MoveDown));

        let removed = kb.remove_binding("j");
        assert_eq!(removed, Some(Action::MoveDown));
        assert_eq!(kb.get_action("j"), None);
    }

    #[test]
    fn test_remove_binding_nonexistent() {
        let mut kb = Keybindings::default();

        let removed = kb.remove_binding("nonexistent");
        assert!(removed.is_none());
    }
}
