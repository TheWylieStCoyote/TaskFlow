# Gamification Features - Analysis

## Overview

Analysis of gamification features for TaskFlow, including daily challenges, progress animations, and achievement systems.

---

## Current Infrastructure

| Component | Location | What It Provides |
|-----------|----------|------------------|
| Pomodoro System | `src/domain/pomodoro.rs` | Session tracking, cycles, streaks |
| Habit Tracking | `src/domain/habit.rs` | Check-ins, streak info |
| Analytics Engine | `src/domain/analytics.rs` | Completion trends, velocity, insights |
| Time Entries | `src/domain/time_entry.rs` | Duration tracking with timestamps |
| Dashboard | `src/ui/components/dashboard/` | Stats display with charts |

### Existing Streak Tracking
- `PomodoroStats.current_streak()` and `longest_streak`
- `ProductivityInsights.current_streak` and `longest_streak`
- Habit check-in history with `streaks: Vec<StreakInfo>`

---

## Feature 1: Daily Challenges

**Goal**: Auto-generated daily tasks that reward completion

### What Exists
- Pomodoro statistics with streak tracking
- `ProductivityInsights` with `avg_tasks_per_day`, `best_day`, `peak_hour`
- Recurrence patterns for recurring tasks
- Daily completion tracking in analytics engine

### What Needs to Be Added

```rust
pub struct Challenge {
    pub id: ChallengeId,
    pub title: String,
    pub description: Option<String>,
    pub challenge_type: ChallengeType,
    pub target: u32,            // e.g., complete 5 tasks
    pub reward_points: u32,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub difficulty: u8,         // 1-5
    pub is_active: bool,
}

pub enum ChallengeType {
    CompleteNTasks(u32),
    PomodoroCycles(u32),
    TimeTrackedMinutes(u32),
    MaintainStreak(u32),
    CompletePriorityTasks(Priority, u32),
}
```

### Challenge Generator
```rust
pub struct ChallengeEngine {
    fn generate_daily_challenges(&self, model: &Model) -> Vec<Challenge>;
    fn check_progress(&self, model: &Model) -> HashMap<ChallengeId, u32>;
    fn mark_completed(&mut self, challenge_id: ChallengeId);
}
```

### Model Extensions
```rust
pub active_challenges: Vec<Challenge>,
pub completed_challenges_today: HashSet<ChallengeId>,
pub challenge_progress: HashMap<ChallengeId, u32>,
```

### Key Files to Modify

| File | Change |
|------|--------|
| `src/domain/challenge.rs` (NEW) | Challenge, ChallengeType types |
| `src/app/challenge_engine.rs` (NEW) | Generation and progress logic |
| `src/app/model/mod.rs` | Add challenge state |
| `src/app/message/` | Add ChallengeMessage variant |
| `src/app/update/` | Handle challenge operations |
| Storage backends | Persist challenges |

### Complexity: Medium (25-35 hours)
- Core entity definition: 8-10 hours
- Challenge engine & generation: 8-10 hours
- Progress tracking: 4-6 hours
- UI for displaying challenges: 8-10 hours

---

## Feature 2: Progress Animations & Celebrations

**Goal**: Visual feedback for task completions and milestones

### What Exists
- Ratatui + Crossterm for TUI rendering
- Dashboard with charts (BarChart, status calculations)
- Focus view for single-task focus
- Message system with UiMessage for state changes
- Model has `status_message: Option<String>` for feedback
- Pomodoro already uses emoji: `🍅` (Work), `☕` (Break), `🌴` (Long Break)

### What Needs to Be Added

```rust
pub struct Animation {
    pub anim_type: AnimationType,
    pub start_time: Instant,
    pub duration_ms: u32,
    pub is_playing: bool,
}

pub enum AnimationType {
    TaskCompleted,         // Pulse/fade effect
    ChallengeUnlocked,     // Color transition
    StreakMilestone(u32),  // Progress bar animation
    AchievementEarned,     // Bounce/flash
    LevelUp,               // Color cycle + text
}

pub struct CelebrationEvent {
    pub event_type: CelebrationEventType,
    pub triggered_at: DateTime<Utc>,
    pub message: String,
    pub points_earned: u32,
}

pub enum CelebrationEventType {
    TaskCompleted,
    ChallengeCompleted(ChallengeId),
    StreakMilestone(u32),
    AchievementUnlocked(AchievementId),
    NewPersonalRecord,
}
```

