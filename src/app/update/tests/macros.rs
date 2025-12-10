//! Macro recording and playback tests.

use crate::app::{update::update, Message, Model, UiMessage};

#[test]
fn test_start_record_macro_first_press() {
    let mut model = Model::new();

    update(&mut model, Message::Ui(UiMessage::StartRecordMacro));

    // Should prompt for slot selection
    assert!(model.macro_state.pending_slot.is_some());
    assert!(model.alerts.status_message.is_some());
}

#[test]
fn test_stop_record_macro_cancels() {
    let mut model = Model::new();

    // Start recording
    update(&mut model, Message::Ui(UiMessage::StartRecordMacro));

    // Cancel by stopping without proper setup
    update(&mut model, Message::Ui(UiMessage::StopRecordMacro));

    // Recording should be cancelled
    assert!(!model.macro_state.is_recording());
}

#[test]
fn test_play_macro_empty_slot() {
    let mut model = Model::new();

    update(&mut model, Message::Ui(UiMessage::PlayMacro(5)));

    // Should show message about no macro
    assert!(model.alerts.status_message.is_some());
    assert!(model
        .alerts
        .status_message
        .as_ref()
        .unwrap()
        .contains("No macro"));
}

#[test]
fn test_start_record_while_recording() {
    let mut model = Model::new();

    // First press - prompt for slot
    update(&mut model, Message::Ui(UiMessage::StartRecordMacro));
    assert!(model.macro_state.pending_slot.is_some());

    // Simulate slot selection by starting recording on slot 0
    model.macro_state.start_recording(0);
    assert!(model.macro_state.is_recording());

    // Second press while recording
    update(&mut model, Message::Ui(UiMessage::StartRecordMacro));

    // Should enter slot selection mode
    assert!(model.alerts.status_message.is_some());
}
