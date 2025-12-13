# Productivity & Focus Features - Analysis

## Overview

Analysis of productivity and focus features for TaskFlow, leveraging existing analytics, Pomodoro, and time tracking infrastructure.

---

## Current Infrastructure

| Component | Location | What It Provides |
|-----------|----------|------------------|
| Pomodoro System | `src/domain/pomodoro.rs` | Session tracking, cycles, streaks |
| Time Entries | `src/domain/time_entry.rs` | Start/end times, duration tracking |
| Analytics Engine | `src/domain/analytics.rs` | Completion trends, velocity, insights |
| Task Fields | `src/domain/task/mod.rs` | `estimated_minutes`, `actual_minutes`, `completed_at` |

---

## Feature 1: Focus Analytics

**Goal**: Track context switches during focus sessions, show "average focus duration before switching"

### What Exists
- `PomodoroSession` tracks work cycles with pause/resume
- `PomodoroStats` records cycles by date and streaks
- `TimeEntry` captures task work duration with timestamps

### What Needs to Be Added

```rust
// New types needed
pub struct FocusSession {
    pub id: FocusSessionId,
    pub task_id: TaskId,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub context_switches: Vec<ContextSwitch>,
    pub interruptions: Vec<Interruption>,
    pub total_focus_duration_secs: u32,
}

pub struct ContextSwitch {
    pub from_task_id: TaskId,
    pub to_task_id: TaskId,
    pub switched_at: DateTime<Utc>,
    pub reason: Option<String>,
}
```

### Analytics to Compute
- Average focus duration before switching
- Context switch frequency by task/project
- Focus quality score (fewer switches = better)
- Deep work threshold tracking (45+ min uninterrupted)

### Complexity: Medium-High (4-5 days)

---

## Feature 2: Smart Task Suggestions

**Goal**: Analyze completion patterns to suggest which tasks to work on next

### What Exists
- `ProductivityInsights.best_day` - most productive day of week
- `ProductivityInsights.peak_hour` - peak productivity hour (0-23)
- Task completion data with timestamps

### What Needs to Be Added

```rust
pub struct TaskSuggestion {
    pub task_id: TaskId,
    pub reason: SuggestionReason,
    pub score: f64,  // 0-100 confidence
    pub predicted_completion_time_mins: Option<u32>,
}

pub enum SuggestionReason {
    IsDueToday,
    HighPriority,
    BlockingOtherTasks,
    AlignedWithPeakHours,
    SimilarToRecentlyCompleted,
    ProgressDependency,
}
```

### Suggestion Algorithm
1. Filter tasks by due date, priority, status
2. Score by alignment with peak productivity hours
3. Boost score if similar to recently completed tasks
4. Consider task chains (`next_task_id`)
5. Factor in estimated duration

### Complexity: Medium (3-4 days)

---

## Feature 3: Task Effort Estimation

**Goal**: Suggest time estimates based on similar completed tasks

### What Exists (Already Implemented!)
- `EstimationAnalytics` in `src/domain/analytics.rs`
- `suggest_estimate()` method exists
- Per-project multiplier calculation
- Tag-based accuracy breakdown

### What Needs Enhancement

```rust
pub struct EstimationSuggestion {
    pub suggested_minutes: u32,
    pub confidence_level: f64,
    pub similar_tasks: Vec<TaskId>,
    pub historical_range: (u32, u32),  // min/max from similar
    pub reasoning: String,
}
```

### New Methods Needed
- `find_similar_tasks(task, limit)` - Match by tags, project, title similarity
- `estimate_from_similar(task)` - Calculate median actual time from similar tasks

### Complexity: Low-Medium (2-3 days) - Foundation already exists

---

## Feature 4: Burndown Prediction

**Goal**: Predict sprint/week burndown based on historical velocity

### What Exists
- `BurnChart` with scope/completed/ideal lines
- `VelocityMetrics` with weekly/monthly velocity, trends
- Per-project burndown tracking

### What Needs to Be Added

```rust
pub struct BurndownPrediction {
    pub predicted_completion_date: Option<NaiveDate>,
    pub confidence_level: f64,
    pub completion_scenarios: Vec<Scenario>,
    pub risk_factors: Vec<RiskFactor>,
}

pub enum Scenario {
    Optimistic,   // best_week velocity
    Realistic,    // avg_weekly velocity
    Pessimistic,  // worst_week velocity
}

pub enum RiskFactor {
    ScopeCreepDetected,
    DecliningVelocity,
    HighVariance,
    InsufficientData,
}
```

### New Analytics Methods
- `predict_completion_date(project_id, scenario)`
- `detect_scope_creep(project_id, window_days)`
- `analyze_velocity_stability()`

### Complexity: Medium (3-4 days)

---

## Feature 5: Workload Balance Dashboard

**Goal**: Track hours committed vs completed, alert on burnout risk

### What Exists
- `TimeAnalytics` with by_project, by_day_of_week, by_hour breakdowns
- Task fields: `estimated_minutes`, `actual_minutes`
- Dashboard with completion rates

### What Needs to Be Added

```rust
pub struct WorkloadMetrics {
    pub period: TimePeriod,
    pub hours_committed: f64,
    pub hours_completed: f64,
    pub hours_tracked: f64,
    pub completion_rate: f64,
    pub burnout_risk: BurnoutLevel,
}

pub enum BurnoutLevel {
    Low,       // 70-100% completion, <8h/day
    Moderate,  // 50-70% completion, 8-10h/day
    High,      // 30-50% completion, 10-12h/day
    Critical,  // <30% completion, >12h/day
}

pub struct WorkloadAlert {
    pub alert_type: AlertType,
    pub severity: Severity,
    pub message: String,
    pub recommended_action: String,
}
```

### Complexity: Medium (3-4 days)

---

## Implementation Priority

| Feature | Effort | Value | Risk | Recommendation |
|---------|--------|-------|------|----------------|
| Task Effort Estimation | 2-3 days | High | Low | Start here (foundation exists) |
| Smart Task Suggestions | 3-4 days | High | Low | Second priority |
| Focus Analytics | 4-5 days | Medium | Medium | Third |
| Workload Balance | 3-4 days | High | Medium | Fourth |
| Burndown Prediction | 3-4 days | Medium | Medium | Fifth |

**Total Estimated Effort**: 15-20 days

---

## Key Files to Modify

```
src/domain/analytics.rs          # Extend existing analytics types
src/app/analytics/estimation.rs  # Enhance estimation logic
src/app/analytics/focus.rs       # NEW: Focus session analytics
src/app/analytics/workload.rs    # NEW: Workload metrics
src/ui/components/dashboard/     # Add new panels
```

---

## See Also

- [FEATURE_IDEAS.md](FEATURE_IDEAS.md) - Feature overview
- [GITHUB_GITLAB_SYNC.md](GITHUB_GITLAB_SYNC.md) - Integration example
