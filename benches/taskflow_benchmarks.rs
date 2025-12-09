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

// ============================================================================
// Storage Backend Benchmarks
// ============================================================================

use taskflow::storage::{create_backend, BackendType};
use tempfile::tempdir;

/// Create N tasks for storage benchmarks.
fn create_tasks(n: usize) -> Vec<Task> {
    (0..n)
        .map(|i| {
            let mut task = Task::new(format!("Task {}", i));
            if i % 5 == 0 {
                task.status = TaskStatus::Done;
            }
            if i % 3 == 0 {
                task.priority = Priority::High;
            }
            if i % 7 == 0 {
                task.tags.push("urgent".to_string());
            }
            task
        })
        .collect()
}

/// Benchmark storage backend write operations.
fn bench_storage_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_write");

    // JSON backend write
    for size in [10, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::new("json", size), size, |b, &size| {
            let tasks = create_tasks(size);
            b.iter_batched(
                || {
                    let dir = tempdir().unwrap();
                    let path = dir.path().join("bench.json");
                    let backend = create_backend(BackendType::Json, &path).unwrap();
                    (dir, backend, tasks.clone())
                },
                |(_dir, mut backend, tasks)| {
                    for task in &tasks {
                        backend.create_task(task).unwrap();
                    }
                    backend.flush().unwrap();
                    black_box(())
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    // SQLite backend write
    for size in [10, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::new("sqlite", size), size, |b, &size| {
            let tasks = create_tasks(size);
            b.iter_batched(
                || {
                    let dir = tempdir().unwrap();
                    let path = dir.path().join("bench.db");
                    let backend = create_backend(BackendType::Sqlite, &path).unwrap();
                    (dir, backend, tasks.clone())
                },
                |(_dir, mut backend, tasks)| {
                    for task in &tasks {
                        backend.create_task(task).unwrap();
                    }
                    backend.flush().unwrap();
                    black_box(())
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    // YAML backend write
    for size in [10, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::new("yaml", size), size, |b, &size| {
            let tasks = create_tasks(size);
            b.iter_batched(
                || {
                    let dir = tempdir().unwrap();
                    let path = dir.path().join("bench.yaml");
                    let backend = create_backend(BackendType::Yaml, &path).unwrap();
                    (dir, backend, tasks.clone())
                },
                |(_dir, mut backend, tasks)| {
                    for task in &tasks {
                        backend.create_task(task).unwrap();
                    }
                    backend.flush().unwrap();
                    black_box(())
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark storage backend read operations.
fn bench_storage_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_read");

    // JSON backend read
    for size in [10, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::new("json", size), size, |b, &size| {
            // Setup: create backend with tasks
            let dir = tempdir().unwrap();
            let path = dir.path().join("bench.json");
            let tasks = create_tasks(size);
            {
                let mut backend = create_backend(BackendType::Json, &path).unwrap();
                for task in &tasks {
                    backend.create_task(task).unwrap();
                }
                backend.flush().unwrap();
            }

            b.iter(|| {
                let backend = create_backend(BackendType::Json, &path).unwrap();
                let listed = backend.list_tasks().unwrap();
                black_box(listed.len())
            });
        });
    }

    // SQLite backend read
    for size in [10, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::new("sqlite", size), size, |b, &size| {
            let dir = tempdir().unwrap();
            let path = dir.path().join("bench.db");
            let tasks = create_tasks(size);
            {
                let mut backend = create_backend(BackendType::Sqlite, &path).unwrap();
                for task in &tasks {
                    backend.create_task(task).unwrap();
                }
                backend.flush().unwrap();
            }

            b.iter(|| {
                let backend = create_backend(BackendType::Sqlite, &path).unwrap();
                let listed = backend.list_tasks().unwrap();
                black_box(listed.len())
            });
        });
    }

    // YAML backend read
    for size in [10, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::new("yaml", size), size, |b, &size| {
            let dir = tempdir().unwrap();
            let path = dir.path().join("bench.yaml");
            let tasks = create_tasks(size);
            {
                let mut backend = create_backend(BackendType::Yaml, &path).unwrap();
                for task in &tasks {
                    backend.create_task(task).unwrap();
                }
                backend.flush().unwrap();
            }

            b.iter(|| {
                let backend = create_backend(BackendType::Yaml, &path).unwrap();
                let listed = backend.list_tasks().unwrap();
                black_box(listed.len())
            });
        });
    }

    group.finish();
}

/// Benchmark storage backend filtered queries.
fn bench_storage_filtered(c: &mut Criterion) {
    use taskflow::domain::Filter;

    let mut group = c.benchmark_group("storage_filtered");

    for size in [50, 100, 200].iter() {
        // JSON filtered query
        group.bench_with_input(
            BenchmarkId::new("json_filter_status", size),
            size,
            |b, &size| {
                let dir = tempdir().unwrap();
                let path = dir.path().join("bench.json");
                let tasks = create_tasks(size);
                {
                    let mut backend = create_backend(BackendType::Json, &path).unwrap();
                    for task in &tasks {
                        backend.create_task(task).unwrap();
                    }
                    backend.flush().unwrap();
                }

                let filter = Filter {
                    status: Some(vec![TaskStatus::Done]),
                    include_completed: true,
                    ..Default::default()
                };

                b.iter(|| {
                    let backend = create_backend(BackendType::Json, &path).unwrap();
                    let filtered = backend.list_tasks_filtered(&filter).unwrap();
                    black_box(filtered.len())
                });
            },
        );

        // SQLite filtered query
        group.bench_with_input(
            BenchmarkId::new("sqlite_filter_status", size),
            size,
            |b, &size| {
                let dir = tempdir().unwrap();
                let path = dir.path().join("bench.db");
                let tasks = create_tasks(size);
                {
                    let mut backend = create_backend(BackendType::Sqlite, &path).unwrap();
                    for task in &tasks {
                        backend.create_task(task).unwrap();
                    }
                    backend.flush().unwrap();
                }

                let filter = Filter {
                    status: Some(vec![TaskStatus::Done]),
                    include_completed: true,
                    ..Default::default()
                };

                b.iter(|| {
                    let backend = create_backend(BackendType::Sqlite, &path).unwrap();
                    let filtered = backend.list_tasks_filtered(&filter).unwrap();
                    black_box(filtered.len())
                });
            },
        );
    }

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
    bench_storage_write,
    bench_storage_read,
    bench_storage_filtered,
);

criterion_main!(benches);
