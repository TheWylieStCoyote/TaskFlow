//! Time log editor component.
//!
//! A modal popup for viewing and editing the time entries of a task. Entries
//! are grouped by local calendar day (newest day first) under a header that
//! carries the day's total, and a footer sums the whole log.
//!
//! ```text
//! Tue 2026-10-21 ·························· 2h 15m
//!   09:30 – 11:15   1h 45m   standup prep
//! ● 14:00 – now     30m
//! Mon 2026-10-20 ·························· 3h 30m
//!   08:15 – 11:45   3h 30m
//! Total: 5h 45m · 3 entries
//! ```
//!
//! # Modes
//!
//! - **Browse**: Navigate through time entries (headers are skipped)
//! - **EditStart** / **EditEnd**: Edit the start or end clock time inline
//! - **EditNote**: Edit the entry's description inline
//! - **EditDate**: Move the entry to another date (prompt in the footer)
//! - **Add**: Add a new entry from a one-line prompt in the footer
//! - **ConfirmDelete**: Confirm deletion of an entry

use chrono::{Local, NaiveDate};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Widget},
};

use crate::config::Theme;
use crate::domain::{format_minutes, LocalDate, TimeEntry, TimeEntryId};

/// Mode for time log editor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeLogMode {
    #[default]
    Browse,
    EditStart,
    EditEnd,
    EditNote,
    EditDate,
    Add,
    ConfirmDelete,
}

impl TimeLogMode {
    /// Whether this mode has an open text prompt that consumes typed characters.
    #[must_use]
    pub const fn is_text_input(self) -> bool {
        matches!(
            self,
            Self::EditStart | Self::EditEnd | Self::EditNote | Self::EditDate | Self::Add
        )
    }
}

/// The entries of one local calendar day, newest first.
#[derive(Debug)]
pub struct DayGroup<'a> {
    /// Local calendar date of the group
    pub date: NaiveDate,
    /// Sum of the group's entry durations (running entries count their elapsed time)
    pub total_minutes: u32,
    /// `(index into the original entry list, entry)` pairs, newest first
    pub entries: Vec<(usize, &'a TimeEntry)>,
}

/// Group entries by their local start date, newest day first.
///
/// Each entry keeps its index in `entries` so that the caller's selection
/// index (which counts entries only, never headers) stays meaningful.
#[must_use]
pub fn group_by_day<'a>(entries: &[&'a TimeEntry]) -> Vec<DayGroup<'a>> {
    let mut indexed: Vec<(usize, &'a TimeEntry)> = entries.iter().copied().enumerate().collect();
    indexed.sort_by_key(|(_, e)| std::cmp::Reverse(e.started_at));

    let mut groups: Vec<DayGroup<'a>> = Vec::new();
    for (idx, entry) in indexed {
        let date = entry.started_at.local_date();
        let minutes = entry.calculated_duration_minutes();
        match groups.last_mut() {
            Some(group) if group.date == date => {
                group.total_minutes += minutes;
                group.entries.push((idx, entry));
            }
            _ => groups.push(DayGroup {
                date,
                total_minutes: minutes,
                entries: vec![(idx, entry)],
            }),
        }
    }
    groups
}

/// Sum of all entry durations (running entries count their elapsed time).
#[must_use]
pub fn total_minutes(entries: &[&TimeEntry]) -> u32 {
    entries
        .iter()
        .map(|e| e.calculated_duration_minutes())
        .sum()
}

/// Day header label, e.g. `Tue 2026-10-21`.
#[must_use]
pub fn day_label(date: NaiveDate) -> String {
    date.format("%a %Y-%m-%d").to_string()
}

/// Footer summary, e.g. `Total: 5h 45m · 7 entries`.
#[must_use]
pub fn footer_summary(entries: &[&TimeEntry]) -> String {
    let noun = if entries.len() == 1 {
        "entry"
    } else {
        "entries"
    };
    format!(
        "Total: {} · {} {noun}",
        format_minutes(total_minutes(entries)),
        entries.len()
    )
}

