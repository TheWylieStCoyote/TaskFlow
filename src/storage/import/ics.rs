//! ICS (iCalendar) import functionality.
//!
//! Parses both VTODO (tasks) and VEVENT (calendar events) components
//! from iCalendar files.

use std::collections::HashMap;
use std::io::BufRead;

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use uuid::Uuid;

use crate::domain::{CalendarEvent, CalendarEventStatus, Priority, Task, TaskId, TaskStatus};
use crate::storage::StorageResult;

use super::types::{ImportError, ImportOptions, ImportResult};

/// Tracks which component type is currently being parsed
#[derive(Clone, Copy, PartialEq, Eq)]
enum IcsComponent {
    None,
    Vtodo,
    Vevent,
}

/// Import tasks and events from ICS (iCalendar) format
///
/// Parses both VTODO (tasks) and VEVENT (calendar events) components
/// from an iCalendar file.
///
/// # Errors
///
/// Returns a [`StorageError`](crate::storage::StorageError) if the file cannot be read or parsed.
pub fn import_from_ics<R: BufRead>(
    reader: R,
    options: &ImportOptions,
) -> StorageResult<ImportResult> {
    let mut result = ImportResult::default();
    let mut current_component = IcsComponent::None;
    let mut current_props: HashMap<String, String> = HashMap::new();
    let mut line_num = 0;
    let mut unfolded_line = String::new();

    for line_result in reader.lines() {
        line_num += 1;

        let line = match line_result {
            Ok(l) => l,
            Err(e) => {
                result.errors.push(ImportError {
                    line: line_num,
                    message: format!("Failed to read line: {e}"),
                    content: None,
                });
                continue;
            }
        };

        // Handle line folding (continuation lines start with space or tab)
        if line.starts_with(' ') || line.starts_with('\t') {
            unfolded_line.push_str(line.trim_start());
            continue;
        }

        // Process the previous unfolded line
        if !unfolded_line.is_empty() {
            process_ics_line(
                &unfolded_line,
                &mut current_component,
                &mut current_props,
                &mut result,
                line_num,
                options,
            );
        }

        unfolded_line = line;
    }

    // Process final line
    if !unfolded_line.is_empty() {
        process_ics_line(
            &unfolded_line,
            &mut current_component,
            &mut current_props,
            &mut result,
            line_num,
            options,
        );
    }

    Ok(result)
}

/// Process a single ICS line
fn process_ics_line(
    line: &str,
    current_component: &mut IcsComponent,
    current_props: &mut HashMap<String, String>,
    result: &mut ImportResult,
    line_num: usize,
    options: &ImportOptions,
) {
    let trimmed = line.trim();

    // Handle component begin markers
    if trimmed == "BEGIN:VTODO" {
        *current_component = IcsComponent::Vtodo;
        current_props.clear();
        return;
    }

    if trimmed == "BEGIN:VEVENT" {
        *current_component = IcsComponent::Vevent;
        current_props.clear();
        return;
    }

    // Handle component end markers
    if trimmed == "END:VTODO" && *current_component == IcsComponent::Vtodo {
        match parse_ics_vtodo(current_props, options.validate) {
            Ok(task) => result.imported.push(task),
            Err(e) => {
                result.errors.push(ImportError {
                    line: line_num,
                    message: e,
                    content: None,
                });
            }
        }
        *current_component = IcsComponent::None;
        current_props.clear();
        return;
    }

    if trimmed == "END:VEVENT" && *current_component == IcsComponent::Vevent {
        match parse_ics_vevent(current_props, options.validate) {
            Ok(event) => result.imported_events.push(event),
            Err(e) => {
                result.errors.push(ImportError {
                    line: line_num,
                    message: e,
                    content: None,
                });
            }
        }
        *current_component = IcsComponent::None;
        current_props.clear();
        return;
    }

    // Parse property if inside a component
    if *current_component != IcsComponent::None {
        if let Some((key, value)) = parse_ics_property(trimmed) {
            current_props.insert(key, value);
        }
    }
}

/// Parse an ICS property line (KEY:VALUE or KEY;PARAM=X:VALUE)
fn parse_ics_property(line: &str) -> Option<(String, String)> {
    let colon_pos = line.find(':')?;
    let key_part = &line[..colon_pos];
    let value = &line[colon_pos + 1..];

    // Extract just the property name (before any parameters)
    let key = key_part.split(';').next()?.to_uppercase();

    Some((key, unescape_ics(value)))
}

