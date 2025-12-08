//! Navigation tests.

use crate::app::{update::update, Message, NavigationMessage, ViewId};

use super::create_test_model_with_tasks;

#[test]
fn test_navigation_up() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 2;

    update(&mut model, Message::Navigation(NavigationMessage::Up));

    assert_eq!(model.selected_index, 1);
}

#[test]
fn test_navigation_up_stops_at_zero() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 0;

    update(&mut model, Message::Navigation(NavigationMessage::Up));

    assert_eq!(model.selected_index, 0);
}

#[test]
fn test_navigation_down() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 2;

    update(&mut model, Message::Navigation(NavigationMessage::Down));

    assert_eq!(model.selected_index, 3);
}

#[test]
fn test_navigation_down_stops_at_max() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 4;

    update(&mut model, Message::Navigation(NavigationMessage::Down));

    assert_eq!(model.selected_index, 4);
}

#[test]
fn test_navigation_first() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 3;

    update(&mut model, Message::Navigation(NavigationMessage::First));

    assert_eq!(model.selected_index, 0);
}

#[test]
fn test_navigation_last() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 0;

    update(&mut model, Message::Navigation(NavigationMessage::Last));

    assert_eq!(model.selected_index, 4);
}

#[test]
fn test_navigation_page_up() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 4;

    update(&mut model, Message::Navigation(NavigationMessage::PageUp));

    assert_eq!(model.selected_index, 0); // saturating_sub from 4 - 10
}

#[test]
fn test_navigation_page_down() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 0;

    update(&mut model, Message::Navigation(NavigationMessage::PageDown));

    assert_eq!(model.selected_index, 4); // capped at max
}

#[test]
fn test_navigation_select() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 0;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::Select(3)),
    );

    assert_eq!(model.selected_index, 3);
}

#[test]
fn test_navigation_select_invalid_ignored() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 2;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::Select(100)),
    );

    assert_eq!(model.selected_index, 2); // unchanged
}

#[test]
fn test_navigation_go_to_view() {
    let mut model = create_test_model_with_tasks();
    model.selected_index = 3;
    model.current_view = ViewId::TaskList;

    update(
        &mut model,
        Message::Navigation(NavigationMessage::GoToView(ViewId::Today)),
    );

    assert_eq!(model.current_view, ViewId::Today);
    assert_eq!(model.selected_index, 0); // reset to 0
}
