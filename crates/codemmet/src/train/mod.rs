//! `codemmet train` — an interactive trainer for the abbreviation syntax.
//!
//! The trainer is split into a pure, testable core and a thin front end:
//!
//! - [`exercise`] generates exercises by composing real triggers and rendering
//!   them through the [`Engine`](codemmet_core::engine::Engine), so an exercise
//!   can never disagree with actual expansion.
//! - [`grade`] scores answers by output-equivalence.
//! - [`hint`] derives progressive hints from an exercise's concepts.
//! - [`session`] is the line-based read-eval-print front end (T0/T1). The T2
//!   milestone adds a ratatui TUI over the same core.

mod exercise;
mod grade;
mod hint;
mod rng;
mod session;

use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use codemmet_core::context::ExpansionContext;
use codemmet_core::engine::Engine;

use crate::cli::{EXIT_CONFIG, Sources, load_registry};
use exercise::Level;
use session::Session;

/// Arguments for the `train` subcommand.
#[derive(Debug, clap::Args)]
pub struct TrainArgs {
    /// Language to train (id or extension).
    #[arg(long, default_value = "rust")]
    lang: String,

    /// Restrict to a single level (1-7); otherwise levels are mixed.
    #[arg(long, value_name = "N")]
    level: Option<u8>,

    /// Seed the exercise generator for a reproducible session.
    #[arg(long, value_name = "N")]
    seed: Option<u64>,
}

/// Run the trainer.
pub fn run(sources: Sources<'_>, args: &TrainArgs) -> ExitCode {
    let level = if let Some(n) = args.level {
        let Some(level) = Level::from_number(n) else {
            eprintln!("error: --level must be between 1 and 7");
            return ExitCode::from(EXIT_CONFIG);
        };
        Some(level)
    } else {
        None
    };

    let registry = match load_registry(sources) {
        Ok(registry) => registry,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(EXIT_CONFIG);
        }
    };
    let engine = Engine::with_registry(registry);

    let Some(lang) = engine.registry().language(&args.lang) else {
        eprintln!("error: unknown language `{}`", args.lang);
        return ExitCode::from(EXIT_CONFIG);
    };
    let ctx = ExpansionContext::plain(String::new(), lang.profile.default_indent.clone());
    let seed = args.seed.unwrap_or_else(clock_seed);

    let mut session = Session::new(&engine, args.lang.clone(), ctx, level, seed);
    let result = {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut input = stdin.lock();
        let mut output = stdout.lock();
        session.run(&mut input, &mut output)
    };
    if let Err(err) = result {
        eprintln!("error: {err}");
        return ExitCode::from(EXIT_CONFIG);
    }
    ExitCode::SUCCESS
}

/// A best-effort seed from the wall clock; falls back to a constant if the
/// clock is before the epoch.
fn clock_seed() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(
        0x9E37_79B9_7F4A_7C15,
        // Nanoseconds since the epoch, kept in `u64` without a lossy `u128` cast.
        |d| {
            d.as_secs()
                .wrapping_mul(1_000_000_000)
                .wrapping_add(u64::from(d.subsec_nanos()))
        },
    )
}