/// One rendered line of the grouped list.
#[derive(Debug, Clone, Copy)]
enum Row<'a> {
    /// A day header; the index points into the group list.
    Header(usize),
    /// An entry row with its index into the original entry list.
    Entry(usize, &'a TimeEntry),
}

fn build_rows<'a>(groups: &[DayGroup<'a>]) -> Vec<Row<'a>> {
    let mut rows = Vec::new();
    for (g, group) in groups.iter().enumerate() {
        rows.push(Row::Header(g));
        for &(idx, entry) in &group.entries {
            rows.push(Row::Entry(idx, entry));
        }
    }
    rows
}

/// Time log editor popup widget
pub struct TimeLogEditor<'a> {
    entries: Vec<&'a TimeEntry>,
    selected: usize,
    mode: TimeLogMode,
    edit_buffer: &'a str,
    theme: &'a Theme,
}

impl<'a> TimeLogEditor<'a> {
    #[must_use]
    pub fn new(
        entries: Vec<&'a TimeEntry>,
        selected: usize,
        mode: TimeLogMode,
        edit_buffer: &'a str,
        theme: &'a Theme,
    ) -> Self {
        Self {
            entries,
            selected,
            mode,
            edit_buffer,
            theme,
        }
    }

    /// Get the selected entry ID if any
    #[must_use]
    pub fn selected_entry_id(&self) -> Option<&TimeEntryId> {
        self.entries.get(self.selected).map(|e| &e.id)
    }

    /// Popup height that shows every header and entry row plus the footer
    /// and borders, with room for the empty-state hint.
    #[must_use]
    pub fn preferred_height(entries: &[&TimeEntry]) -> u16 {
        let groups = group_by_day(entries);
        let rows = groups.len() + entries.len();
        // rows + footer + 2 borders; at least one list row
        u16::try_from(rows.max(1) + 3).unwrap_or(u16::MAX)
    }

