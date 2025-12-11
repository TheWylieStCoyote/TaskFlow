//! Goal and Key Result CRUD and navigation tests.

use crate::app::{update::update, GoalMessage, Message, Model};
use crate::domain::{Goal, GoalId, GoalStatus, Quarter};

fn create_test_model_with_goal() -> (Model, GoalId) {
    let mut model = Model::new();
    let goal = Goal::new("Test Goal");
    let goal_id = goal.id;
    model.goals.insert(goal.id, goal);
    model.refresh_visible_goals();
    (model, goal_id)
}

#[test]
fn test_goal_create() {
    let mut model = Model::new();
    assert!(model.goals.is_empty());

    update(
        &mut model,
        Message::Goal(GoalMessage::Create("Q1 Objective".to_string())),
    );

    assert_eq!(model.goals.len(), 1);
    let goal = model.goals.values().next().unwrap();
    assert_eq!(goal.name, "Q1 Objective");
    assert_eq!(goal.status, GoalStatus::Active);
}

#[test]
fn test_goal_update_name() {
    let (mut model, goal_id) = create_test_model_with_goal();

    update(
        &mut model,
        Message::Goal(GoalMessage::UpdateName {
            id: goal_id,
            name: "Renamed Goal".to_string(),
        }),
    );

    assert_eq!(model.goals.get(&goal_id).unwrap().name, "Renamed Goal");
}

#[test]
fn test_goal_set_status() {
    let (mut model, goal_id) = create_test_model_with_goal();
    assert!(model.goals.get(&goal_id).unwrap().is_active());

    update(
        &mut model,
        Message::Goal(GoalMessage::SetStatus {
            id: goal_id,
            status: GoalStatus::OnHold,
        }),
    );

    assert_eq!(
        model.goals.get(&goal_id).unwrap().status,
        GoalStatus::OnHold
    );
}

#[test]
fn test_goal_set_quarter() {
    let (mut model, goal_id) = create_test_model_with_goal();

    update(
        &mut model,
        Message::Goal(GoalMessage::SetQuarter {
            id: goal_id,
            quarter: Some((2025, Quarter::Q1)),
        }),
    );

    assert_eq!(
        model.goals.get(&goal_id).unwrap().quarter,
        Some((2025, Quarter::Q1))
    );
}

#[test]
fn test_goal_manual_progress_capped() {
    let (mut model, goal_id) = create_test_model_with_goal();

    update(
        &mut model,
        Message::Goal(GoalMessage::SetManualProgress {
            id: goal_id,
            progress: Some(150),
        }),
    );

    assert_eq!(
        model.goals.get(&goal_id).unwrap().manual_progress,
        Some(100)
    );
}

#[test]
fn test_goal_delete() {
    let (mut model, goal_id) = create_test_model_with_goal();
    assert!(model.goals.contains_key(&goal_id));

    update(&mut model, Message::Goal(GoalMessage::Delete(goal_id)));

    assert!(!model.goals.contains_key(&goal_id));
}

#[test]
fn test_key_result_create() {
    let (mut model, goal_id) = create_test_model_with_goal();
    assert!(model.key_results.is_empty());

    update(
        &mut model,
        Message::Goal(GoalMessage::CreateKeyResult {
            goal_id,
            name: "Ship MVP".to_string(),
        }),
    );

    assert_eq!(model.key_results.len(), 1);
    let kr = model.key_results.values().next().unwrap();
    assert_eq!(kr.name, "Ship MVP");
    assert_eq!(kr.goal_id, goal_id);
}

