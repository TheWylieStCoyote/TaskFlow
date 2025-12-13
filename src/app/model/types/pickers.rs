//! Picker state types for modal dialogs.

/// State for template picker modal.
#[derive(Debug, Clone, Default)]
pub struct TemplatePickerState {
    /// Whether template picker is visible
    pub visible: bool,
    /// Index of selected template in picker
    pub selected: usize,
}

/// State for saved filter picker modal.
#[derive(Debug, Clone, Default)]
pub struct SavedFilterPickerState {
    /// Whether saved filter picker is visible
    pub visible: bool,
    /// Selected index in saved filter picker
    pub selected: usize,
}

/// State for keybindings editor modal.
#[derive(Debug, Clone, Default)]
pub struct KeybindingsEditorState {
    /// Whether keybindings editor is visible
    pub visible: bool,
    /// Selected keybinding index in editor
    pub selected: usize,
    /// Whether currently capturing a new key
    pub capturing: bool,
}

/// State for command palette popup.
///
/// The command palette provides a searchable list of all available
/// actions, similar to VS Code's Ctrl+P command palette.
#[derive(Debug, Clone, Default)]
pub struct CommandPaletteState {
    /// Whether the command palette is visible
    pub visible: bool,
    /// Current search query
    pub query: String,
    /// Cursor position in the query string
    pub cursor: usize,
    /// Index of selected command in filtered list
    pub selected: usize,
}
