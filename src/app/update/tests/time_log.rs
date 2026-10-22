//! Time log editor tests.

use chrono::{DateTime, Duration, Local, NaiveDate, TimeZone, Utc};

use crate::app::{update::update, Message, Model, SystemMessage, UiMessage};
use crate::domain::{today, LocalDate, Task, TimeEntry};
use crate::ui::TimeLogMode;

fn create_model_with_time_entries() -> Model {
    let mut model = Model::new();

    // Create a task
    let task = Task::new("Test task");
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.selected_index = 0;

    // Two stopped hour-long entries today, 09:00 and 11:00 local. Fixed clock
    // times keep both on the same local day whatever time the tests run, so
    // the list order (chronological within the day) is 09:00 then 11:00.
    for h in [9, 11] {
        let started_at = local_dt(today(), h, 0);
        let mut e = TimeEntry::start(task_id);
        e.started_at = started_at;
        e.ended_at = Some(started_at + Duration::hours(1));
        e.duration_minutes = Some(60);
        model.time_entries.insert(e.id, e);
    }

    // Refresh after all data is inserted so caches are populated
    model.refresh_visible_tasks();

    model
}

#[test]
fn test_show_time_log() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = false;

    update(&mut model, Message::Ui(UiMessage::ShowTimeLog));

    assert!(model.time_log.visible);
    assert_eq!(model.time_log.selected, 0);
    assert_eq!(model.time_log.mode, TimeLogMode::Browse);
}

#[test]
fn test_show_time_log_no_task_selected() {
    let mut model = Model::new();
    model.time_log.visible = false;

    update(&mut model, Message::Ui(UiMessage::ShowTimeLog));

    // Should not show if no task selected
    assert!(!model.time_log.visible);
}

#[test]
fn test_hide_time_log() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.mode = TimeLogMode::EditStart;
    model.time_log.buffer = "test".to_string();

    update(&mut model, Message::Ui(UiMessage::HideTimeLog));

    assert!(!model.time_log.visible);
    assert_eq!(model.time_log.mode, TimeLogMode::Browse);
    assert!(model.time_log.buffer.is_empty());
}

#[test]
fn test_time_log_up() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.selected = 1;

    update(&mut model, Message::Ui(UiMessage::TimeLogUp));

    assert_eq!(model.time_log.selected, 0);
}

#[test]
fn test_time_log_up_at_zero() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.selected = 0;

    update(&mut model, Message::Ui(UiMessage::TimeLogUp));

    assert_eq!(model.time_log.selected, 0);
}

#[test]
fn test_time_log_down() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.selected = 0;

    update(&mut model, Message::Ui(UiMessage::TimeLogDown));

    assert_eq!(model.time_log.selected, 1);
}

#[test]
fn test_time_log_down_at_end() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.selected = 1;

    update(&mut model, Message::Ui(UiMessage::TimeLogDown));

    // Should stay at end
    assert_eq!(model.time_log.selected, 1);
}

#[test]
fn test_time_log_edit_start() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.selected = 0;

    update(&mut model, Message::Ui(UiMessage::TimeLogEditStart));

    assert_eq!(model.time_log.mode, TimeLogMode::EditStart);
    assert!(!model.time_log.buffer.is_empty());
}

#[test]
fn test_time_log_edit_end() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.selected = 0;

    update(&mut model, Message::Ui(UiMessage::TimeLogEditEnd));

    assert_eq!(model.time_log.mode, TimeLogMode::EditEnd);
}

#[test]
fn test_time_log_confirm_delete() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.mode = TimeLogMode::Browse;

    update(&mut model, Message::Ui(UiMessage::TimeLogConfirmDelete));

    assert_eq!(model.time_log.mode, TimeLogMode::ConfirmDelete);
}

#[test]
fn test_time_log_cancel() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.mode = TimeLogMode::EditStart;
    model.time_log.buffer = "12:30".to_string();

    update(&mut model, Message::Ui(UiMessage::TimeLogCancel));

    assert_eq!(model.time_log.mode, TimeLogMode::Browse);
    assert!(model.time_log.buffer.is_empty());
}

#[test]
fn test_time_log_add_entry_opens_prompt_prefilled_with_today() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    let initial_count = model.time_entries.len();

    update(&mut model, Message::Ui(UiMessage::TimeLogAddEntry));

    assert_eq!(model.time_log.mode, TimeLogMode::Add);
    assert_eq!(model.time_log.buffer, format!("{} ", today()));
    assert_eq!(model.time_entries.len(), initial_count, "nothing added yet");
}