    fn title(&self) -> &'static str {
        match self.mode {
            TimeLogMode::Browse => {
                " Time Log · a add · s/e start/end · n note · t date · +/- 5m · d delete · Esc "
            }
            TimeLogMode::EditStart => " Edit start (HH:MM) · Enter save · Esc cancel ",
            TimeLogMode::EditEnd => {
                " Edit end (HH:MM, before start = next day) · Enter save · Esc cancel "
            }
            TimeLogMode::EditNote => " Edit note · Enter save · Esc cancel ",
            TimeLogMode::EditDate => {
                " Move to date (YYYY-MM-DD, today, yesterday, -N) · Enter save · Esc cancel "
            }
            TimeLogMode::Add => {
                " Add entry · [date] START[-END | +DURATION] [note] · Enter save · Esc cancel "
            }
            TimeLogMode::ConfirmDelete => " Delete this entry? (y=yes, n=no) ",
        }
    }

    fn header_line(&self, group: &DayGroup<'_>, width: usize) -> Line<'static> {
        let theme = self.theme;
        let label = day_label(group.date);
        let total = format_minutes(group.total_minutes);
        let dots = width
            .saturating_sub(label.chars().count() + total.chars().count() + 2)
            .max(1);
        let is_today = group.date == crate::domain::today();
        Line::from(vec![
            Span::styled(
                label,
                Style::default()
                    .fg(if is_today {
                        theme.colors.success.to_color()
                    } else {
                        theme.colors.accent.to_color()
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                "·".repeat(dots),
                Style::default().fg(theme.colors.muted.to_color()),
            ),
            Span::raw(" "),
            Span::styled(
                total,
                Style::default()
                    .fg(theme.colors.foreground.to_color())
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    }

    fn entry_line(&self, idx: usize, entry: &TimeEntry) -> Line<'static> {
        let theme = self.theme;
        let is_selected = idx == self.selected;
        let is_running = entry.is_running();
        let editing = |mode: TimeLogMode| is_selected && self.mode == mode;
        let prompt_style = Style::default()
            .fg(theme.colors.warning.to_color())
            .add_modifier(Modifier::BOLD);

        let local_start = entry.started_at.with_timezone(&Local);
        let start_time = local_start.format("%H:%M").to_string();
        let end_time = entry.ended_at.map_or_else(
            || "now".to_string(),
            |t| t.with_timezone(&Local).format("%H:%M").to_string(),
        );
        let duration = format_minutes(entry.calculated_duration_minutes());

        let mut spans = vec![Span::styled(
            if is_running { "● " } else { "  " },
            Style::default().fg(Color::Red).add_modifier(if is_running {
                Modifier::SLOW_BLINK
            } else {
                Modifier::empty()
            }),
        )];

        if editing(TimeLogMode::EditStart) {
            spans.push(Span::styled(
                format!("[{}]", self.edit_buffer),
                prompt_style,
            ));
        } else {
            spans.push(Span::styled(
                start_time,
                Style::default()
                    .fg(theme.colors.accent.to_color())
                    .add_modifier(Modifier::BOLD),
            ));
        }

        spans.push(Span::styled(
            " – ",
            Style::default().fg(theme.colors.muted.to_color()),
        ));

        if editing(TimeLogMode::EditEnd) {
            spans.push(Span::styled(
                format!("[{}]", self.edit_buffer),
                prompt_style,
            ));
        } else {
            spans.push(Span::styled(
                format!("{end_time:<5}"),
                Style::default().fg(if is_running {
                    Color::Red
                } else {
                    theme.colors.accent.to_color()
                }),
            ));
        }

        spans.push(Span::raw("   "));
        spans.push(Span::styled(
            format!("{duration:<7}"),
            Style::default().fg(theme.colors.foreground.to_color()),
        ));

        if editing(TimeLogMode::EditNote) {
            spans.push(Span::raw("   "));
            spans.push(Span::styled(
                format!("[{}]", self.edit_buffer),
                prompt_style,
            ));
        } else if let Some(note) = entry.description.as_deref().filter(|n| !n.is_empty()) {
            spans.push(Span::raw("   "));
            spans.push(Span::styled(
                note.to_string(),
                Style::default().fg(theme.colors.muted.to_color()),
            ));
        }

        Line::from(spans)
    }

    fn footer_line(&self) -> Line<'static> {
        let theme = self.theme;
        let prompt_style = Style::default()
            .fg(theme.colors.warning.to_color())
            .add_modifier(Modifier::BOLD);
        match self.mode {
            TimeLogMode::Add => Line::from(vec![
                Span::styled("Add › ", Style::default().fg(theme.colors.muted.to_color())),
                Span::styled(format!("{}▏", self.edit_buffer), prompt_style),
            ]),
            TimeLogMode::EditDate => Line::from(vec![
                Span::styled(
                    "Move to › ",
                    Style::default().fg(theme.colors.muted.to_color()),
                ),
                Span::styled(format!("{}▏", self.edit_buffer), prompt_style),
            ]),
            _ => {
                let summary = footer_summary(&self.entries);
                Line::from(Span::styled(
                    summary,
                    Style::default()
                        .fg(theme.colors.foreground.to_color())
                        .add_modifier(Modifier::BOLD),
                ))
            }
        }
    }
}

impl Widget for TimeLogEditor<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let theme = self.theme;
        let border_color = match self.mode {
            TimeLogMode::Browse => theme.colors.accent.to_color(),
            TimeLogMode::ConfirmDelete => theme.colors.danger.to_color(),
            _ => theme.colors.warning.to_color(),
        };

        let block = Block::default()
            .title(self.title())
            .title_style(
                Style::default()
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        // Last inner line is the footer; the rest is the scrolling list.
        let footer_area = Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1);
        let list_height = usize::from(inner.height - 1);
        let width = usize::from(inner.width);

        if self.entries.is_empty() {
            if list_height > 0 {
                buf.set_line(
                    inner.x,
                    inner.y,
                    &Line::from(Span::styled(
                        "  No time entries yet. Press 'a' to add one.",
                        Style::default().fg(theme.colors.muted.to_color()),
                    )),
                    inner.width,
                );
            }
            buf.set_line(
                footer_area.x,
                footer_area.y,
                &self.footer_line(),
                inner.width,
            );
            return;
        }

        let groups = group_by_day(&self.entries);
        let rows = build_rows(&groups);
        let selected_row = rows
            .iter()
            .position(|r| matches!(r, Row::Entry(idx, _) if *idx == self.selected))
            .unwrap_or(0);

        // Keep the selection visible, centred once the list overflows.
        let offset = if list_height == 0 {
            0
        } else {
            selected_row
                .saturating_sub(list_height / 2)
                .min(rows.len().saturating_sub(list_height))
        };

        let selected_style = Style::default()
            .bg(theme.colors.accent_secondary.to_color())
            .add_modifier(Modifier::BOLD);

        for (i, row) in rows.iter().skip(offset).take(list_height).enumerate() {
            let y = inner.y + u16::try_from(i).unwrap_or(u16::MAX);
            match row {
                Row::Header(g) => {
                    buf.set_line(
                        inner.x,
                        y,
                        &self.header_line(&groups[*g], width),
                        inner.width,
                    );
                }
                Row::Entry(idx, entry) => {
                    buf.set_line(inner.x, y, &self.entry_line(*idx, entry), inner.width);
                    if *idx == self.selected {
                        buf.set_style(Rect::new(inner.x, y, inner.width, 1), selected_style);
                    }
                }
            }
        }

        buf.set_line(
            footer_area.x,
            footer_area.y,
            &self.footer_line(),
            inner.width,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Theme;
    use crate::domain::{today, TaskId};
    use crate::ui::test_utils::{buffer_content, render_widget};
    use chrono::{DateTime, Duration, TimeZone, Utc};

    /// A stopped entry starting at `h:m` local on `today() - days_ago`.
    fn entry_on(days_ago: i64, h: u32, m: u32, minutes: u32) -> TimeEntry {
        let date = today() - Duration::days(days_ago);
        let local = date.and_hms_opt(h, m, 0).unwrap();
        let started_at: DateTime<Utc> = Local
            .from_local_datetime(&local)
            .single()
            .unwrap()
            .with_timezone(&Utc);
        let mut e = TimeEntry::start(TaskId::new());
        e.started_at = started_at;
        e.ended_at = Some(started_at + Duration::minutes(i64::from(minutes)));
        e.duration_minutes = Some(minutes);
        e
    }

    fn running_on(days_ago: i64, h: u32, m: u32, minutes: u32) -> TimeEntry {
        let mut e = entry_on(days_ago, h, m, minutes);
        e.ended_at = None;
        e.duration_minutes = Some(minutes);
        e
    }

    fn render(entries: &[&TimeEntry], selected: usize, mode: TimeLogMode, buf: &str) -> String {
        let theme = Theme::default();
        let editor = TimeLogEditor::new(entries.to_vec(), selected, mode, buf, &theme);
        buffer_content(&render_widget(editor, 90, 14))
    }

    #[test]
    fn empty_log_shows_hint_and_zero_footer() {
        let content = render(&[], 0, TimeLogMode::Browse, "");
        assert!(content.contains("Time Log"));
        assert!(content.contains("No time entries"));
        assert!(content.contains("Total: 0m · 0 entries"));
    }

    #[test]
    fn groups_by_local_day_newest_first_with_day_totals() {
        let a = entry_on(0, 9, 30, 105); // today
        let b = entry_on(0, 14, 0, 30); // today
        let c = entry_on(1, 8, 15, 210); // yesterday
        let groups = group_by_day(&[&c, &a, &b]);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].date, today());
        assert_eq!(groups[0].total_minutes, 135);
        // newest first within the day, indices refer to the input order
        assert_eq!(
            groups[0]
                .entries
                .iter()
                .map(|(i, _)| *i)
                .collect::<Vec<_>>(),
            vec![2, 1]
        );
        assert_eq!(groups[1].date, today() - Duration::days(1));
        assert_eq!(groups[1].total_minutes, 210);
        assert_eq!(groups[1].entries[0].0, 0);
    }

    #[test]
    fn renders_headers_entries_and_footer() {
        let a = entry_on(0, 9, 30, 105);
        let b = entry_on(1, 8, 15, 210);
        let content = render(&[&a, &b], 0, TimeLogMode::Browse, "");

        assert!(content.contains(&day_label(today())));
        assert!(content.contains(&day_label(today() - Duration::days(1))));
        assert!(content.contains("09:30 – 11:15   1h 45m"));
        assert!(content.contains("08:15 – 11:45   3h 30m"));
        assert!(content.contains("Total: 5h 15m · 2 entries"));
        // header total sits at the right edge after a dotted leader
        assert!(content.contains("·· 1h 45m"));
        assert!(content.contains("·· 3h 30m"));
    }

    #[test]
    fn footer_uses_singular_for_one_entry() {
        let a = entry_on(0, 9, 0, 30);
        assert_eq!(footer_summary(&[&a]), "Total: 30m · 1 entry");
        assert_eq!(total_minutes(&[&a, &a]), 60);
    }

    #[test]
    fn running_entry_shows_now_marker_and_counts_in_totals() {
        let running = running_on(0, 10, 0, 20);
        let done = entry_on(0, 8, 0, 60);
        let content = render(&[&running, &done], 0, TimeLogMode::Browse, "");

        assert!(content.contains("● 10:00 – now"));
        assert!(content.contains("Total: 1h 20m · 2 entries"));
    }

    #[test]
    fn description_is_shown_after_duration() {
        let mut a = entry_on(0, 9, 0, 45);
        a.description = Some("standup prep".to_string());
        let content = render(&[&a], 0, TimeLogMode::Browse, "");
        assert!(content.contains("09:00 – 09:45   45m       standup prep"));
    }

    #[test]
    fn selection_highlights_an_entry_row_never_a_header() {
        let a = entry_on(0, 9, 0, 30);
        let b = entry_on(1, 9, 0, 30);
        let theme = Theme::default();
        let bg = theme.colors.accent_secondary.to_color();

        // selected = 1 is the yesterday entry, which sits under the second header
        let editor = TimeLogEditor::new(vec![&a, &b], 1, TimeLogMode::Browse, "", &theme);
        let buffer = render_widget(editor, 80, 10);
        let highlighted: Vec<u16> = (0..buffer.area.height)
            .filter(|&y| buffer.cell((2, y)).unwrap().style().bg == Some(bg))
            .collect();
        assert_eq!(highlighted.len(), 1, "exactly one highlighted row");
        let y = highlighted[0];
        let row: String = (0..buffer.area.width)
            .map(|x| buffer.cell((x, y)).unwrap().symbol().to_string())
            .collect();
        assert!(
            row.contains("09:00 – 09:30"),
            "highlighted row is an entry: {row}"
        );
        assert!(!row.contains(&day_label(today() - Duration::days(1))));
        // rows: border, header(today), entry a, header(yesterday), entry b
        assert_eq!(y, 4);
    }

    #[test]
    fn list_scrolls_to_keep_selection_visible() {
        let entries: Vec<TimeEntry> = (0..12).map(|d| entry_on(d, 9, 0, 15 + d as u32)).collect();
        let refs: Vec<&TimeEntry> = entries.iter().collect();
        let theme = Theme::default();
        // 24 rows (12 headers + 12 entries) into 10 lines: 8 list rows + footer + borders
        let editor = TimeLogEditor::new(refs.clone(), 11, TimeLogMode::Browse, "", &theme);
        let content = buffer_content(&render_widget(editor, 80, 10));
        assert!(
            content.contains("09:00 – 09:26   26m"),
            "last entry visible: {content}"
        );
        assert!(
            !content.contains("09:00 – 09:15   15m"),
            "first entry scrolled away"
        );
        assert!(content.contains("Total:"), "footer always visible");

        let editor = TimeLogEditor::new(refs, 0, TimeLogMode::Browse, "", &theme);
        let content = buffer_content(&render_widget(editor, 80, 10));
        assert!(
            content.contains("09:00 – 09:15   15m"),
            "first entry visible at top"
        );
        assert!(!content.contains("26m"));
    }

    #[test]
    fn preferred_height_counts_headers_entries_footer_and_borders() {
        assert_eq!(TimeLogEditor::preferred_height(&[]), 4);
        let a = entry_on(0, 9, 0, 30);
        let b = entry_on(0, 10, 0, 30);
        let c = entry_on(1, 9, 0, 30);
        // 2 headers + 3 entries + footer + 2 borders
        assert_eq!(TimeLogEditor::preferred_height(&[&a, &b, &c]), 8);
    }

    #[test]
    fn titles_list_keys_per_mode() {
        let a = entry_on(0, 9, 0, 30);
        let browse = render(&[&a], 0, TimeLogMode::Browse, "");
        assert!(
            browse.contains("a add · s/e start/end · n note · t date · +/- 5m · d delete · Esc")
        );
        assert!(render(&[&a], 0, TimeLogMode::EditStart, "10:30").contains("Edit start"));
        assert!(render(&[&a], 0, TimeLogMode::EditEnd, "10:30").contains("Edit end"));
        assert!(render(&[&a], 0, TimeLogMode::EditNote, "x").contains("Edit note"));
        assert!(render(&[&a], 0, TimeLogMode::EditDate, "today").contains("Move to date"));
        assert!(render(&[&a], 0, TimeLogMode::Add, "").contains("Add entry"));
        assert!(render(&[&a], 0, TimeLogMode::ConfirmDelete, "").contains("Delete this entry?"));
    }

    #[test]
    fn inline_edits_show_buffer_in_place() {
        let mut a = entry_on(0, 9, 0, 30);
        a.description = Some("old note".to_string());
        assert!(render(&[&a], 0, TimeLogMode::EditStart, "10:30").contains("[10:30] – 09:30"));
        assert!(render(&[&a], 0, TimeLogMode::EditEnd, "11:00").contains("09:00 – [11:00]"));
        let note = render(&[&a], 0, TimeLogMode::EditNote, "new note");
        assert!(note.contains("[new note]"));
        assert!(!note.contains("old note"));
    }

    #[test]
    fn add_and_date_prompts_replace_footer() {
        let a = entry_on(0, 9, 0, 30);
        let add = render(&[&a], 0, TimeLogMode::Add, "09:00-10:30 standup");
        assert!(add.contains("Add › 09:00-10:30 standup▏"));
        assert!(!add.contains("Total:"));
        let date = render(&[&a], 0, TimeLogMode::EditDate, "-1");
        assert!(date.contains("Move to › -1▏"));
        // The add prompt also works with an empty log
        assert!(render(&[], 0, TimeLogMode::Add, "").contains("Add › ▏"));
    }

    #[test]
    fn text_input_modes() {
        for mode in [
            TimeLogMode::EditStart,
            TimeLogMode::EditEnd,
            TimeLogMode::EditNote,
            TimeLogMode::EditDate,
            TimeLogMode::Add,
        ] {
            assert!(mode.is_text_input(), "{mode:?}");
        }
        assert!(!TimeLogMode::Browse.is_text_input());
        assert!(!TimeLogMode::ConfirmDelete.is_text_input());
    }

    #[test]
    fn selected_entry_id_follows_selection() {
        let a = entry_on(0, 9, 0, 30);
        let b = entry_on(1, 9, 0, 30);
        let theme = Theme::default();
        let editor = TimeLogEditor::new(vec![&a, &b], 1, TimeLogMode::Browse, "", &theme);
        assert_eq!(editor.selected_entry_id(), Some(&b.id));
        let editor = TimeLogEditor::new(vec![&a, &b], 5, TimeLogMode::Browse, "", &theme);
        assert_eq!(editor.selected_entry_id(), None);
    }
}