### Animation Integration
```rust
// In Model
pub active_animations: Vec<Animation>,
pub celebration_queue: VecDeque<CelebrationEvent>,
pub current_frame: u32,
pub last_animation_time: Option<DateTime<Utc>>,

// New messages
SystemMessage::AnimationFrame
SystemMessage::TriggerCelebration(CelebrationEvent)
```

### Visual Effects (ASCII-based)
- Confetti particles: `*`, `+`, `✨`, `⭐`, `★`
- Screen flash: brief background color change
- Progress bar fill animation
- Toast notifications: slide in/out
- Milestone banners with Unicode borders

### Key Files to Modify

| File | Change |
|------|--------|
| `src/ui/animation.rs` (NEW) | Animation types and timing |
| `src/domain/celebration.rs` (NEW) | Celebration events |
| `src/ui/components/celebration_popup.rs` (NEW) | Overlay rendering |
| `src/app/model/mod.rs` | Animation/celebration state |
| `src/app/message.rs` | SystemMessage variants |
| `src/app/update.rs` | Frame rendering + celebration triggers |
| `src/ui/view.rs` | Integrate overlays into main render |
| `src/config/theme.rs` | Celebration colors/styles |

### Complexity: Medium (30-40 hours)
- Animation system core: 8-10 hours
- Celebration event pipeline: 4-6 hours
- UI components for celebrations: 10-12 hours
- Integration with update loop: 8-10 hours

---

## Feature 3: Achievement System

**Goal**: Unlock badges for productivity milestones

### What Exists
- Analytics engine with extensive metrics:
  - Completion trends, velocity (weekly/monthly)
  - `ProductivityInsights`: streaks, avg_tasks_per_day, best_day, peak_hour
  - Time analytics by project, day of week, hour
- `PomodoroStats`: total_cycles, total_work_mins, longest_streak
- Habit check-ins with historical data
- Task fields: `estimated_minutes`, `actual_minutes`, `completed_at`

### What Needs to Be Added

```rust
pub struct Achievement {
    pub id: AchievementId,
    pub key: String,           // e.g., "first_task_completed"
    pub name: String,
    pub description: String,
    pub icon: String,          // Emoji or ASCII art
    pub category: AchievementCategory,
    pub points: u32,
    pub rarity: Rarity,
    pub unlock_condition: UnlockCondition,
}

pub enum AchievementCategory {
    TaskMaster,     // Completion-based
    TimeKeeper,     // Time tracking
    PomodoroGuru,   // Pomodoro cycles
    StreakChampion, // Streak milestones
    Speedrunner,    // Task completion speed
    Organizer,      // Project/tag management
}

pub enum Rarity {
    Common,     // 10 pts
    Uncommon,   // 25 pts
    Rare,       // 50 pts
    Epic,       // 100 pts
    Legendary,  // 250 pts
}

pub enum UnlockCondition {
    CompleteNTasks(u32),
    MaintainStreak(u32),
    TotalTimeTracked(u32),      // minutes
    CompletePomodoroSessions(u32),
    AchieveProductivityLevel(f64), // completion rate %
    TasksCompletedOnTime(u32),
    MultiCondition(Vec<UnlockCondition>),
}
```

### Built-in Achievements
```
Task Master Tier:
  - FirstTask (complete 1 task, 10 pts)
  - Workhorse (complete 100 tasks, 100 pts)
  - TaskOverload (complete 500 tasks, Legendary)

Streak Champion Tier:
  - WeekWarrior (7 day streak, 50 pts)
  - MonthMaster (30 day streak, 200 pts)
  - YearYogi (365 day streak, Legendary)

Time Keeper Tier:
  - TimeTrackerBeginner (5 hours tracked, 25 pts)
  - TimeTrackerExpert (100 hours tracked, 150 pts)
  - TimeTrackerLegend (1000 hours tracked, Legendary)

Pomodoro Guru Tier:
  - PomodoroInitiate (10 cycles, 25 pts)
  - PomodoroAdept (100 cycles, 100 pts)
  - PomodoroMaster (500 cycles, 250 pts)

Speedrunner Tier:
  - QuickCompleter (complete task in <5 mins, 30 pts)
  - BulkCompletionist (complete 10 tasks in 1 day, 75 pts)

Perfectionist Tier:
  - OnTimeDelivery (complete 10 tasks before due, 50 pts)
  - NoOverdues (0 overdue tasks for 30 days, 100 pts)
```