#[test]
fn test_time_log_add_entry_without_task_does_nothing() {
    let mut model = Model::new();
    model.time_log.visible = true;
    update(&mut model, Message::Ui(UiMessage::TimeLogAddEntry));
    assert_eq!(model.time_log.mode, TimeLogMode::Browse);
}

#[test]
fn test_time_log_delete() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.mode = TimeLogMode::ConfirmDelete;
    model.time_log.selected = 0;
    let initial_count = model.time_entries.len();

    update(&mut model, Message::Ui(UiMessage::TimeLogDelete));

    assert_eq!(model.time_entries.len(), initial_count - 1);
    assert_eq!(model.time_log.mode, TimeLogMode::Browse);
}

#[test]
fn test_time_log_submit_edit_start() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.mode = TimeLogMode::EditStart;
    model.time_log.selected = 0;
    model.time_log.buffer = "10:30".to_string();

    update(&mut model, Message::Ui(UiMessage::TimeLogSubmit));

    assert_eq!(model.time_log.mode, TimeLogMode::Browse);
    assert!(model.time_log.buffer.is_empty());
    assert!(model.alerts.status_message.is_some());
}

#[test]
fn test_time_log_submit_edit_end() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.mode = TimeLogMode::EditEnd;
    model.time_log.selected = 0;
    model.time_log.buffer = "11:30".to_string();

    update(&mut model, Message::Ui(UiMessage::TimeLogSubmit));

    assert_eq!(model.time_log.mode, TimeLogMode::Browse);
    assert!(model.time_log.buffer.is_empty());
}

#[test]
fn test_time_log_submit_invalid_format() {
    let mut model = create_model_with_time_entries();
    model.time_log.visible = true;
    model.time_log.mode = TimeLogMode::EditStart;
    model.time_log.selected = 0;
    model.time_log.buffer = "invalid".to_string();

    update(&mut model, Message::Ui(UiMessage::TimeLogSubmit));

    // Should show error message
    assert!(model.alerts.status_message.is_some());
    assert!(model
        .alerts
        .status_message
        .as_ref()
        .unwrap()
        .contains("Invalid"));
}

#[test]
fn add_entry_updates_task_total_and_cache() {
    let mut model = create_model_with_time_entries();
    let task_id = model.selected_task_id().unwrap();
    model.time_log.visible = true;
    assert_eq!(
        model.tasks[&task_id].actual_minutes, 0,
        "fixture never stopped a timer"
    );

    update(&mut model, Message::Ui(UiMessage::TimeLogAddEntry));
    model.time_log.buffer = "09:00-09:30".to_string();
    update(&mut model, Message::Ui(UiMessage::TimeLogSubmit));

    assert_eq!(
        model.tasks[&task_id].actual_minutes, 150,
        "60 + 60 + the new 30"
    );
    assert_eq!(model.task_cache.time_sums.get(&task_id), Some(&150));
    assert_eq!(model.time_entries_for_task(&task_id).len(), 3);
}

#[test]
fn delete_entry_updates_task_total() {
    let mut model = create_model_with_time_entries();
    let task_id = model.selected_task_id().unwrap();
    model.recompute_task_time(&task_id);
    assert_eq!(model.tasks[&task_id].actual_minutes, 120);
    model.time_log.visible = true;
    model.time_log.mode = TimeLogMode::ConfirmDelete;
    model.time_log.selected = 0;

    update(&mut model, Message::Ui(UiMessage::TimeLogDelete));

    assert_eq!(model.tasks[&task_id].actual_minutes, 60);
    assert_eq!(model.time_entries_for_task(&task_id).len(), 1);
}

#[test]
fn edit_start_is_interpreted_as_local_clock_time() {
    let mut model = create_model_with_time_entries();
    let task_id = model.selected_task_id().unwrap();
    model.time_log.visible = true;
    model.time_log.mode = TimeLogMode::EditStart;
    model.time_log.selected = 0;
    model.time_log.buffer = "08:15".to_string();
    // Entries are listed chronologically within a day, so the edited entry
    // may move after the edit; follow it by id rather than by position.
    let edited_id = model.time_entries_for_task(&task_id)[0].id;

    update(&mut model, Message::Ui(UiMessage::TimeLogSubmit));

    let entry = &model.time_entries[&edited_id];
    let local = entry.started_at.with_timezone(&chrono::Local);
    assert_eq!(local.format("%H:%M").to_string(), "08:15");
    assert_eq!(
        model.tasks[&task_id].actual_minutes,
        model.total_time_for_task(&task_id)
    );
}