/// Unescape ICS special characters
fn unescape_ics(s: &str) -> String {
    s.replace("\\n", "\n")
        .replace("\\;", ";")
        .replace("\\,", ",")
        .replace("\\\\", "\\")
}

/// Parse a VTODO component into a Task
fn parse_ics_vtodo(props: &HashMap<String, String>, validate: bool) -> Result<Task, String> {
    // SUMMARY (title) is required
    let title = props
        .get("SUMMARY")
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Missing SUMMARY (title)".to_string())?;

    if validate && title.trim().is_empty() {
        return Err("SUMMARY cannot be whitespace only".to_string());
    }

    // Parse UID or generate new
    let id = props
        .get("UID")
        .and_then(|s| Uuid::parse_str(s).ok())
        .map_or_else(TaskId::new, TaskId);

    // Parse STATUS
    let status =
        props
            .get("STATUS")
            .map_or(TaskStatus::Todo, |s| match s.to_uppercase().as_str() {
                "NEEDS-ACTION" => TaskStatus::Todo,
                "IN-PROCESS" => TaskStatus::InProgress,
                "COMPLETED" => TaskStatus::Done,
                "CANCELLED" => TaskStatus::Cancelled,
                _ => TaskStatus::Todo,
            });

    // Parse PRIORITY (1-9 in ICS, 1 is highest)
    let priority = props
        .get("PRIORITY")
        .and_then(|s| s.parse::<u8>().ok())
        .map_or(Priority::None, |p| match p {
            1 => Priority::Urgent,
            2..=3 => Priority::High,
            4..=6 => Priority::Medium,
            7..=8 => Priority::Low,
            _ => Priority::None,
        });

    // Parse DUE date
    let due_date = props.get("DUE").and_then(|s| parse_ics_date(s));

    // Parse DESCRIPTION
    let description = props.get("DESCRIPTION").filter(|s| !s.is_empty()).cloned();

    // Parse CATEGORIES (tags)
    let tags: Vec<String> = props
        .get("CATEGORIES")
        .map(|s| {
            s.split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect()
        })
        .unwrap_or_default();

    // Parse CREATED timestamp
    let created_at = props
        .get("CREATED")
        .or_else(|| props.get("DTSTAMP"))
        .and_then(|s| parse_ics_datetime(s))
        .unwrap_or_else(Utc::now);

    // Parse COMPLETED timestamp
    let completed_at = props.get("COMPLETED").and_then(|s| parse_ics_datetime(s));

    let task = Task {
        id,
        title: title.clone(),
        description,
        status,
        priority,
        due_date,
        scheduled_date: None,
        tags,
        project_id: None,
        parent_task_id: None,
        dependencies: Vec::new(),
        next_task_id: None,
        recurrence: None,
        estimated_minutes: None,
        actual_minutes: 0,
        sort_order: None,
        custom_fields: std::collections::HashMap::new(),
        created_at,
        updated_at: Utc::now(),
        completed_at,
        snooze_until: None,
        git_ref: None,
    };

    Ok(task)
}

/// Parse a VEVENT component into a CalendarEvent
fn parse_ics_vevent(
    props: &HashMap<String, String>,
    validate: bool,
) -> Result<CalendarEvent, String> {
    // SUMMARY (title) is required
    let title = props
        .get("SUMMARY")
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Missing SUMMARY (title)".to_string())?;

    if validate && title.trim().is_empty() {
        return Err("SUMMARY cannot be whitespace only".to_string());
    }

    // DTSTART is required for events
    let dtstart_raw = props
        .get("DTSTART")
        .ok_or_else(|| "Missing DTSTART (start time)".to_string())?;

    // Detect if this is an all-day event (date-only format)
    let all_day = is_date_only(dtstart_raw);

    // Parse start time
    let start = if all_day {
        parse_ics_date(dtstart_raw)
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| Utc.from_utc_datetime(&dt))
            .ok_or_else(|| "Invalid DTSTART date format".to_string())?
    } else {
        parse_ics_datetime(dtstart_raw)
            .ok_or_else(|| "Invalid DTSTART datetime format".to_string())?
    };

    // Parse end time (optional)
    let end = props.get("DTEND").and_then(|s| {
        if is_date_only(s) {
            parse_ics_date(s)
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|dt| Utc.from_utc_datetime(&dt))
        } else {
            parse_ics_datetime(s)
        }
    });

    // Parse UID or generate new
    let uid = props
        .get("UID")
        .cloned()
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Parse STATUS
    let status = props
        .get("STATUS")
        .map_or(CalendarEventStatus::Confirmed, |s| {
            match s.to_uppercase().as_str() {
                "TENTATIVE" => CalendarEventStatus::Tentative,
                "CONFIRMED" => CalendarEventStatus::Confirmed,
                "CANCELLED" => CalendarEventStatus::Cancelled,
                _ => CalendarEventStatus::Confirmed,
            }
        });

    // Parse DESCRIPTION
    let description = props.get("DESCRIPTION").filter(|s| !s.is_empty()).cloned();

    // Parse LOCATION
    let location = props.get("LOCATION").filter(|s| !s.is_empty()).cloned();

    // Parse CREATED timestamp
    let created_at = props
        .get("CREATED")
        .or_else(|| props.get("DTSTAMP"))
        .and_then(|s| parse_ics_datetime(s))
        .unwrap_or_else(Utc::now);

    let event = CalendarEvent::new(title.clone())
        .with_uid(uid)
        .with_start(start)
        .with_all_day(all_day)
        .with_status(status);

    let event = if let Some(end) = end {
        event.with_end(end)
    } else {
        event
    };

    let event = if let Some(desc) = description {
        event.with_description(desc)
    } else {
        event
    };

    let event = if let Some(loc) = location {
        event.with_location(loc)
    } else {
        event
    };

    // Manually set created_at since we parsed it
    let mut event = event;
    event.created_at = created_at;

    Ok(event)
}