#[test]
fn test_key_result_set_target_and_value() {
    let (mut model, goal_id) = create_test_model_with_goal();

    update(
        &mut model,
        Message::Goal(GoalMessage::CreateKeyResult {
            goal_id,
            name: "Acquire users".to_string(),
        }),
    );

    let kr_id = *model.key_results.keys().next().unwrap();

    update(
        &mut model,
        Message::Goal(GoalMessage::SetKeyResultTarget {
            id: kr_id,
            target: 100.0,
            unit: Some("users".to_string()),
        }),
    );

    update(
        &mut model,
        Message::Goal(GoalMessage::SetKeyResultValue {
            id: kr_id,
            value: 45.0,
        }),
    );

    let kr = model.key_results.get(&kr_id).unwrap();
    assert!((kr.target_value - 100.0).abs() < f64::EPSILON);
    assert!((kr.current_value - 45.0).abs() < f64::EPSILON);
    assert_eq!(kr.unit, Some("users".to_string()));
}

#[test]
fn test_delete_goal_cascades_to_key_results() {
    let (mut model, goal_id) = create_test_model_with_goal();

    update(
        &mut model,
        Message::Goal(GoalMessage::CreateKeyResult {
            goal_id,
            name: "KR1".to_string(),
        }),
    );
    update(
        &mut model,
        Message::Goal(GoalMessage::CreateKeyResult {
            goal_id,
            name: "KR2".to_string(),
        }),
    );

    assert_eq!(model.key_results.len(), 2);

    update(&mut model, Message::Goal(GoalMessage::Delete(goal_id)));

    assert!(model.goals.is_empty());
    assert!(model.key_results.is_empty()); // Cascade delete
}

#[test]
fn test_goal_navigation() {
    let mut model = Model::new();

    // Create 3 goals
    for i in 0..3 {
        update(
            &mut model,
            Message::Goal(GoalMessage::Create(format!("Goal {i}"))),
        );
    }

    assert_eq!(model.goal_view.selected_goal, 0);

    update(&mut model, Message::Goal(GoalMessage::NavigateDown));
    assert_eq!(model.goal_view.selected_goal, 1);

    update(&mut model, Message::Goal(GoalMessage::NavigateDown));
    assert_eq!(model.goal_view.selected_goal, 2);

    update(&mut model, Message::Goal(GoalMessage::NavigateUp));
    assert_eq!(model.goal_view.selected_goal, 1);
}

#[test]
fn test_goal_expand_collapse() {
    let (mut model, goal_id) = create_test_model_with_goal();

    assert!(model.goal_view.expanded_goal.is_none());

    update(&mut model, Message::Goal(GoalMessage::ExpandGoal(goal_id)));
    assert_eq!(model.goal_view.expanded_goal, Some(goal_id));

    update(&mut model, Message::Goal(GoalMessage::CollapseGoal));
    assert!(model.goal_view.expanded_goal.is_none());
}

#[test]
fn test_goal_toggle_archived() {
    let mut model = Model::new();
    assert!(!model.goal_view.show_archived);

    update(&mut model, Message::Goal(GoalMessage::ToggleArchived));
    assert!(model.goal_view.show_archived);

    update(&mut model, Message::Goal(GoalMessage::ToggleArchived));
    assert!(!model.goal_view.show_archived);
}

#[test]
fn test_goal_operations_on_nonexistent() {
    let mut model = Model::new();
    let fake_id = GoalId::new();

    // Should not panic
    update(
        &mut model,
        Message::Goal(GoalMessage::UpdateName {
            id: fake_id,
            name: "Test".to_string(),
        }),
    );
    update(&mut model, Message::Goal(GoalMessage::Delete(fake_id)));
}

#[test]
fn test_navigate_into_expands_goal() {
    let (mut model, goal_id) = create_test_model_with_goal();

    // Initially no goal is expanded
    assert!(model.goal_view.expanded_goal.is_none());

    // NavigateInto should expand the selected goal
    update(&mut model, Message::Goal(GoalMessage::NavigateInto));
    assert_eq!(model.goal_view.expanded_goal, Some(goal_id));
}

#[test]
fn test_navigate_back_collapses_goal() {
    let (mut model, goal_id) = create_test_model_with_goal();

    // Expand the goal first
    model.goal_view.expanded_goal = Some(goal_id);

    // NavigateBack should collapse
    update(&mut model, Message::Goal(GoalMessage::NavigateBack));
    assert!(model.goal_view.expanded_goal.is_none());
}