#[test]
fn edit_end_before_start_rolls_over_to_next_day() {
    let mut model = Model::new();
    let task = Task::new("Night shift");
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.selected_index = 0;

    // Start at 23:00 local today, currently ending at 23:30.
    let local_start = chrono::Local::now()
        .date_naive()
        .and_hms_opt(23, 0, 0)
        .unwrap();
    let started_at = chrono::TimeZone::from_local_datetime(&chrono::Local, &local_start)
        .single()
        .unwrap()
        .with_timezone(&Utc);
    let mut entry = TimeEntry::start(task_id);
    entry.started_at = started_at;
    entry.ended_at = Some(started_at + Duration::minutes(30));
    entry.duration_minutes = Some(30);
    model.time_entries.insert(entry.id, entry);
    model.refresh_visible_tasks();

    model.time_log.visible = true;
    model.time_log.mode = TimeLogMode::EditEnd;
    model.time_log.selected = 0;
    model.time_log.buffer = "01:30".to_string();
    update(&mut model, Message::Ui(UiMessage::TimeLogSubmit));

    let entry = model.time_entries_for_task(&task_id)[0];
    assert_eq!(entry.duration_minutes, Some(150), "23:00 to 01:30 next day");
    assert!(entry.ended_at.unwrap() > entry.started_at);
    assert_eq!(model.tasks[&task_id].actual_minutes, 150);
}

// ============================================================================
// Deterministic fixtures: dates from `today()`, clock times via `Local`.
// ============================================================================

fn local_dt(date: NaiveDate, h: u32, m: u32) -> DateTime<Utc> {
    Local
        .from_local_datetime(&date.and_hms_opt(h, m, 0).unwrap())
        .single()
        .unwrap()
        .with_timezone(&Utc)
}

fn local_hm(dt: DateTime<Utc>) -> String {
    dt.with_timezone(&Local).format("%H:%M").to_string()
}

/// A model with one selected task and entries given as
/// `(days ago, start hour, start minute, minutes)`, all stopped.
fn model_with_entries(spec: &[(i64, u32, u32, u32)]) -> Model {
    let mut model = Model::new();
    let task = Task::new("Deterministic task");
    let task_id = task.id;
    model.tasks.insert(task_id, task);
    model.selected_index = 0;
    for &(days_ago, h, m, minutes) in spec {
        let started_at = local_dt(today() - Duration::days(days_ago), h, m);
        let mut e = TimeEntry::start(task_id);
        e.started_at = started_at;
        e.ended_at = Some(started_at + Duration::minutes(i64::from(minutes)));
        e.duration_minutes = Some(minutes);
        model.time_entries.insert(e.id, e);
    }
    model.refresh_visible_tasks();
    model.recompute_task_time(&task_id);
    model.time_log.visible = true;
    model
}

fn selected_task(model: &Model) -> crate::domain::TaskId {
    model.selected_task_id().unwrap()
}

fn selected_entry(model: &Model) -> TimeEntry {
    let task_id = selected_task(model);
    (*model.time_entries_for_task(&task_id)[model.time_log.selected]).clone()
}

fn add(model: &mut Model, line: &str) {
    update(model, Message::Ui(UiMessage::TimeLogAddEntry));
    model.time_log.buffer = line.to_string();
    update(model, Message::Ui(UiMessage::TimeLogSubmit));
}

fn status(model: &Model) -> &str {
    model.alerts.status_message.as_deref().unwrap_or("")
}

// ---------------------------------------------------------------------------
// Add syntax
// ---------------------------------------------------------------------------

#[test]
fn add_start_end_with_note_defaults_to_today() {
    let mut model = model_with_entries(&[(1, 9, 0, 60)]);
    let task_id = selected_task(&model);

    add(&mut model, "09:00-10:30 standup");

    assert_eq!(model.time_log.mode, TimeLogMode::Browse);
    assert!(model.time_log.buffer.is_empty());
    let entries = model.time_entries_for_task(&task_id);
    assert_eq!(entries.len(), 2);
    let new = entries[0];
    assert_eq!(new.started_at.local_date(), today());
    assert_eq!(local_hm(new.started_at), "09:00");
    assert_eq!(local_hm(new.ended_at.unwrap()), "10:30");
    assert_eq!(new.duration_minutes, Some(90));
    assert_eq!(new.description.as_deref(), Some("standup"));
    assert_eq!(model.tasks[&task_id].actual_minutes, 150);
    assert_eq!(model.task_cache.time_sums.get(&task_id), Some(&150));
    assert_eq!(model.time_log.selected, 0, "cursor lands on the new entry");
    assert!(
        status(&model).starts_with("Added 1h 30m on "),
        "{}",
        status(&model)
    );
}

