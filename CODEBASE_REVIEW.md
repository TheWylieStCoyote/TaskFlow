TaskFlow Code Improvements

 Status: Recommended improvements identified

 ---
 Summary

 | Category      | Issues    | Priority |
 |---------------|-----------|----------|
 | Performance   | 8 issues  | High     |
 | Code Quality  | 5 issues  | Medium   |
 | Testing Gaps  | 6 areas   | Medium   |
 | Documentation | 6 modules | Medium   |

 ---
 HIGH PRIORITY: Performance Issues

 1. O(n²) Hierarchy Traversal

 File: src/app/model/hierarchy.rs:105-124

 // Current: Scans ALL tasks for each node in hierarchy
 for (id, task) in &self.tasks {
     if task.parent_task_id.as_ref() == Some(&current_id) { ... }
 }

 Fix: Use pre-built TaskCache.children HashMap for O(1) lookups instead of linear scan.

 ---
 2. O(n²) Duplicate Detection

 File: src/domain/duplicate_detector.rs:76-106

 - Compares all task pairs even across different projects
 - Calls .to_lowercase() twice per comparison (allocation overhead)

 Fix:
 1. Pre-group tasks by project_id
 2. Only compare within groups
 3. Cache lowercase titles

 ---
 3. Nested Tag Filtering Loop

 File: src/app/model/filtering/matching.rs:38-51

 // Current: O(n*m) nested iteration + allocation per task
 let task_tags_lower: Vec<String> = task.tags.iter().map(|t| t.to_lowercase()).collect();
 filter_tags.iter().any(|ft| task_tags_lower.iter().any(|t| t == ft))

 Fix: Use HashSet for O(1) tag lookup; cache lowercase tags in FilterCache.

 ---
 4. Linear Scan for Task ID Lookup

 File: src/storage/backends/in_memory/task_repo.rs:11

 if self.data().tasks.iter().any(|t| t.id == task.id) { ... }

 Fix: Use HashMap<TaskId, Task> instead of Vec.

 ---
 5. Excessive Task Cloning for Undo

 File: src/app/update/task.rs:124-131

 let before = task.clone();  // Full clone
 task.toggle_complete();
 let after = task.clone();   // Second clone

 Fix: Use Rc<Task> or store deltas instead of full clones.

 ---
 6. String Allocations in SQLite Queries

 File: src/storage/backends/sqlite/task_repo.rs:200,215,223

 params.push(s.as_str().to_string());  // Unnecessary &str → String

 Fix: Use borrowed references with rusqlite::params![] macro.

 ---
 7. Missing Secondary Indexes

 File: src/app/model/mod.rs

 Common queries lack indexes:
 - Tasks by project_id
 - Tasks by tag
 - Tasks by due_date

 Fix: Add HashMap<ProjectId, Vec<TaskId>>, HashMap<String, Vec<TaskId>>.

 ---
 8. Frequent refresh_visible_tasks Calls

 Files: Multiple in src/app/update/

 Called 60+ times per update cycle.

 Fix: Batch updates, debounce refresh.

 ---
 MEDIUM PRIORITY: Code Quality

 1. Unsafe unwrap() in Lexer

 File: src/domain/filter_dsl/lexer.rs:222,235,262

 let ch = remaining.chars().next().unwrap();  // Can panic on malformed input

 Fix: Use proper Option handling or bounds checking.

 ---
 2. Long Function: handle_navigation()

 File: src/app/update/navigation.rs:17

 677 lines with #[allow(clippy::too_many_lines)].

 Fix: Split into view-specific handlers.

 ---
 3. Complex Sort Logic

 File: src/app/model/filtering/visibility.rs:65-118

 Large match on SortField duplicated in sort closure.

 Fix: Extract into impl SortField comparison methods.

 ---
 4. Repeated Clone Pattern

 File: src/app/update/task.rs

 Pattern appears 10+ times:
 let before = task.clone();
 // modify
 let after = task.clone();
 model.undo_stack.push(...)

 Fix: Create with_undo() helper function.

 ---
 5. Dead Code Annotations

 Files:
 - src/app/model/filtering/matching.rs:81
 - src/bin/taskflow/commands/pipe/types.rs:16,148,159

 Multiple #[allow(dead_code)] on unused functions.

 Fix: Remove or document why they exist.

 ---
 MEDIUM PRIORITY: Testing Gaps

 1. Filter DSL Parser

 File: src/domain/filter_dsl/parser.rs

 Missing property-based tests for:
 - Operator precedence edge cases
 - Date range boundaries
 - Malformed input handling

 ---
 2. Goal-KeyResult Integration

 Files: src/domain/goal.rs, src/domain/key_result.rs

 Missing:
 - Tests linking KeyResults to Goals
 - Progress calculation edge cases (0 target, division)

 ---
 3. Storage Stress Tests

 Files: src/storage/backends/

 Missing:
 - Large dataset tests (10K+ tasks)
 - Deep hierarchy tests
 - Performance benchmarks

 ---
 4. Export Round-trip Tests

 Files: src/storage/export/

 Missing:
 - CSV export → parse validation
 - ICS special character handling
 - DOT/Mermaid structure verification

 ---
 5. Habit Recurrence Edge Cases

 File: src/domain/habit/

 Missing:
 - Feb 29 on leap years
 - Monthly habits on 31st in short months

 ---
 6. Duplicate Detector Edge Cases

 File: src/domain/duplicate_detector.rs

 Missing:
 - Unicode/emoji in titles
 - Very long titles
 - Performance with large datasets

 ---
 DOCUMENTATION IMPROVEMENTS

 1. Analytics Module

 File: src/domain/analytics.rs

 800 lines with limited method-level doc comments.

 Needed:
 - Doc comments on CompletionTrend calculation methods
 - Examples for VelocityMetrics usage
 - BurnChart edge case documentation (0 scope, 0 completed)
 - Inline comments for complex aggregation logic

 ---
 2. Work Log Module

 File: src/domain/work_log.rs

 Needed:
 - Documentation on aggregation/reporting patterns
 - Examples for building summary reports
 - Usage patterns for chronological task journals

 ---
 3. Calendar Event Module

 File: src/domain/calendar_event.rs

 Needed:
 - Doc comments on CalendarEventStatus enum variants
 - All-day vs timed event differentiation
 - Integration examples with ICS import

 ---
 4. Pomodoro Module

 File: src/domain/pomodoro.rs

 Needed:
 - Doc comments on cycle_progress(), is_work_phase() methods
 - Configuration examples for custom intervals

 ---
 5. Goal/KeyResult Modules

 Files: src/domain/goal.rs, src/domain/key_result.rs

 Needed:
 - Doc comments on GoalStatus enum variants
 - progress_percent() calculation explanation
 - OKR workflow examples

 ---
 6. Tag Module

 File: src/domain/tag.rs

 Needed:
 - Doc comments on Tag struct fields
 - Context tag convention examples (@home, @work)
 - Color palette recommendations

 ---
 Quick Wins (Recommended First)

 | Fix                         | File          | Impact | Effort |
 |-----------------------------|---------------|--------|--------|
 | Use TaskCache for hierarchy | hierarchy.rs  | High   | Low    |
 | HashSet for tag matching    | matching.rs   | High   | Low    |
 | Fix lexer unwrap()          | lexer.rs      | Medium | Low    |
 | Cache lowercase tags        | matching.rs   | Medium | Low    |
 | Extract sort logic          | visibility.rs | Medium | Medium |
 | Add with_undo() helper      | task.rs       | Medium | Medium |

 ---
 Verification

 cargo test
 cargo clippy --all-targets
 cargo bench  # if benchmarks added