/// Check if an ICS date/datetime string is date-only (all-day event)
fn is_date_only(s: &str) -> bool {
    // All-day events have format YYYYMMDD (8 chars) or VALUE=DATE:YYYYMMDD
    // Datetime events have format YYYYMMDDTHHMMSSz (16+ chars with T separator)
    let clean = s.trim_start_matches("VALUE=DATE:");
    !clean.contains('T') && clean.len() == 8
}

/// Parse an ICS date (YYYYMMDD)
fn parse_ics_date(s: &str) -> Option<NaiveDate> {
    // Handle both "YYYYMMDD" and "VALUE=DATE:YYYYMMDD" formats
    let date_str = s.trim_start_matches("VALUE=DATE:");

    if date_str.len() >= 8 {
        let date_part = &date_str[..8];
        NaiveDate::parse_from_str(date_part, "%Y%m%d").ok()
    } else {
        None
    }
}

/// Parse an ICS datetime (YYYYMMDDTHHMMSSz or YYYYMMDDTHHMMSS)
fn parse_ics_datetime(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim().to_uppercase();

    // Try with Z suffix (UTC)
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y%m%dT%H%M%SZ") {
        return Some(Utc.from_utc_datetime(&dt));
    }

    // Try without Z
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y%m%dT%H%M%S") {
        return Some(Utc.from_utc_datetime(&dt));
    }

    // Try date only
    if let Some(d) = parse_ics_date(&s) {
        return Some(Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0)?));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_import_ics_basic() {
        let ics = r"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VTODO
UID:test-123
SUMMARY:Test Task
STATUS:NEEDS-ACTION
PRIORITY:5
END:VTODO
END:VCALENDAR
";
        let reader = Cursor::new(ics);
        let options = ImportOptions::default();

        let result = import_from_ics(reader, &options).unwrap();

        assert_eq!(result.imported.len(), 1);
        assert_eq!(result.imported[0].title, "Test Task");
        assert_eq!(result.imported[0].status, TaskStatus::Todo);
        assert_eq!(result.imported[0].priority, Priority::Medium);
    }

    #[test]
    fn test_import_ics_with_due_date() {
        let ics = r"BEGIN:VCALENDAR
BEGIN:VTODO
SUMMARY:Task with due
DUE;VALUE=DATE:20251225
END:VTODO
END:VCALENDAR
";
        let reader = Cursor::new(ics);
        let options = ImportOptions::default();

        let result = import_from_ics(reader, &options).unwrap();

        assert_eq!(result.imported.len(), 1);
        assert_eq!(
            result.imported[0].due_date,
            Some(NaiveDate::from_ymd_opt(2025, 12, 25).unwrap())
        );
    }

    #[test]
    fn test_import_ics_with_categories() {
        let ics = r"BEGIN:VCALENDAR
BEGIN:VTODO
SUMMARY:Tagged task
CATEGORIES:work,urgent,project
END:VTODO
END:VCALENDAR
";
        let reader = Cursor::new(ics);
        let options = ImportOptions::default();

        let result = import_from_ics(reader, &options).unwrap();

        assert_eq!(result.imported.len(), 1);
        assert_eq!(result.imported[0].tags, vec!["work", "urgent", "project"]);
    }

    #[test]
    fn test_import_ics_completed() {
        let ics = r"BEGIN:VCALENDAR
BEGIN:VTODO
SUMMARY:Completed task
STATUS:COMPLETED
COMPLETED:20251201T120000Z
END:VTODO
END:VCALENDAR
";
        let reader = Cursor::new(ics);
        let options = ImportOptions::default();

        let result = import_from_ics(reader, &options).unwrap();

        assert_eq!(result.imported.len(), 1);
        assert_eq!(result.imported[0].status, TaskStatus::Done);
        assert!(result.imported[0].completed_at.is_some());
    }

    #[test]
    fn test_import_ics_line_folding() {
        let ics = r"BEGIN:VCALENDAR
BEGIN:VTODO
SUMMARY:Task with a very long title that spans
  multiple lines due to line folding
END:VTODO
END:VCALENDAR
";
        let reader = Cursor::new(ics);
        let options = ImportOptions::default();

        let result = import_from_ics(reader, &options).unwrap();

        assert_eq!(result.imported.len(), 1);
        assert!(result.imported[0].title.contains("long title"));
        assert!(result.imported[0].title.contains("multiple lines"));
    }

    #[test]
    fn test_import_ics_escaped_chars() {
        let ics = r"BEGIN:VCALENDAR
BEGIN:VTODO
SUMMARY:Task with\, commas
DESCRIPTION:Line 1\nLine 2
END:VTODO
END:VCALENDAR
";
        let reader = Cursor::new(ics);
        let options = ImportOptions::default();

        let result = import_from_ics(reader, &options).unwrap();

        assert_eq!(result.imported.len(), 1);
        assert_eq!(result.imported[0].title, "Task with, commas");
        assert_eq!(
            result.imported[0].description,
            Some("Line 1\nLine 2".to_string())
        );
    }

    #[test]
    fn test_import_ics_multiple_vtodos() {
        let ics = r"BEGIN:VCALENDAR
BEGIN:VTODO
SUMMARY:Task 1
END:VTODO
BEGIN:VTODO
SUMMARY:Task 2
END:VTODO
BEGIN:VTODO
SUMMARY:Task 3
END:VTODO
END:VCALENDAR
";
        let reader = Cursor::new(ics);
        let options = ImportOptions::default();

        let result = import_from_ics(reader, &options).unwrap();

        assert_eq!(result.imported.len(), 3);
        assert_eq!(result.imported[0].title, "Task 1");
        assert_eq!(result.imported[1].title, "Task 2");
        assert_eq!(result.imported[2].title, "Task 3");
    }

    #[test]
    fn test_parse_ics_date() {
        assert_eq!(
            parse_ics_date("20251225"),
            Some(NaiveDate::from_ymd_opt(2025, 12, 25).unwrap())
        );
        assert_eq!(
            parse_ics_date("VALUE=DATE:20251225"),
            Some(NaiveDate::from_ymd_opt(2025, 12, 25).unwrap())
        );
    }

    #[test]
    fn test_parse_ics_datetime() {
        let result = parse_ics_datetime("20251225T120000Z");
        assert!(result.is_some());

        let result = parse_ics_datetime("20251225T120000");
        assert!(result.is_some());
    }

    #[test]
    fn test_unescape_ics() {
        assert_eq!(unescape_ics("hello"), "hello");
        assert_eq!(unescape_ics("a\\;b"), "a;b");
        assert_eq!(unescape_ics("a\\,b"), "a,b");
        assert_eq!(unescape_ics("a\\nb"), "a\nb");
        assert_eq!(unescape_ics("a\\\\b"), "a\\b");
    }

    // VEVENT tests

    #[test]
    fn test_import_vevent_basic() {
        let ics = r"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:event-123
SUMMARY:Team Meeting
DTSTART:20241215T100000Z
DTEND:20241215T110000Z
END:VEVENT
END:VCALENDAR
";
        let reader = Cursor::new(ics);
        let options = ImportOptions::default();

        let result = import_from_ics(reader, &options).unwrap();

        assert_eq!(result.imported.len(), 0);
        assert_eq!(result.imported_events.len(), 1);
        assert_eq!(result.imported_events[0].title, "Team Meeting");
        assert_eq!(result.imported_events[0].uid, "event-123");
        assert!(!result.imported_events[0].all_day);
        assert!(result.imported_events[0].end.is_some());
    }

    #[test]
    fn test_import_vevent_all_day() {
        let ics = r"BEGIN:VCALENDAR
BEGIN:VEVENT
SUMMARY:Holiday
DTSTART;VALUE=DATE:20241225
END:VEVENT
END:VCALENDAR
";
        let reader = Cursor::new(ics);
        let options = ImportOptions::default();

        let result = import_from_ics(reader, &options).unwrap();

        assert_eq!(result.imported_events.len(), 1);
        assert_eq!(result.imported_events[0].title, "Holiday");
        assert!(result.imported_events[0].all_day);
    }

    #[test]
    fn test_import_vevent_with_location_description() {
        let ics = r"BEGIN:VCALENDAR
BEGIN:VEVENT
SUMMARY:Conference
DTSTART:20241215T090000Z
LOCATION:Main Hall Room A
DESCRIPTION:Annual team conference
END:VEVENT
END:VCALENDAR
";
        let reader = Cursor::new(ics);
        let options = ImportOptions::default();

        let result = import_from_ics(reader, &options).unwrap();

        assert_eq!(result.imported_events.len(), 1);
        assert_eq!(
            result.imported_events[0].location,
            Some("Main Hall Room A".to_string())
        );
        assert_eq!(
            result.imported_events[0].description,
            Some("Annual team conference".to_string())
        );
    }

    #[test]
    fn test_import_vevent_status() {
        let ics = r"BEGIN:VCALENDAR
BEGIN:VEVENT
SUMMARY:Maybe Meeting
DTSTART:20241215T100000Z
STATUS:TENTATIVE
END:VEVENT
END:VCALENDAR
";
        let reader = Cursor::new(ics);
        let options = ImportOptions::default();

        let result = import_from_ics(reader, &options).unwrap();

        assert_eq!(result.imported_events.len(), 1);
        assert_eq!(
            result.imported_events[0].status,
            CalendarEventStatus::Tentative
        );
    }

    #[test]
    fn test_import_mixed_vtodo_vevent() {
        let ics = r"BEGIN:VCALENDAR
BEGIN:VTODO
SUMMARY:Buy groceries
STATUS:NEEDS-ACTION
END:VTODO
BEGIN:VEVENT
SUMMARY:Doctor appointment
DTSTART:20241220T140000Z
DTEND:20241220T150000Z
END:VEVENT
BEGIN:VTODO
SUMMARY:Review code
STATUS:IN-PROCESS
END:VTODO
BEGIN:VEVENT
SUMMARY:Team lunch
DTSTART:20241221T120000Z
END:VEVENT
END:VCALENDAR
";
        let reader = Cursor::new(ics);
        let options = ImportOptions::default();

        let result = import_from_ics(reader, &options).unwrap();

        // Should have 2 tasks and 2 events
        assert_eq!(result.imported.len(), 2);
        assert_eq!(result.imported_events.len(), 2);

        // Verify tasks
        assert_eq!(result.imported[0].title, "Buy groceries");
        assert_eq!(result.imported[1].title, "Review code");

        // Verify events
        assert_eq!(result.imported_events[0].title, "Doctor appointment");
        assert_eq!(result.imported_events[1].title, "Team lunch");
    }

    #[test]
    fn test_import_vevent_missing_summary() {
        let ics = r"BEGIN:VCALENDAR
BEGIN:VEVENT
DTSTART:20241215T100000Z
END:VEVENT
END:VCALENDAR
";
        let reader = Cursor::new(ics);
        let options = ImportOptions::default();

        let result = import_from_ics(reader, &options).unwrap();

        assert_eq!(result.imported_events.len(), 0);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].message.contains("SUMMARY"));
    }

    #[test]
    fn test_import_vevent_missing_dtstart() {
        let ics = r"BEGIN:VCALENDAR
BEGIN:VEVENT
SUMMARY:Bad Event
END:VEVENT
END:VCALENDAR
";
        let reader = Cursor::new(ics);
        let options = ImportOptions::default();

        let result = import_from_ics(reader, &options).unwrap();

        assert_eq!(result.imported_events.len(), 0);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].message.contains("DTSTART"));
    }

    #[test]
    fn test_is_date_only() {
        assert!(is_date_only("20241225"));
        assert!(is_date_only("VALUE=DATE:20241225"));
        assert!(!is_date_only("20241225T100000Z"));
        assert!(!is_date_only("20241225T100000"));
    }
}