#[test]
fn add_days_ago_with_plus_duration() {
    let mut model = model_with_entries(&[]);
    let task_id = selected_task(&model);

    add(&mut model, "-1 14:00 +45m review");

    let new = model.time_entries_for_task(&task_id)[0];
    assert_eq!(new.started_at.local_date(), today() - Duration::days(1));
    assert_eq!(local_hm(new.started_at), "14:00");
    assert_eq!(local_hm(new.ended_at.unwrap()), "14:45");
    assert_eq!(new.duration_minutes, Some(45));
    assert_eq!(new.description.as_deref(), Some("review"));
}

#[test]
fn add_explicit_date_without_note() {
    let mut model = model_with_entries(&[]);
    let task_id = selected_task(&model);

    add(&mut model, "2026-10-20 08:15-09:00");

    let new = model.time_entries_for_task(&task_id)[0];
    assert_eq!(
        new.started_at.local_date(),
        NaiveDate::from_ymd_opt(2026, 10, 20).unwrap()
    );
    assert_eq!(local_hm(new.started_at), "08:15");
    assert_eq!(new.duration_minutes, Some(45));
    assert_eq!(new.description, None);
}

#[test]
fn add_today_and_yesterday_keywords() {
    let mut model = model_with_entries(&[]);
    let task_id = selected_task(&model);

    add(&mut model, "yesterday 10:00 +1h");
    add(&mut model, "today 11:00 +2h deep work on parser");

    let entries = model.time_entries_for_task(&task_id);
    assert_eq!(entries[0].started_at.local_date(), today());
    assert_eq!(entries[0].duration_minutes, Some(120));
    assert_eq!(
        entries[0].description.as_deref(),
        Some("deep work on parser")
    );
    assert_eq!(
        entries[1].started_at.local_date(),
        today() - Duration::days(1)
    );
    assert_eq!(entries[1].duration_minutes, Some(60));
}

#[test]
fn add_accepts_decimal_hours_and_glued_duration() {
    let mut model = model_with_entries(&[]);
    let task_id = selected_task(&model);

    add(&mut model, "09:00+1.5h");

    assert_eq!(
        model.time_entries_for_task(&task_id)[0].duration_minutes,
        Some(90)
    );
}

#[test]
fn add_end_before_start_rolls_to_next_day() {
    let mut model = model_with_entries(&[]);
    let task_id = selected_task(&model);

    add(&mut model, "-1 23:00-01:00 night shift");

    let new = model.time_entries_for_task(&task_id)[0];
    assert_eq!(new.duration_minutes, Some(120));
    assert_eq!(new.ended_at.unwrap().local_date(), today());
    assert_eq!(new.started_at.local_date(), today() - Duration::days(1));
}

#[test]
fn add_errors_keep_prompt_open_and_explain() {
    let cases: [(&str, &str); 6] = [
        ("", "nothing to add"),
        ("banana", "bad start time"),
        ("09:00", "give an end time"),
        ("09:00 +0m", "longer than 0"),
        ("09:00-banana", "bad end time"),
        ("09:00 +soon", "bad duration"),
    ];
    for (line, expected) in cases {
        let mut model = model_with_entries(&[]);
        let task_id = selected_task(&model);
        add(&mut model, line);

        assert_eq!(
            model.time_log.mode,
            TimeLogMode::Add,
            "{line:?} stays in Add"
        );
        assert_eq!(
            model.time_log.buffer, line,
            "{line:?} buffer kept for fixing"
        );
        assert!(model.time_entries_for_task(&task_id).is_empty(), "{line:?}");
        assert!(
            status(&model).starts_with("Add: "),
            "{line:?}: {}",
            status(&model)
        );
        assert!(
            status(&model).contains(expected),
            "{line:?}: {}",
            status(&model)
        );
        assert!(
            status(&model).contains("[date] START[-END | +DURATION] [note]"),
            "{line:?}: syntax hint shown"
        );
    }
}