### Achievement Engine
```rust
pub struct AchievementEngine {
    pub achievements: Vec<Achievement>,
    pub user_progress: HashMap<AchievementId, AchievementProgress>,
}

impl AchievementEngine {
    fn evaluate(&self, model: &Model) -> Vec<AchievementId>; // newly unlocked
    fn check_task_completion(&self, task: &Task) -> Vec<AchievementId>;
    fn check_streaks(&self, insights: &ProductivityInsights) -> Vec<AchievementId>;
    fn calculate_user_level(&self) -> (u32, u32, u32); // level, xp, next_threshold
}
```

### User Progress Tracking
```rust
pub struct AchievementProgress {
    pub achievement_id: AchievementId,
    pub unlocked_at: Option<DateTime<Utc>>,
    pub progress: u32,  // For partial progress
}

pub struct UserAchievementStats {
    pub total_points_earned: u32,
    pub current_level: u32,
    pub current_xp: u32,
    pub xp_threshold_for_next_level: u32,
    pub completed_achievements: Vec<AchievementId>,
}
```

### Key Files to Modify

| File | Change |
|------|--------|
| `src/domain/achievement.rs` (NEW) | Achievement, UnlockCondition types |
| `src/domain/badge.rs` (NEW) | Badge display types |
| `src/app/achievement_engine.rs` (NEW) | Evaluation logic |
| `src/app/model/mod.rs` | Add achievement state |
| `src/app/message.rs` | Add AchievementMessage variant |
| `src/app/update.rs` | Integrate checking after task completion |
| `src/app/analytics.rs` | Call engine after computing insights |
| `src/ui/components/dashboard/` | Display achievements section |
| Storage backends | Persist achievements |
| `src/config/theme.rs` | Achievement colors |

### Complexity: High (50-70 hours)
- Achievement definitions & registry: 8-10 hours
- Achievement engine & evaluation: 12-15 hours
- Progress tracking system: 8-10 hours
- UI components for display: 12-15 hours
- Storage integration: 8-10 hours
- Level/XP system: 4-6 hours

---

## TEA Message Flow for Gamification

```
User Action (Complete Task)
    ↓
Message::Task(TaskMessage::ToggleComplete)
    ↓
handle_task() in update.rs
    ↓
[NEW] Trigger Achievement Check
    ↓
AchievementEngine::evaluate(model)
    ↓
[NEW] Create AchievementMessage::UnlockAchievement
    ↓
[NEW] handle_achievement() → Update stats
    ↓
[NEW] Trigger Celebration Animation
    ↓
SystemMessage::TriggerCelebration(event)
    ↓
View renders celebration overlay
```

---

## Implementation Priority

| Feature | Effort | Value | Risk | Priority |
|---------|--------|-------|------|----------|
| Achievement System | 50-70h | High | Medium | 1st (foundation) |
| Progress Animations | 30-40h | Medium | Low | 2nd (builds on achievements) |
| Daily Challenges | 25-35h | Medium | Low | 3rd (uses achievement engine) |

**Total Estimated Effort**: 105-145 hours

---

## Quick Win: Leverage Existing Streaks

**Immediate value (8-12 hours):**
1. Display current/longest Pomodoro streak in Dashboard (already computed)
2. Show productivity insights with visual bars (BarChart exists)
3. Add milestone celebrations for streak milestones
4. Export streak data to achievements immediately

The Pomodoro streak and analytics streak systems are already 80% complete - just need UI and celebration layer!

---

## Recommended Implementation Order

### Phase 1: Foundation (5-7 days)
- Achievement domain model
- Achievement evaluator
- Storage persistence
- Basic UI display in dashboard

### Phase 2: Visual Feedback (3-4 days)
- Animation loop
- Celebration popups
- Integration with achievements

### Phase 3: Polish (3-4 days)
- Daily challenge generation
- Challenge rewards to points
- Level progression UI

### Phase 4: Dashboard Integration (2-3 days)
- Achievement showcase widget
- Level progress indicator
- Streak visualization

---

## Key Files Reference

**Analytics Foundation:**
- `src/app/analytics.rs` - 150+ lines of analytics computation
- `src/domain/analytics.rs` - ProductivityInsights struct
- `src/domain/pomodoro.rs` - PomodoroStats with streaks

**UI Framework:**
- `src/ui/components/dashboard.rs` - Dashboard widgets
- `src/ui/components/charts.rs` - BarChart and visualizations
- `src/config/theme.rs` - Color and style definitions

---

## See Also

- [FEATURE_IDEAS.md](FEATURE_IDEAS.md) - Feature overview
- [UI_UX_FEATURES.md](UI_UX_FEATURES.md) - Related: Progress visualizations
