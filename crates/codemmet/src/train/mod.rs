//! `codemmet train` — an interactive trainer for the abbreviation syntax.
//!
//! The trainer is split into a pure, testable core and a thin front end:
//!
//! - [`exercise`] generates exercises by composing real triggers and rendering
//!   them through the [`Engine`], so an exercise can never disagree with
//!   actual expansion.
//! - [`grade`] scores answers by output-equivalence.
//! - [`hint`] derives progressive hints from an exercise's concepts.
//! - [`review`] is the spaced-repetition schedule; [`progress`] persists it,
//!   completed lessons, and lifetime stats under the data directory.
//! - [`trainer`] is the pure state machine tying it together across modes.
//! - [`tui`] is the ratatui full-screen front end; [`session`] is the
//!   line-based fallback for non-TTY / piped use. Both drive [`trainer`].

// The pure trainer core lives in the `codemmet-train` library so it can also
// compile to WebAssembly for the browser playground. Re-export the modules the
// native front ends use under `crate::train::…` so their paths are unchanged.
#[cfg(test)]
use codemmet_train::rng;
use codemmet_train::{exercise, review, trainer};

mod progress;
mod session;
mod tui;

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use codemmet_core::engine::Engine;

use crate::cli::{EXIT_CONFIG, Sources, load_registry};
use exercise::Level;
use trainer::{Config, Mode, Stats, Trainer};

/// The trainer mode, as a CLI value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum ModeArg {
    /// Guided progression through the levels (default).
    Lessons,
    /// Endless randomized exercises.
    Drills,
    /// Only spaced-repetition items that are due.
    Review,
    /// Free play: type abbreviations and see them expand.
    Sandbox,
}

impl From<ModeArg> for Mode {
    fn from(value: ModeArg) -> Self {
        match value {
            ModeArg::Lessons => Self::Lessons,
            ModeArg::Drills => Self::Drills,
            ModeArg::Review => Self::Review,
            ModeArg::Sandbox => Self::Sandbox,
        }
    }
}

/// Arguments for the `train` subcommand.
#[derive(Debug, clap::Args)]
pub struct TrainArgs {
    /// Language to train (id or extension).
    #[arg(long, default_value = "rust")]
    lang: String,

    /// What to run.
    #[arg(long, value_enum, default_value_t = ModeArg::Lessons)]
    mode: ModeArg,

    /// Focus a single level (1-7); otherwise lessons cover all levels in order
    /// and drills mix them.
    #[arg(long, value_name = "N")]
    level: Option<u8>,

    /// Seed the exercise generator for a reproducible session.
    #[arg(long, value_name = "N")]
    seed: Option<u64>,

    /// Use the line-based interface instead of the full-screen TUI.
    #[arg(long)]
    plain: bool,

    /// Progress file (default: `<data-dir>/codemmet/trainer.json`).
    #[arg(long, value_name = "FILE")]
    data_file: Option<PathBuf>,

    /// Delete saved progress and exit.
    #[arg(long)]
    reset: bool,
}

/// Run the trainer.
pub fn run(sources: Sources<'_>, args: &TrainArgs) -> ExitCode {
    let data_path = args.data_file.clone().or_else(progress::default_path);

    if args.reset {
        return reset(data_path.as_deref());
    }

    let level = match parse_level(args.level) {
        Ok(level) => level,
        Err(code) => return code,
    };

    let registry = match load_registry(sources) {
        Ok(registry) => registry,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(EXIT_CONFIG);
        }
    };
    let engine = Engine::with_registry(registry);

    if engine.registry().language(&args.lang).is_none() {
        eprintln!("error: unknown language `{}`", args.lang);
        return ExitCode::from(EXIT_CONFIG);
    }

    let now = now_unix();
    let mut saved = data_path.as_deref().map(progress::load).unwrap_or_default();
    let config = Config {
        language: args.lang.clone(),
        mode: args.mode.into(),
        level,
        seed: args.seed.unwrap_or_else(clock_seed),
        now,
    };
    let review = saved.review_for(&args.lang);
    let mut trainer = Trainer::new(&engine, config, review);

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

    let due = trainer.review_state().due_count(now);
    persist(&mut saved, &args.lang, &trainer, data_path.as_deref());
    print_summary(trainer.stats(), due);
    ExitCode::SUCCESS
}

/// Parse `--level`, returning the level or an exit code on error.
fn parse_level(level: Option<u8>) -> Result<Option<Level>, ExitCode> {
    level.map_or(Ok(None), |n| {
        Level::from_number(n).map(Some).ok_or_else(|| {
            eprintln!("error: --level must be between 1 and 7");
            ExitCode::from(EXIT_CONFIG)
        })
    })
}

/// Handle `--reset`.
fn reset(path: Option<&std::path::Path>) -> ExitCode {
    let Some(path) = path else {
        eprintln!("error: no data directory; nothing to reset");
        return ExitCode::from(EXIT_CONFIG);
    };
    match progress::reset(path) {
        Ok(()) => {
            println!("progress reset ({})", path.display());
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: could not reset progress: {err}");
            ExitCode::from(EXIT_CONFIG)
        }
    }
}

/// Fold the finished session into `saved` and write it back (best effort).
fn persist(
    saved: &mut progress::Progress,
    language: &str,
    trainer: &Trainer,
    path: Option<&std::path::Path>,
) {
    let Some(path) = path else {
        return;
    };
    saved.lifetime.absorb(trainer.stats());
    saved.set_review(language, trainer.review_state().clone());
    saved.mark_completed(language, trainer.completed_levels());
    if let Err(err) = progress::save(path, saved) {
        eprintln!("warning: could not save progress: {err}");
    }
}

/// Print a one-line session receipt to stdout.
fn print_summary(stats: &Stats, due: usize) {
    let accuracy = if stats.attempts == 0 {
        0
    } else {
        (stats.solved * 100) / stats.attempts
    };
    let reviews = if due > 0 {
        format!(" · {due} due for review (run `--mode review`)")
    } else {
        String::new()
    };
    println!(
        "session: {} solved of {} shown · {accuracy}% of answers correct · best streak {}{reviews}",
        stats.solved, stats.presented, stats.best_streak
    );
}

/// The current time in Unix seconds (0 if the clock is before the epoch).
fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
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
