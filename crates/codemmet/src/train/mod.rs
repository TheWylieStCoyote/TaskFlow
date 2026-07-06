//! `codemmet train` — an interactive trainer for the abbreviation syntax.
//!
//! The trainer is split into a pure, testable core and a thin front end:
//!
//! - [`exercise`] generates exercises by composing real triggers and rendering
//!   them through the [`Engine`](codemmet_core::engine::Engine), so an exercise
//!   can never disagree with actual expansion.
//! - [`grade`] scores answers by output-equivalence.
//! - [`hint`] derives progressive hints from an exercise's concepts.
//! - [`trainer`] is the pure state machine tying it together.
//! - [`tui`] is the ratatui full-screen front end; [`session`] is the
//!   line-based fallback for non-TTY / piped use. Both drive [`trainer`].

mod exercise;
mod grade;
mod hint;
mod rng;
mod session;
mod trainer;
mod tui;

use std::io::IsTerminal;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use codemmet_core::context::ExpansionContext;
use codemmet_core::engine::Engine;

use crate::cli::{EXIT_CONFIG, Sources, load_registry};
use exercise::Level;
use trainer::{Stats, Trainer};

/// Arguments for the `train` subcommand.
#[derive(Debug, clap::Args)]
pub struct TrainArgs {
    /// Language to train (id or extension).
    #[arg(long, default_value = "rust")]
    lang: String,

    /// Restrict to a single level (1-7); otherwise all levels are taught in order.
    #[arg(long, value_name = "N")]
    level: Option<u8>,

    /// Seed the exercise generator for a reproducible session.
    #[arg(long, value_name = "N")]
    seed: Option<u64>,

    /// Use the line-based interface instead of the full-screen TUI.
    #[arg(long)]
    plain: bool,
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

    let mut trainer = Trainer::new(&engine, args.lang.clone(), ctx, level, seed);

    // Full-screen TUI when attached to a terminal; the line runner otherwise
    // (piped input, redirected output, or an explicit `--plain`).
    let interactive =
        !args.plain && std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
    let result = if interactive {
        tui::run(&mut trainer)
    } else {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut input = stdin.lock();
        let mut output = stdout.lock();
        session::run_plain(&mut trainer, &mut input, &mut output)
    };

    if let Err(err) = result {
        eprintln!("error: {err}");
        return ExitCode::from(EXIT_CONFIG);
    }
    print_summary(trainer.stats());
    ExitCode::SUCCESS
}

/// Print a one-line session receipt to stdout.
fn print_summary(stats: &Stats) {
    let accuracy = if stats.attempts == 0 {
        0
    } else {
        (stats.solved * 100) / stats.attempts
    };
    println!(
        "session: {} solved of {} shown · {accuracy}% of answers correct · best streak {}",
        stats.solved, stats.presented, stats.best_streak
    );
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