#[test]
fn add_can_be_undone() {
    let mut model = model_with_entries(&[(0, 8, 0, 30)]);
    let task_id = selected_task(&model);

    add(&mut model, "10:00 +20m");
    assert_eq!(model.tasks[&task_id].actual_minutes, 50);

    update(&mut model, Message::System(SystemMessage::Undo));
    assert_eq!(model.time_entries_for_task(&task_id).len(), 1);
    assert_eq!(model.tasks[&task_id].actual_minutes, 30);
}

// ---------------------------------------------------------------------------
// Selection and prompt input
// ---------------------------------------------------------------------------

#[test]
fn entries_list_days_newest_first_and_within_day_chronological() {
    // Inserted out of order: today 14:00, today 09:00, yesterday 10:00.
    let model = model_with_entries(&[(0, 14, 0, 30), (0, 9, 0, 30), (1, 10, 0, 30)]);
    let task_id = selected_task(&model);

    let entries = model.time_entries_for_task(&task_id);
    let listed: Vec<(NaiveDate, String)> = entries
        .iter()
        .map(|e| (e.started_at.local_date(), local_hm(e.started_at)))
        .collect();
    assert_eq!(
        listed,
        vec![
            (today(), "09:00".to_string()),
            (today(), "14:00".to_string()),
            (today() - Duration::days(1), "10:00".to_string()),
        ]
    );

    // The editor groups the same list without moving anything, so the
    // selection index means the same entry in both.
    let groups = crate::ui::group_by_day(&entries);
    let positions: Vec<Vec<usize>> = groups
        .iter()
        .map(|g| g.entries.iter().map(|(i, _)| *i).collect())
        .collect();
    assert_eq!(positions, vec![vec![0, 1], vec![2]]);
    assert_eq!(local_hm(selected_entry(&model).started_at), "09:00");
}

#[test]
fn selection_moves_over_entries_only_across_days() {
    // Three entries on three days (newest day first): three headers, three
    // entry rows.
    let mut model = model_with_entries(&[(0, 9, 0, 30), (1, 9, 0, 30), (2, 9, 0, 30)]);
    assert_eq!(model.time_log.selected, 0);
    assert_eq!(selected_entry(&model).started_at.local_date(), today());

    update(&mut model, Message::Ui(UiMessage::TimeLogDown));
    assert_eq!(model.time_log.selected, 1);
    assert_eq!(
        selected_entry(&model).started_at.local_date(),
        today() - Duration::days(1)
    );
    update(&mut model, Message::Ui(UiMessage::TimeLogDown));
    assert_eq!(model.time_log.selected, 2);
    update(&mut model, Message::Ui(UiMessage::TimeLogDown));
    assert_eq!(model.time_log.selected, 2, "stops at the last entry");
    update(&mut model, Message::Ui(UiMessage::TimeLogUp));
    update(&mut model, Message::Ui(UiMessage::TimeLogUp));
    assert_eq!(model.time_log.selected, 0);
}

#[test]
fn prompt_input_only_edits_buffer_in_text_modes() {
    let mut model = model_with_entries(&[(0, 9, 0, 30)]);

    update(&mut model, Message::Ui(UiMessage::TimeLogInputChar('x')));
    assert!(model.time_log.buffer.is_empty(), "Browse ignores typing");

    update(&mut model, Message::Ui(UiMessage::TimeLogEditNote));
    for c in "note".chars() {
        update(&mut model, Message::Ui(UiMessage::TimeLogInputChar(c)));
    }
    update(&mut model, Message::Ui(UiMessage::TimeLogInputBackspace));
    assert_eq!(model.time_log.buffer, "not");

    update(&mut model, Message::Ui(UiMessage::TimeLogCancel));
    assert_eq!(model.time_log.mode, TimeLogMode::Browse);
    assert!(model.time_log.buffer.is_empty());
}

// ---------------------------------------------------------------------------
// Note (n)
// ---------------------------------------------------------------------------

