# Filter Syntax Guide

TaskFlow includes a powerful filter DSL (Domain-Specific Language) for querying tasks. Use it with the `--filter` option in CLI commands or in saved views.

## Quick Reference

```
# Basic filters
priority:high              # By priority
status:todo               # By status
tags:bug                  # By tag
project:backend           # By project name

# Date filters
due:today                 # Due today
due:thisweek              # Due this week
due:2025-01-01..2025-01-31  # Date range

# Combine with operators
priority:high AND !status:done
tags:bug OR tags:urgent
(priority:high OR priority:urgent) AND due:thisweek
```

---

## Field Reference

### Basic Fields

| Field | Values | Description |
|-------|--------|-------------|
| `priority:` | `none`, `low`, `medium`/`med`, `high`, `urgent` | Task priority level |
| `status:` | `todo`, `in_progress`, `blocked`, `done`, `cancelled` | Task status |
| `tags:` or `tag:` | Any string | Tag name (case-insensitive) |
| `project:` | Any string | Project name (partial match) |
| `title:` | Any string | Title text (partial match) |
| `search:` | Any string | Searches title and description |

### Date Fields

All date fields support keywords, comparisons, and ranges.

| Field | Keywords |
|-------|----------|
| `due:` | `today`, `tomorrow`, `thisweek`, `nextweek`, `overdue`, `none` |
| `created:` | `today`, `yesterday`, `thisweek`, `lastweek` |
| `scheduled:` | `today`, `tomorrow`, `thisweek`, `nextweek`, `none` |
| `completed:` | `today`, `yesterday`, `thisweek`, `lastweek` |
| `modified:` | `today`, `yesterday`, `thisweek`, `lastweek` |

**Date syntax:**
- `due:2025-06-15` - Exact date (YYYY-MM-DD format)
- `due:<2025-06-15` - Before date (exclusive)
- `due:>2025-06-15` - After date (exclusive)
- `due:2025-01-01..2025-12-31` - Date range (inclusive)
- `due:2025-06-01..` - On or after date
- `due:..2025-06-30` - On or before date

### Numeric Fields

| Field | Description |
|-------|-------------|
| `estimate:` or `est:` | Time estimate (in minutes) |
| `actual:` or `tracked:` | Tracked time (in minutes) |

**Numeric syntax:**
- `estimate:60` - Exact value
- `estimate:>60` - Greater than
- `estimate:<30` - Less than
- `estimate:>=60` - Greater or equal
- `estimate:<=30` - Less or equal
- `estimate:30..120` - Range (inclusive)
- `estimate:60..` - Greater or equal
- `estimate:..30` - Less or equal
- `estimate:none` - No value set

### Field Presence (`has:`)

Check if a task has a value set for a field.

| Filter | Description |
|--------|-------------|
| `has:due` | Has a due date |
| `has:project` | Assigned to a project |
| `has:tags` | Has at least one tag |
| `has:estimate` | Has a time estimate |
| `has:description` | Has a description |
| `has:recurrence` | Has a recurrence pattern |
| `has:scheduled` | Has a scheduled date |
| `has:dependencies` | Has blocking dependencies |
| `has:parent` | Is a subtask |
| `has:tracked` | Has tracked time |

---

## Operators

| Operator | Precedence | Description |
|----------|------------|-------------|
| `!` | Highest | Negation (NOT) |
| `AND` | Medium | Both conditions must match |
| `OR` | Lowest | Either condition must match |
| `()` | Override | Grouping for precedence |

**Precedence example:**
```
# Without parentheses: status:todo OR priority:high AND tags:bug
# Parsed as: status:todo OR (priority:high AND tags:bug)

# With parentheses:
(status:todo OR priority:high) AND tags:bug
```

---

## Examples by Use Case

### Priority & Status

```
# High-priority incomplete tasks
priority:high AND !status:done

# Urgent blocked tasks
priority:urgent AND status:blocked

# All active tasks
!status:done AND !status:cancelled

# Ready to work on (todo, not blocked)
status:todo AND !has:dependencies
```

