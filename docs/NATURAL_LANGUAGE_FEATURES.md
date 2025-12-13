# Natural Language & Parsing Features - Analysis

## Overview

Analysis of natural language parsing features to enhance TaskFlow's quick-add syntax.

---

## Current Parsing Capabilities

**Location**: `src/app/quick_add/`

### Existing Syntax
| Syntax | Example | Field Set |
|--------|---------|-----------|
| `#tag` | `#work` | `tags` |
| `!priority` | `!high` | `priority` |
| `due:date` | `due:tomorrow` | `due_date` |
| `sched:date` | `sched:monday` | `scheduled_date` |
| `time:range` | `time:9:00-11:00` | `scheduled_start_time`, `scheduled_end_time` |
| `@project` | `@Work` | `project_name` |

### Existing Date Parser (`date.rs`)
- Keywords: `today`, `tomorrow`, `yesterday`
- Weekdays: `mon`, `monday`, `next monday`
- Relative: `in 3 days`, `in 2 weeks`
- Period markers: `next week`, `eow`, `eom`
- Ordinal days: `1st`, `15th`
- ISO format: `YYYY-MM-DD`

### Existing Time Parser (`mod.rs`)
- 24-hour: `9:00`, `14:30`
- 12-hour: `9am`, `2pm`, `9:30am`
- Ranges: `9am-11am`, `9:00-11:00`

---

## Feature 1: Relative Time Parsing

**Goal**: Parse "in 2 hours" or "this afternoon" for time blocks

### What Exists
- Relative date parsing works (`in 3 days`)
- Time-of-day not supported

### What Needs to Be Added

```rust
// Time-of-day keywords
fn parse_time_of_day(s: &str) -> Option<NaiveTime> {
    match s.to_lowercase().as_str() {
        "morning" => Some(NaiveTime::from_hms(9, 0, 0)),
        "mid-morning" => Some(NaiveTime::from_hms(10, 0, 0)),
        "afternoon" => Some(NaiveTime::from_hms(14, 0, 0)),
        "late afternoon" => Some(NaiveTime::from_hms(16, 0, 0)),
        "evening" => Some(NaiveTime::from_hms(18, 0, 0)),
        "night" => Some(NaiveTime::from_hms(20, 0, 0)),
        _ => None,
    }
}

// Relative time parsing
fn parse_relative_time(s: &str, now: NaiveTime) -> Option<NaiveTime> {
    // "in 2 hours" -> now + 2 hours
    // "in 30 minutes" -> now + 30 minutes
    static RE: LazyLock<Regex> = LazyLock::new(||
        Regex::new(r"^in\s+(\d+(?:\.\d+)?)\s*(hour|h|minute|min|m)s?$")
            .unwrap()
    );
    // ...
}
```

### Supported Phrases
| Input | Output |
|-------|--------|
| `morning` | 09:00 |
| `afternoon` | 14:00 |
| `in 2 hours` | now + 2h |
| `in 30 minutes` | now + 30m |
| `this afternoon` | 14:00-17:00 (range) |
| `tomorrow morning` | next_day 09:00-12:00 |

### Complexity: Low-Medium (2-3 hours)
- Straightforward keyword mapping
- Time arithmetic with `chrono`
- ~150-200 lines of code

---

## Feature 2: Conversational Input

**Goal**: Parse "Call John next Tuesday after 2pm" into structured task

### What Exists
- Structured metadata syntax only
- No free-form NLP extraction

### What Needs to Be Added

```rust
pub struct ConversationalParsed {
    pub action: Option<String>,        // "Call"
    pub subject: Option<String>,       // "John"
    pub entities: HashMap<String, String>,
    pub confidence: f32,
}

// Patterns to support
static ACTION_VERBS: &[&str] = &[
    "call", "meet", "contact", "email", "send",
    "review", "finish", "complete", "schedule", "book"
];

fn extract_action_verb(input: &str) -> Option<String> {
    // First word if it's a known action verb
}

fn extract_person_name(input: &str) -> Option<String> {
    // Pattern: "call|meet|with|contact X"
    // X = capitalized word(s)
}

fn extract_temporal_phrases(input: &str) -> Vec<String> {
    // Find date/time phrases using existing parser
}
```

### Conversational Patterns
```
"Call {person} {date} {time}"
"Meet with {person} on {date} at {time}"
"Review {thing} by {date}"
"Finish {task} in {duration}"
```