#[test]
fn edit_note_sets_description_and_records_undo() {
    let mut model = model_with_entries(&[(0, 9, 0, 30)]);
    let task_id = selected_task(&model);

    update(&mut model, Message::Ui(UiMessage::TimeLogEditNote));
    assert_eq!(model.time_log.mode, TimeLogMode::EditNote);
    assert!(
        model.time_log.buffer.is_empty(),
        "prefilled with the (empty) note"
    );
    model.time_log.buffer = "  wrote tests  ".to_string();
    update(&mut model, Message::Ui(UiMessage::TimeLogSubmit));

    let entry = selected_entry(&model);
    assert_eq!(entry.description.as_deref(), Some("wrote tests"));
    assert_eq!(entry.duration_minutes, Some(30), "duration untouched");
    assert_eq!(model.time_log.mode, TimeLogMode::Browse);
    assert_eq!(status(&model), "Note updated");
    assert_eq!(model.tasks[&task_id].actual_minutes, 30);

    // Reopening prefills the current note; clearing it removes the description
    update(&mut model, Message::Ui(UiMessage::TimeLogEditNote));
    assert_eq!(model.time_log.buffer, "wrote tests");
    model.time_log.buffer.clear();
    update(&mut model, Message::Ui(UiMessage::TimeLogSubmit));
    assert_eq!(selected_entry(&model).description, None);
    assert_eq!(status(&model), "Note cleared");

    update(&mut model, Message::System(SystemMessage::Undo));
    assert_eq!(
        selected_entry(&model).description.as_deref(),
        Some("wrote tests")
    );
}

// ---------------------------------------------------------------------------
// Date (t)
// ---------------------------------------------------------------------------

#[test]
fn edit_date_moves_entry_keeping_clock_times() {
    let mut model = model_with_entries(&[(0, 9, 30, 105)]);
    let task_id = selected_task(&model);

    update(&mut model, Message::Ui(UiMessage::TimeLogEditDate));
    assert_eq!(model.time_log.mode, TimeLogMode::EditDate);
    assert_eq!(
        model.time_log.buffer,
        today().format("%Y-%m-%d").to_string()
    );
    model.time_log.buffer = "-2".to_string();
    update(&mut model, Message::Ui(UiMessage::TimeLogSubmit));

    let entry = selected_entry(&model);
    assert_eq!(entry.started_at.local_date(), today() - Duration::days(2));
    assert_eq!(local_hm(entry.started_at), "09:30");
    assert_eq!(local_hm(entry.ended_at.unwrap()), "11:15");
    assert_eq!(entry.duration_minutes, Some(105));
    assert_eq!(model.tasks[&task_id].actual_minutes, 105);
    assert!(
        status(&model).starts_with("Moved entry to "),
        "{}",
        status(&model)
    );

    update(&mut model, Message::System(SystemMessage::Undo));
    assert_eq!(selected_entry(&model).started_at.local_date(), today());
}

#[test]
fn edit_date_reselects_moved_entry_after_reordering() {
    // Two entries today, listed 09:00 then 14:00; moving the first one back
    // a day puts it under yesterday's header, after today's 14:00 entry.
    let mut model = model_with_entries(&[(0, 9, 0, 30), (0, 14, 0, 30)]);
    assert_eq!(local_hm(selected_entry(&model).started_at), "09:00");

    update(&mut model, Message::Ui(UiMessage::TimeLogEditDate));
    model.time_log.buffer = "yesterday".to_string();
    update(&mut model, Message::Ui(UiMessage::TimeLogSubmit));

    assert_eq!(model.time_log.selected, 1, "cursor follows the moved entry");
    let entry = selected_entry(&model);
    assert_eq!(local_hm(entry.started_at), "09:00");
    assert_eq!(entry.started_at.local_date(), today() - Duration::days(1));
    assert_eq!(
        local_hm(model.time_entries_for_task(&selected_task(&model))[0].started_at),
        "14:00",
        "today's remaining entry now heads the list"
    );
}

#[test]
fn edit_date_rejects_bad_input_and_running_entry() {
    let mut model = model_with_entries(&[(0, 9, 0, 30)]);
    update(&mut model, Message::Ui(UiMessage::TimeLogEditDate));
    model.time_log.buffer = "someday".to_string();
    update(&mut model, Message::Ui(UiMessage::TimeLogSubmit));
    assert_eq!(
        model.time_log.mode,
        TimeLogMode::EditDate,
        "prompt stays open"
    );
    assert!(status(&model).contains("Invalid date"));
    assert_eq!(selected_entry(&model).started_at.local_date(), today());

    let mut model = model_with_entries(&[]);
    let task_id = selected_task(&model);
    model.start_time_tracking(task_id);
    model.time_log.selected = 0;
    update(&mut model, Message::Ui(UiMessage::TimeLogEditDate));
    assert_eq!(model.time_log.mode, TimeLogMode::Browse);
    assert!(status(&model).contains("running"));
}