### Due Dates

```
# Due today or overdue
due:today OR due:overdue

# Due this week, high priority
due:thisweek AND (priority:high OR priority:urgent)

# Tasks due in January 2025
due:2025-01-01..2025-01-31

# Tasks due in Q1 2025
due:2025-01-01..2025-03-31

# Overdue in a specific project
due:overdue AND project:frontend

# Tasks with no due date
due:none AND status:todo
```

### Scheduling & Planning

```
# Scheduled for today
scheduled:today

# Scheduled but not started
has:scheduled AND status:todo

# Needs planning (no estimate)
!has:estimate AND status:todo

# Large tasks (over 2 hours)
estimate:>120

# Quick tasks (under 30 min)
estimate:<30
```

### Time Tracking

```
# Tasks with tracked time
has:tracked

# Over an hour tracked
actual:>60

# 1-2 hours tracked
actual:60..120

# Medium tasks (30 min to 2 hours)
estimate:30..120

# Completed with no time tracked
status:done AND actual:0
```

### Recently Changed

```
# Modified today
modified:today

# Created this week, still pending
created:thisweek AND status:todo

# Completed yesterday
completed:yesterday

# Created in last month
created:2025-05-01..2025-05-31
```

### Tags & Projects

```
# Bugs not done
tags:bug AND !status:done

# Tasks with both tags
tags:bug AND tags:urgent

# Tasks with either tag
tags:bug OR tags:feature

# Search in a project
project:backend AND search:"authentication"

# Untagged tasks
!has:tags AND status:todo
```

### Task Relationships

```
# Blocked tasks (has dependencies)
has:dependencies

# Subtasks only
has:parent

# Recurring tasks
has:recurrence

# Top-level tasks only (not subtasks)
!has:parent
```

### Complex Queries

```
# High priority bugs due this week
(priority:high OR priority:urgent) AND tags:bug AND due:thisweek AND !status:done

# Tasks needing attention
due:overdue OR status:blocked

# Unplanned work
!has:estimate AND !has:due AND status:todo

# Ready for review (done but recently completed)
status:done AND completed:thisweek

# Sprint tasks (estimated, scheduled, in progress)
has:estimate AND has:scheduled AND status:in_progress
```

---

## CLI Usage

Use the `--filter` option with the `list` command:

```bash
# List high-priority bugs
taskflow list --filter "priority:high AND tags:bug"

# List overdue tasks
taskflow list --filter "due:overdue"

# List tasks for today
taskflow list --filter "due:today OR scheduled:today"

# Complex filter with quotes
taskflow list --filter "(priority:high OR priority:urgent) AND !status:done"
```

---

## Tips

1. **Case insensitive**: `priority:HIGH` and `priority:high` are equivalent
2. **Partial matching**: `project:back` matches "Backend Services"
3. **Quoted strings**: Use quotes for values with spaces: `search:"fix login"`
4. **Aliases**: `tag:` = `tags:`, `est:` = `estimate:`, `updated:` = `modified:`
5. **Status variants**: `in_progress` = `in-progress` = `inprogress`

---

## Troubleshooting

### Common Errors

| Error | Cause | Fix |
|-------|-------|-----|
| "Unknown field" | Typo in field name | Check spelling: `priority`, `status`, `tags`, etc. |
| "Invalid value" | Wrong value for field | Check valid values (e.g., priority: none/low/medium/high/urgent) |
| "Empty expression" | No filter provided | Provide at least one condition |
| "Unexpected token" | Syntax error | Check operator placement, parentheses balance |

### Examples of Invalid Filters

```
# Wrong: unknown field
category:work          # Use: tags:work or project:work

# Wrong: invalid priority value
priority:critical      # Use: priority:urgent

# Wrong: invalid date format
due:01-15-2025        # Use: due:2025-01-15 (YYYY-MM-DD)

# Wrong: missing operator
priority:high tags:bug # Use: priority:high AND tags:bug
```
