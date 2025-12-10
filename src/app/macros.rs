//! Keyboard macro recording and playback.
//!
//! This module enables users to record sequences of actions and replay them.
//! Macros are useful for repetitive tasks like bulk editing or applying
//! the same changes to multiple items.
//!
//! # Usage
//!
//! 1. Start recording with `q` followed by a register key
//! 2. Perform the desired actions
//! 3. Stop recording with `q`
//! 4. Replay with `@` followed by the register key
//!
//! # Filtering
//!
//! Certain messages (like quit, undo, macro control) are filtered out
//! during recording to prevent unexpected behavior during playback.

use super::Message;

/// A recorded macro consisting of a sequence of messages
#[derive(Debug, Clone, Default)]
pub struct Macro {
    /// Name of the macro (for display)
    pub name: String,
    /// Sequence of recorded messages
    pub messages: Vec<Message>,
}

impl Macro {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            messages: Vec::new(),
        }
    }

    pub fn push(&mut self, message: Message) {
        // Don't record certain messages that shouldn't be replayed
        if Self::should_record(&message) {
            self.messages.push(message);
        }
    }

    /// Check if a message should be recorded
    const fn should_record(message: &Message) -> bool {
        use super::{SystemMessage, UiMessage};
        match message {
            // Don't record system messages or macro control messages
            Message::System(
                SystemMessage::Quit | SystemMessage::Tick | SystemMessage::Resize { .. },
            )
            | Message::Ui(
                UiMessage::StartRecordMacro | UiMessage::StopRecordMacro | UiMessage::PlayMacro(_),
            ) => false,
            // Record everything else
            _ => true,
        }
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.messages.len()
    }
}

/// Macro recording/playback state
#[derive(Debug, Clone, Default)]
pub struct MacroState {
    /// Currently recording macro (if any)
    pub recording: Option<Macro>,
    /// Stored macros (up to 10, slots 0-9)
    pub slots: [Option<Macro>; 10],
    /// Whether currently playing back a macro
    pub playing: bool,
    /// Pending slot for record/playback (waiting for digit input)
    pub pending_slot: Option<usize>,
}

impl MacroState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Start recording a macro to a slot
    pub fn start_recording(&mut self, slot: usize) -> bool {
        if slot >= 10 || self.playing {
            return false;
        }
        self.recording = Some(Macro::new(format!("Macro {slot}")));
        true
    }

    /// Stop recording and save to the slot
    pub fn stop_recording(&mut self, slot: usize) -> bool {
        if slot >= 10 {
            return false;
        }
        if let Some(macro_) = self.recording.take() {
            if !macro_.is_empty() {
                self.slots[slot] = Some(macro_);
                return true;
            }
        }
        false
    }

    /// Cancel recording without saving
    pub fn cancel_recording(&mut self) {
        self.recording = None;
    }

    /// Check if currently recording
    #[must_use]
    pub const fn is_recording(&self) -> bool {
        self.recording.is_some()
    }

    /// Record a message if currently recording
    pub fn record(&mut self, message: &Message) {
        if let Some(ref mut macro_) = self.recording {
            macro_.push(message.clone());
        }
    }

    /// Get messages for playback from a slot
    #[must_use]
    pub fn get_playback_messages(&self, slot: usize) -> Option<Vec<Message>> {
        if slot >= 10 {
            return None;
        }
        self.slots[slot].as_ref().map(|m| m.messages.clone())
    }

    /// Check if a slot has a macro
    #[must_use]
    pub const fn has_macro(&self, slot: usize) -> bool {
        slot < 10 && self.slots[slot].is_some()
    }

    /// Get macro info for display
    #[must_use]
    pub fn macro_info(&self, slot: usize) -> Option<(String, usize)> {
        if slot >= 10 {
            return None;
        }
        self.slots[slot].as_ref().map(|m| (m.name.clone(), m.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{NavigationMessage, SystemMessage, TaskMessage, UiMessage};

    #[test]
    fn test_macro_new() {
        let macro_ = Macro::new("Test");
        assert_eq!(macro_.name, "Test");
        assert!(macro_.is_empty());
        assert_eq!(macro_.len(), 0);
    }

    #[test]
    fn test_macro_push_recorded_message() {
        let mut macro_ = Macro::new("Test");
        macro_.push(Message::Navigation(NavigationMessage::Down));
        assert_eq!(macro_.len(), 1);
    }

    #[test]
    fn test_macro_ignores_quit() {
        let mut macro_ = Macro::new("Test");
        macro_.push(Message::System(SystemMessage::Quit));
        assert!(macro_.is_empty());
    }

    #[test]
    fn test_macro_ignores_tick() {
        let mut macro_ = Macro::new("Test");
        macro_.push(Message::System(SystemMessage::Tick));
        assert!(macro_.is_empty());
    }

    #[test]
    fn test_macro_ignores_resize() {
        let mut macro_ = Macro::new("Test");
        macro_.push(Message::System(SystemMessage::Resize {
            width: 80,
            height: 24,
        }));
        assert!(macro_.is_empty());
    }

    #[test]
    fn test_macro_state_new() {
        let state = MacroState::new();
        assert!(!state.is_recording());
        assert!(!state.playing);
    }

    #[test]
    fn test_start_recording() {
        let mut state = MacroState::new();
        assert!(state.start_recording(0));
        assert!(state.is_recording());
    }

    #[test]
    fn test_start_recording_invalid_slot() {
        let mut state = MacroState::new();
        assert!(!state.start_recording(10));
        assert!(!state.is_recording());
    }

    #[test]
    fn test_stop_recording_saves_macro() {
        let mut state = MacroState::new();
        state.start_recording(0);
        state.record(&Message::Navigation(NavigationMessage::Down));
        assert!(state.stop_recording(0));
        assert!(!state.is_recording());
        assert!(state.has_macro(0));
    }

    #[test]
    fn test_stop_recording_empty_macro_not_saved() {
        let mut state = MacroState::new();
        state.start_recording(0);
        assert!(!state.stop_recording(0));
        assert!(!state.has_macro(0));
    }

    #[test]
    fn test_cancel_recording() {
        let mut state = MacroState::new();
        state.start_recording(0);
        state.record(&Message::Navigation(NavigationMessage::Down));
        state.cancel_recording();
        assert!(!state.is_recording());
        assert!(!state.has_macro(0));
    }

    #[test]
    fn test_get_playback_messages() {
        let mut state = MacroState::new();
        state.start_recording(5);
        state.record(&Message::Navigation(NavigationMessage::Down));
        state.record(&Message::Navigation(NavigationMessage::Up));
        state.stop_recording(5);

        let messages = state.get_playback_messages(5);
        assert!(messages.is_some());
        assert_eq!(messages.unwrap().len(), 2);
    }

    #[test]
    fn test_macro_info() {
        let mut state = MacroState::new();
        state.start_recording(3);
        state.record(&Message::Navigation(NavigationMessage::Down));
        state.record(&Message::Task(TaskMessage::ToggleComplete));
        state.stop_recording(3);

        let info = state.macro_info(3);
        assert!(info.is_some());
        let (name, len) = info.unwrap();
        assert_eq!(name, "Macro 3");
        assert_eq!(len, 2);
    }

    #[test]
    fn test_macro_ignores_macro_control_messages() {
        let mut macro_ = Macro::new("Test");
        macro_.push(Message::Ui(UiMessage::StartRecordMacro));
        macro_.push(Message::Ui(UiMessage::StopRecordMacro));
        macro_.push(Message::Ui(UiMessage::PlayMacro(0)));
        assert!(macro_.is_empty());
    }
}