// ---------------------------------------------------------------------------
// Nudge (+/-)
// ---------------------------------------------------------------------------

#[test]
fn nudge_adjusts_ended_at_by_five_minutes_each_way() {
    let mut model = model_with_entries(&[(0, 9, 0, 30)]);
    let task_id = selected_task(&model);

    update(&mut model, Message::Ui(UiMessage::TimeLogNudge(5)));
    let entry = selected_entry(&model);
    assert_eq!(entry.duration_minutes, Some(35));
    assert_eq!(local_hm(entry.started_at), "09:00", "start untouched");
    assert_eq!(local_hm(entry.ended_at.unwrap()), "09:35");
    assert_eq!(model.tasks[&task_id].actual_minutes, 35);
    assert_eq!(status(&model), "Duration 35m (+5m)");

    update(&mut model, Message::Ui(UiMessage::TimeLogNudge(-5)));
    update(&mut model, Message::Ui(UiMessage::TimeLogNudge(-5)));
    let entry = selected_entry(&model);
    assert_eq!(entry.duration_minutes, Some(25));
    assert_eq!(local_hm(entry.ended_at.unwrap()), "09:25");
    assert_eq!(model.tasks[&task_id].actual_minutes, 25);
    assert_eq!(status(&model), "Duration 25m (-5m)");
}

#[test]
fn nudge_never_drops_below_the_minimum() {
    let mut model = model_with_entries(&[(0, 9, 0, 7)]);
    let task_id = selected_task(&model);

    update(&mut model, Message::Ui(UiMessage::TimeLogNudge(-5)));
    assert_eq!(
        selected_entry(&model).duration_minutes,
        Some(5),
        "clamps to the minimum"
    );
    let undo_depth = model.undo_stack.len();

    update(&mut model, Message::Ui(UiMessage::TimeLogNudge(-5)));
    assert_eq!(selected_entry(&model).duration_minutes, Some(5));
    assert_eq!(
        model.undo_stack.len(),
        undo_depth,
        "refused nudge records nothing"
    );
    assert!(status(&model).contains("minimum"));
    assert_eq!(model.tasks[&task_id].actual_minutes, 5);
}

#[test]
fn nudge_is_refused_on_running_entry() {
    let mut model = model_with_entries(&[]);
    let task_id = selected_task(&model);
    model.start_time_tracking(task_id);
    model.time_log.selected = 0;
    let undo_depth = model.undo_stack.len();

    update(&mut model, Message::Ui(UiMessage::TimeLogNudge(5)));

    assert!(selected_entry(&model).is_running());
    assert_eq!(model.undo_stack.len(), undo_depth);
    assert!(status(&model).contains("running"));
}

#[test]
fn undo_of_a_nudge_restores_duration_and_task_total() {
    let mut model = model_with_entries(&[(0, 9, 0, 30)]);
    let task_id = selected_task(&model);

    update(&mut model, Message::Ui(UiMessage::TimeLogNudge(5)));
    assert_eq!(model.tasks[&task_id].actual_minutes, 35);

    update(&mut model, Message::System(SystemMessage::Undo));
    let entry = selected_entry(&model);
    assert_eq!(entry.duration_minutes, Some(30));
    assert_eq!(local_hm(entry.ended_at.unwrap()), "09:30");
    assert_eq!(model.tasks[&task_id].actual_minutes, 30);
    assert_eq!(model.task_cache.time_sums.get(&task_id), Some(&30));
}

#[test]
fn undo_of_start_edit_restores_entry() {
    let mut model = model_with_entries(&[(0, 9, 0, 60)]);
    let task_id = selected_task(&model);

    update(&mut model, Message::Ui(UiMessage::TimeLogEditStart));
    model.time_log.buffer = "09:30".to_string();
    update(&mut model, Message::Ui(UiMessage::TimeLogSubmit));
    assert_eq!(selected_entry(&model).duration_minutes, Some(30));
    assert_eq!(model.tasks[&task_id].actual_minutes, 30);

    update(&mut model, Message::System(SystemMessage::Undo));
    let entry = selected_entry(&model);
    assert_eq!(local_hm(entry.started_at), "09:00");
    assert_eq!(entry.duration_minutes, Some(60));
    assert_eq!(model.tasks[&task_id].actual_minutes, 60);
}
