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

// ============================================================================
// Goal-KeyResult Integration Tests
// ============================================================================

#[test]
fn test_key_results_for_goal_linkage() {
    let (mut model, goal_id) = create_test_model_with_goal();

    // Create 3 KRs for the goal
    for name in ["KR1", "KR2", "KR3"] {
        update(
            &mut model,
            Message::Goal(GoalMessage::CreateKeyResult {
                goal_id,
                name: name.to_string(),
            }),
        );
    }

    // Verify linkage
    let linked = model.key_results_for_goal(goal_id);
    assert_eq!(linked.len(), 3);

    // Verify all are linked to the correct goal
    for kr in &linked {
        assert_eq!(kr.goal_id, goal_id);
    }
}

#[test]
fn test_key_results_for_goal_sorted_by_name() {
    let (mut model, goal_id) = create_test_model_with_goal();

    // Create KRs in non-alphabetical order
    for name in ["Zulu", "Alpha", "Mike"] {
        update(
            &mut model,
            Message::Goal(GoalMessage::CreateKeyResult {
                goal_id,
                name: name.to_string(),
            }),
        );
    }

    let linked = model.key_results_for_goal(goal_id);
    let names: Vec<_> = linked.iter().map(|kr| kr.name.as_str()).collect();
    assert_eq!(names, vec!["Alpha", "Mike", "Zulu"]);
}

#[test]
fn test_goal_progress_with_zero_target_key_result() {
    let (mut model, goal_id) = create_test_model_with_goal();

    // Create a KR with zero target
    update(
        &mut model,
        Message::Goal(GoalMessage::CreateKeyResult {
            goal_id,
            name: "Zero target".to_string(),
        }),
    );

    // Set target to 0 and current value to something
    let kr_id = *model.key_results.keys().next().unwrap();
    update(
        &mut model,
        Message::Goal(GoalMessage::SetKeyResultTarget {
            id: kr_id,
            target: 0.0,
            unit: None,
        }),
    );
    update(
        &mut model,
        Message::Goal(GoalMessage::SetKeyResultValue {
            id: kr_id,
            value: 50.0,
        }),
    );

    // Should not panic, should return 0
    let progress = model.goal_progress(goal_id);
    assert_eq!(progress, 0);
}

#[test]
fn test_goal_progress_with_no_key_results() {
    let (model, goal_id) = create_test_model_with_goal();

    // Goal with no KRs should have 0 progress
    let progress = model.goal_progress(goal_id);
    assert_eq!(progress, 0);
}

#[test]
fn test_goal_progress_aggregate_multiple_key_results() {
    let (mut model, goal_id) = create_test_model_with_goal();

    // Create 3 KRs with different progress levels
    // KR1: 0% (0/100), KR2: 50% (50/100), KR3: 100% (100/100)
    let kr_data = [
        ("KR1", 100.0, 0.0),   // 0%
        ("KR2", 100.0, 50.0),  // 50%
        ("KR3", 100.0, 100.0), // 100%
    ];

    for (name, _target, _current) in kr_data {
        update(
            &mut model,
            Message::Goal(GoalMessage::CreateKeyResult {
                goal_id,
                name: name.to_string(),
            }),
        );
    }

    // Set targets and values
    let kr_ids: Vec<_> = model.key_results.keys().copied().collect();
    for (i, kr_id) in kr_ids.iter().enumerate() {
        let (_, target, current) = kr_data[i];
        update(
            &mut model,
            Message::Goal(GoalMessage::SetKeyResultTarget {
                id: *kr_id,
                target,
                unit: None,
            }),
        );
        update(
            &mut model,
            Message::Goal(GoalMessage::SetKeyResultValue {
                id: *kr_id,
                value: current,
            }),
        );
    }

    // Average: (0 + 50 + 100) / 3 = 50%
    let progress = model.goal_progress(goal_id);
    assert_eq!(progress, 50);
}

#[test]
fn test_goal_progress_manual_overrides_calculated() {
    let (mut model, goal_id) = create_test_model_with_goal();

    // Create KR at 50%
    update(
        &mut model,
        Message::Goal(GoalMessage::CreateKeyResult {
            goal_id,
            name: "Test KR".to_string(),
        }),
    );

    let kr_id = *model.key_results.keys().next().unwrap();
    update(
        &mut model,
        Message::Goal(GoalMessage::SetKeyResultTarget {
            id: kr_id,
            target: 100.0,
            unit: None,
        }),
    );
    update(
        &mut model,
        Message::Goal(GoalMessage::SetKeyResultValue {
            id: kr_id,
            value: 50.0,
        }),
    );

    // Without manual, progress is 50%
    assert_eq!(model.goal_progress(goal_id), 50);

    // Set manual progress to 75%
    update(
        &mut model,
        Message::Goal(GoalMessage::SetManualProgress {
            id: goal_id,
            progress: Some(75),
        }),
    );

    // Manual overrides calculated
    assert_eq!(model.goal_progress(goal_id), 75);

    // Clear manual progress
    update(
        &mut model,
        Message::Goal(GoalMessage::SetManualProgress {
            id: goal_id,
            progress: None,
        }),
    );

    // Back to calculated
    assert_eq!(model.goal_progress(goal_id), 50);
}

#[test]
fn test_key_result_progress_exceeds_target() {
    let (mut model, goal_id) = create_test_model_with_goal();

    // Create KR
    update(
        &mut model,
        Message::Goal(GoalMessage::CreateKeyResult {
            goal_id,
            name: "Overachiever".to_string(),
        }),
    );

    let kr_id = *model.key_results.keys().next().unwrap();
    update(
        &mut model,
        Message::Goal(GoalMessage::SetKeyResultTarget {
            id: kr_id,
            target: 100.0,
            unit: None,
        }),
    );
    // Set value to 150% of target
    update(
        &mut model,
        Message::Goal(GoalMessage::SetKeyResultValue {
            id: kr_id,
            value: 150.0,
        }),
    );

    let kr = model.key_results.get(&kr_id).unwrap();
    // Progress should be capped at 100%
    assert_eq!(kr.progress_percent(), 100);
}

#[test]
fn test_key_results_for_nonexistent_goal() {
    let model = Model::new();
    let fake_id = GoalId::new();

    // Should return empty vec, not panic
    let linked = model.key_results_for_goal(fake_id);
    assert!(linked.is_empty());
}

#[test]
fn test_goal_progress_for_nonexistent_goal() {
    let model = Model::new();
    let fake_id = GoalId::new();

    // Should return 0, not panic
    let progress = model.goal_progress(fake_id);
    assert_eq!(progress, 0);
}