### Implementation Approaches

**Simple (Recommended Start)**:
- High-confidence patterns only
- Use existing date parser
- Extract capitalized words as names
- ~150-250 lines

**Advanced**:
- Part-of-speech tagging (optional `nlprule` crate)
- Entity recognition
- ~400-600 lines

### Complexity: High for full NLP, Medium for pattern-based (4-16 hours)

---

## Feature 3: Assignee Parsing

**Goal**: Support `@person` syntax for task assignment

### What Exists
- `@project` syntax implemented
- No `assigned_to` field on Task

### Problem: Ambiguity
`@john` - Is this a project named "john" or a person?

### Solutions

**Option A: Different Syntax**
```
@project      # Project (existing)
=>john        # Person (new)
assign:john   # Person (explicit)
owner:john    # Person (explicit)
```

**Option B: Context-Based Resolution**
```rust
if is_existing_project(&mention) {
    result.project_name = Some(mention);
} else {
    result.assigned_to = Some(mention);
}
```

**Option C: Namespaced**
```
@project:Work
@person:john
```

### Task Model Change
```rust
pub struct Task {
    // ... existing fields ...
    pub assigned_to: Option<String>,  // NEW
}
```

### Implementation
```rust
// In ParsedTask:
pub assigned_to: Option<String>,

// New regex (using =>)
static ASSIGNEE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"=>(\w+)|assign:(\w+)")
        .expect("valid regex"));

// In parse_quick_add():
if let Some(cap) = ASSIGNEE_RE.captures(input) {
    result.assigned_to = cap.get(1)
        .or_else(|| cap.get(2))
        .map(|m| m.as_str().to_string());
}
```

### Complexity: Low-Medium (2-4 hours)
- Add field to Task: ~10 lines
- Parsing logic: ~30-50 lines
- Tests: ~50-100 lines

---

## Implementation Priority

| Feature | Effort | Value | Recommendation |
|---------|--------|-------|----------------|
| Assignee Parsing | 2-4h | High | Start here (simple) |
| Relative Time Parsing | 2-3h | Medium | Second |
| Conversational (Simple) | 4-6h | Medium | Third |
| Conversational (Full NLP) | 8-16h | Low | Optional |

**Total Estimated Effort**: 8-13 hours (without full NLP)

---

## Syntax Decision Summary

### Recommended Quick-Add Syntax

| Syntax | Meaning | Example |
|--------|---------|---------|
| `#tag` | Add tag | `#work` |
| `!priority` | Set priority | `!high` |
| `due:date` | Due date | `due:friday` |
| `sched:date` | Scheduled date | `sched:monday` |
| `time:range` | Time block | `time:9am-11am` |
| `time:keyword` | Time of day | `time:afternoon` |
| `@project` | Project | `@Backend` |
| `=>person` | Assignee (NEW) | `=>john` |

### Example
```
Review PR =>john #code-review !high due:tomorrow time:afternoon @Backend
```

Parses to:
- Title: "Review PR"
- Assigned to: john
- Tags: code-review
- Priority: High
- Due: tomorrow
- Time block: 14:00
- Project: Backend

---

## Key Files to Modify

```
src/domain/task/mod.rs        # Add assigned_to field
src/app/quick_add/mod.rs      # Add assignee regex
src/app/quick_add/date.rs     # Extend for time-of-day
src/app/quick_add/time.rs     # NEW: Time parsing module
src/app/quick_add/conversational.rs  # NEW: Conversational parser
```

---

## Test Patterns

```rust
#[test]
fn test_parse_assignee() {
    let parsed = parse_quick_add("Task =>john");
    assert_eq!(parsed.assigned_to, Some("john".to_string()));
}

#[test]
fn test_parse_time_of_day() {
    let parsed = parse_quick_add("Meeting time:afternoon");
    assert_eq!(parsed.scheduled_start_time,
        Some(NaiveTime::from_hms(14, 0, 0)));
}

#[test]
fn test_parse_relative_time() {
    // "in 2 hours" from 10:00 = 12:00
    let time = parse_relative_time("in 2 hours",
        NaiveTime::from_hms(10, 0, 0));
    assert_eq!(time, Some(NaiveTime::from_hms(12, 0, 0)));
}
```

---

## See Also

- [FEATURE_IDEAS.md](FEATURE_IDEAS.md) - Feature overview
- `src/app/quick_add/tests/` - Existing test patterns
