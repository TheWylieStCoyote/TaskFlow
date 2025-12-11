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
