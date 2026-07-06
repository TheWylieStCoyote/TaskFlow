//! The codemmet syntax trainer's pure core.
//!
//! Everything here is deterministic and free of I/O: exercise generation,
//! output-equivalence grading, progressive hints, spaced-repetition scheduling,
//! curated lesson content, and the [`Trainer`](trainer::Trainer) state machine
//! that ties them together (events in, renderable state out). Front ends — the
//! terminal TUI and line runner in the `codemmet` binary, and the WebAssembly
//! bindings for the browser playground — only render this state and forward
//! events, so the trainer behaves identically everywhere and stays in lockstep
//! with the real expansion engine.

pub mod exercise;
pub mod grade;
pub mod hint;
pub mod lessons;
pub mod review;
pub mod rng;
pub mod trainer;
