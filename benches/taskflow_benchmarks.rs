//! Performance benchmarks for TaskFlow.
//!
//! Run with: `cargo bench`
//!
//! These benchmarks measure the performance of core operations
//! to help identify bottlenecks and track regressions.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use taskflow::app::analytics::AnalyticsEngine;
use taskflow::app::{update, Model};
use taskflow::domain::analytics::ReportConfig;
use taskflow::domain::{Priority, Task, TaskStatus};

/// Create a model with N tasks for benchmarking.
fn create_model_with_tasks(n: usize) -> Model {
    let mut model = Model::new();
    for i in 0..n {
        let mut task = Task::new(format!("Task {}", i));
        // Vary properties to make filtering more realistic
        if i % 5 == 0 {
            task.status = TaskStatus::Done;
        }
        if i % 3 == 0 {
            task.priority = Priority::High;
        }
        if i % 7 == 0 {
            task.tags.push("urgent".to_string());
        }
        if i % 11 == 0 {
            task.tags.push("work".to_string());
        }
        model.tasks.insert(task.id, task);
    }
    model.refresh_visible_tasks();
    model
}

/// Benchmark task filtering with different dataset sizes.
fn bench_task_filtering(c: &mut Criterion) {
    let mut group = c.benchmark_group("task_filtering");

    for size in [100, 500, 1000, 5000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut model = create_model_with_tasks(size);
            b.iter(|| {
                model.refresh_visible_tasks();
                black_box(model.visible_tasks.len())
            });
        });
    }

    group.finish();
}

/// Benchmark analytics report generation.
fn bench_analytics(c: &mut Criterion) {
    let mut group = c.benchmark_group("analytics");

    for size in [100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("generate_report", size),
            size,
            |b, &size| {
                let model = create_model_with_tasks(size);
                let engine = AnalyticsEngine::new(&model);
                let config = ReportConfig::last_n_days(30);

                b.iter(|| {
                    let report = engine.generate_report(black_box(&config));
                    black_box(report.status_breakdown.done)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark task creation and deletion.
fn bench_task_crud(c: &mut Criterion) {
    use taskflow::app::{Message, TaskMessage};

    let mut group = c.benchmark_group("task_crud");

    group.bench_function("create_task", |b| {
        let mut model = Model::new();
        let mut counter = 0;
        b.iter(|| {
            let msg = Message::Task(TaskMessage::Create(format!("Task {}", counter)));
            update(&mut model, msg);
            counter += 1;
            black_box(model.tasks.len())
        });
    });

    group.bench_function("toggle_complete", |b| {
        let mut model = create_model_with_tasks(100);
        model.selected_index = 0;
        b.iter(|| {
            let msg = Message::Task(TaskMessage::ToggleComplete);
            update(&mut model, msg.clone());
            black_box(model.selected_index)
        });
    });

    group.finish();
}

/// Benchmark navigation operations.
fn bench_navigation(c: &mut Criterion) {
    use taskflow::app::{Message, NavigationMessage};

    let mut group = c.benchmark_group("navigation");

    group.bench_function("move_down_1000_tasks", |b| {
        let mut model = create_model_with_tasks(1000);
        b.iter(|| {
            let msg = Message::Navigation(NavigationMessage::Down);
            update(&mut model, msg);
            // Wrap around at end
            if model.selected_index >= model.visible_tasks.len().saturating_sub(1) {
                model.selected_index = 0;
            }
            black_box(model.selected_index)
        });
    });

    group.bench_function("page_down_1000_tasks", |b| {
        let mut model = create_model_with_tasks(1000);
        b.iter(|| {
            let msg = Message::Navigation(NavigationMessage::PageDown);
            update(&mut model, msg);
            if model.selected_index >= model.visible_tasks.len().saturating_sub(1) {
                model.selected_index = 0;
            }
            black_box(model.selected_index)
        });
    });

    group.finish();
}

/// Benchmark search functionality.
fn bench_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");

    for size in [100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("filter_by_text", size),
            size,
            |b, &size| {
                let mut model = create_model_with_tasks(size);
                b.iter(|| {
                    model.filter.search_text = Some("Task 5".to_string());
                    model.refresh_visible_tasks();
                    black_box(model.visible_tasks.len())
                });
            },
        );
    }

    group.finish();
}

/// Benchmark undo/redo operations.
fn bench_undo_redo(c: &mut Criterion) {
    use taskflow::app::{Message, SystemMessage, TaskMessage};

    let mut group = c.benchmark_group("undo_redo");

    group.bench_function("undo_task_creation", |b| {
        b.iter_batched(
            || {
                let mut model = Model::new();
                // Create 10 tasks to have undo history
                for i in 0..10 {
                    let msg = Message::Task(TaskMessage::Create(format!("Task {}", i)));
                    update(&mut model, msg);
                }
                model
            },
            |mut model| {
                let msg = Message::System(SystemMessage::Undo);
                update(&mut model, msg);
                black_box(model.tasks.len())
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("redo_task_creation", |b| {
        b.iter_batched(
            || {
                let mut model = Model::new();
                for i in 0..10 {
                    let msg = Message::Task(TaskMessage::Create(format!("Task {}", i)));
                    update(&mut model, msg);
                }
                // Undo all
                for _ in 0..10 {
                    let msg = Message::System(SystemMessage::Undo);
                    update(&mut model, msg);
                }
                model
            },
            |mut model| {
                let msg = Message::System(SystemMessage::Redo);
                update(&mut model, msg);
                black_box(model.tasks.len())
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_task_filtering,
    bench_analytics,
    bench_task_crud,
    bench_navigation,
    bench_search,
    bench_undo_redo,
);

criterion_main!(benches);
