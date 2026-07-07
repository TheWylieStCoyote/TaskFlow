//! WASM bindings for the codemmet browser playground.
//!
//! A tiny hand-rolled C-ABI (no `wasm-bindgen` toolchain required): JavaScript
//! allocates input buffers with [`cm_alloc`], writes UTF-8 into the exported
//! `memory`, calls [`cm_expand`], then reads the result (length from
//! [`cm_last_len`], success from [`cm_last_ok`]) and frees both buffers with
//! [`cm_free`]. See `playground/index.html` for the JavaScript side.
//!
//! This crate is a raw FFI boundary, so it opts out of the workspace-wide
//! `unsafe_code = "deny"`: exporting symbols and reading caller-provided
//! pointers is unavoidably unsafe here.
#![allow(unsafe_code)]

use std::slice;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use codemmet_core::context::{Collection, ExpansionContext, IndentUnit};
use codemmet_core::engine::Engine;
use codemmet_core::lang::registry::Registry;
use codemmet_core::snippet::render::TabstopMode;
use codemmet_train::exercise::{Direction, Level};
use codemmet_train::review::ReviewState;
use codemmet_train::trainer::{Config, Event, Mode, Phase, Trainer};
use serde::{Deserialize, Serialize};

/// The engine, built once from the compiled-in builtin packs.
fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        Engine::with_registry(Registry::with_builtins().expect("builtin packs compile in"))
    })
}

/// Length in bytes of the result from the most recent [`cm_expand`].
static LAST_LEN: AtomicUsize = AtomicUsize::new(0);
/// Whether the most recent [`cm_expand`] succeeded (1) or returned an error (0).
static LAST_OK: AtomicU32 = AtomicU32::new(1);

/// Allocate `size` bytes for JavaScript to write an input string into.
#[unsafe(no_mangle)]
pub extern "C" fn cm_alloc(size: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// Free a buffer previously returned by [`cm_alloc`] or [`cm_expand`].
///
/// # Safety
/// `ptr`/`size` must come from a prior `cm_alloc(size)` or a `cm_expand`
/// result with its `cm_last_len()`, and be freed at most once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cm_free(ptr: *mut u8, size: usize) {
    if !ptr.is_null() && size > 0 {
        // Reconstruct the allocation (capacity == size) and drop it.
        drop(unsafe { Vec::from_raw_parts(ptr, 0, size) });
    }
}

/// Length of the last expansion result, in bytes.
#[unsafe(no_mangle)]
pub extern "C" fn cm_last_len() -> usize {
    LAST_LEN.load(Ordering::Relaxed)
}

/// `1` if the last expansion succeeded, `0` if the result is an error message.
#[unsafe(no_mangle)]
pub extern "C" fn cm_last_ok() -> u32 {
    LAST_OK.load(Ordering::Relaxed)
}

/// Expand `abbr` for `lang`, optionally with data-driven collections.
///
/// `repeat` is a newline-separated list of `NAME=v1,v2,…` collection specs (the
/// browser equivalent of the CLI `--repeat` flag; each value may carry
/// `value:type`); pass an empty buffer for none. `keep` selects snippet syntax
/// (`1`) or plain stripped text (`0`). Returns a pointer to a freshly allocated
/// UTF-8 buffer whose length is [`cm_last_len`]; the caller must [`cm_free`] it.
///
/// # Safety
/// `abbr`/`lang`/`repeat` must point to `*_len` readable UTF-8 bytes (as
/// written after [`cm_alloc`]).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cm_expand(
    abbr: *const u8,
    abbr_len: usize,
    lang: *const u8,
    lang_len: usize,
    repeat: *const u8,
    repeat_len: usize,
    keep: u32,
) -> *mut u8 {
    let abbr = unsafe { str_from(abbr, abbr_len) };
    let lang = unsafe { str_from(lang, lang_len) };
    let repeat = unsafe { str_from(repeat, repeat_len) };
    let (ok, text) = expand(&abbr, &lang, &repeat, keep != 0);
    LAST_OK.store(u32::from(ok), Ordering::Relaxed);
    return_string(text)
}

/// Move `s` into a freshly allocated buffer JS reads (length in [`cm_last_len`])
/// and frees with [`cm_free`]; records the length.
fn return_string(s: String) -> *mut u8 {
    let mut boxed = s.into_bytes().into_boxed_slice();
    LAST_LEN.store(boxed.len(), Ordering::Relaxed);
    let ptr = boxed.as_mut_ptr();
    std::mem::forget(boxed);
    ptr
}

/// Copy `len` bytes at `ptr` into an owned `String` (lossy on invalid UTF-8).
unsafe fn str_from(ptr: *const u8, len: usize) -> String {
    if ptr.is_null() || len == 0 {
        return String::new();
    }
    let bytes = unsafe { slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(bytes).into_owned()
}

/// Expand, returning `(success, text)` — the expansion, or the error message.
///
/// `repeat_spec` supplies data-driven collections, one `NAME=values` per line;
/// malformed lines are ignored so a half-typed spec never fails the expansion.
fn expand(abbr: &str, lang: &str, repeat_spec: &str, keep: bool) -> (bool, String) {
    let mode = if keep {
        TabstopMode::Keep
    } else {
        TabstopMode::Strip
    };
    let mut ctx = ExpansionContext::plain(String::new(), indent_for(lang));
    for line in repeat_spec.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok((name, collection)) = Collection::parse_spec(line) {
            ctx.collections.insert(name, collection);
        }
    }
    match engine().expand(abbr, lang, &ctx, mode) {
        Ok(expansion) => (true, expansion.body),
        Err(err) => (false, err.to_string()),
    }
}

/// The language's default indent unit (spaces of four when unknown).
fn indent_for(lang: &str) -> IndentUnit {
    engine()
        .registry()
        .language(lang)
        .map_or(IndentUnit::Spaces(4), |language| {
            language.profile.default_indent.clone()
        })
}

// --- trainer -----------------------------------------------------------------

/// A trainer handle is an opaque pointer to a boxed [`Trainer`] bound to the
/// static engine. JavaScript holds it as an integer.
type Handle = *mut Trainer<'static>;

/// Config for [`cmt_new`], parsed from JSON.
#[derive(Deserialize)]
struct WasmConfig {
    lang: String,
    mode: String,
    level: Option<u8>,
    seed: u64,
    now: u64,
    /// The persisted spaced-repetition schedule (from localStorage), if any.
    review: Option<ReviewState>,
}

/// Create a trainer from a JSON config and return an opaque handle.
///
/// # Safety
/// `cfg`/`cfg_len` must point to readable UTF-8 (as written after
/// [`cm_alloc`]). The returned handle must be freed with [`cmt_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmt_new(cfg: *const u8, cfg_len: usize) -> Handle {
    let json = unsafe { str_from(cfg, cfg_len) };
    let cfg: WasmConfig = serde_json::from_str(&json).unwrap_or_else(|_| WasmConfig {
        lang: "rust".to_string(),
        mode: "lessons".to_string(),
        level: None,
        seed: 0,
        now: 0,
        review: None,
    });
    let mode = match cfg.mode.as_str() {
        "drills" => Mode::Drills,
        "review" => Mode::Review,
        "sandbox" => Mode::Sandbox,
        _ => Mode::Lessons,
    };
    let config = Config {
        language: cfg.lang,
        mode,
        level: cfg.level.and_then(Level::from_number),
        seed: cfg.seed,
        now: cfg.now,
    };
    let trainer = Trainer::new(engine(), config, cfg.review.unwrap_or_default());
    Box::into_raw(Box::new(trainer))
}

/// Feed an event to the trainer. `code`: 0 advance, 1 submit, 2 hint, 3 reveal,
/// 4 skip, 5 quit, 6 backspace, 7 insert (`ch` is the Unicode scalar).
///
/// # Safety
/// `handle` must be a live handle from [`cmt_new`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmt_event(handle: Handle, code: u32, ch: u32) {
    let Some(trainer) = (unsafe { handle.as_mut() }) else {
        return;
    };
    let event = match code {
        0 => Event::Advance,
        1 => Event::Submit,
        2 => Event::Hint,
        3 => Event::Reveal,
        4 => Event::Skip,
        5 => Event::Quit,
        6 => Event::Backspace,
        7 => match char::from_u32(ch) {
            Some(c) => Event::Insert(c),
            None => return,
        },
        _ => return,
    };
    trainer.handle(event);
}

/// The current trainer state as a JSON [`Snapshot`]. The returned buffer's
/// length is [`cm_last_len`]; free it with [`cm_free`].
///
/// # Safety
/// `handle` must be a live handle from [`cmt_new`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmt_state(handle: Handle) -> *mut u8 {
    let json = unsafe { handle.as_ref() }
        .map(|trainer| serde_json::to_string(&snapshot(trainer)).unwrap_or_default())
        .unwrap_or_default();
    return_string(json)
}

/// The trainer's spaced-repetition schedule as JSON (for persistence). The
/// returned buffer's length is [`cm_last_len`]; free it with [`cm_free`].
///
/// # Safety
/// `handle` must be a live handle from [`cmt_new`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmt_review(handle: Handle) -> *mut u8 {
    let json = unsafe { handle.as_ref() }
        .map(|trainer| serde_json::to_string(trainer.review_state()).unwrap_or_default())
        .unwrap_or_default();
    return_string(json)
}

/// Free a trainer handle from [`cmt_new`].
///
/// # Safety
/// `handle` must be a live handle from [`cmt_new`], freed at most once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmt_free(handle: Handle) {
    if !handle.is_null() {
        drop(unsafe { Box::from_raw(handle) });
    }
}

/// A renderable view of the trainer, serialized to JSON for the browser UI.
#[derive(Serialize)]
struct Snapshot {
    phase: &'static str,
    mode: &'static str,
    is_done: bool,
    level_num: u8,
    level_title: &'static str,
    header: String,
    blurb: Option<&'static str>,
    direction: Option<&'static str>,
    target: Option<String>,
    abbreviation: Option<String>,
    multiple_choice: bool,
    choices: Vec<String>,
    input: String,
    hints: Vec<String>,
    message: Option<String>,
    preview: Option<String>,
    feedback: Option<FeedbackView>,
    sandbox: Vec<SandboxEntry>,
    stats: StatsView,
    due: usize,
}

#[derive(Serialize)]
struct FeedbackView {
    solved: bool,
    revealed: bool,
    answer: String,
}

#[derive(Serialize)]
struct SandboxEntry {
    abbr: String,
    output: String,
}

#[derive(Serialize)]
struct StatsView {
    presented: u32,
    solved: u32,
    attempts: u32,
    streak: u32,
    best_streak: u32,
}

fn snapshot(trainer: &Trainer) -> Snapshot {
    let phase = match trainer.phase() {
        Phase::Intro(_) => "intro",
        Phase::Answering => "answering",
        Phase::Feedback(_) => "feedback",
        Phase::Sandbox => "sandbox",
        Phase::Done => "done",
    };
    let level = trainer.current_level();
    let exercise = trainer.exercise();
    let direction = exercise.map(|e| match e.direction {
        Direction::Reverse => "reverse",
        Direction::Forward => "forward",
    });
    let target = exercise
        .filter(|e| e.direction == Direction::Reverse)
        .map(|e| e.target.clone());
    let abbreviation = exercise
        .filter(|e| e.direction == Direction::Forward)
        .map(|e| e.abbreviation.clone());
    let feedback = match trainer.phase() {
        Phase::Feedback(f) => Some(FeedbackView {
            solved: f.solved,
            revealed: f.revealed,
            answer: f.answer.clone(),
        }),
        _ => None,
    };
    let stats = trainer.stats();
    Snapshot {
        phase,
        mode: trainer.mode().title(),
        is_done: trainer.is_done(),
        level_num: level.number(),
        level_title: level.title(),
        header: trainer.header_note(),
        blurb: matches!(trainer.phase(), Phase::Intro(_)).then(|| level.blurb()),
        direction,
        target,
        abbreviation,
        multiple_choice: exercise
            .is_some_and(codemmet_train::exercise::Exercise::is_multiple_choice),
        choices: exercise.map(|e| e.choices.clone()).unwrap_or_default(),
        input: trainer.input().to_string(),
        hints: trainer.revealed_hints().to_vec(),
        message: trainer.message().map(str::to_string),
        preview: trainer.live_preview(),
        feedback,
        sandbox: trainer
            .sandbox_log()
            .iter()
            .map(|(abbr, output)| SandboxEntry {
                abbr: abbr.clone(),
                output: output.clone(),
            })
            .collect(),
        stats: StatsView {
            presented: stats.presented,
            solved: stats.solved,
            attempts: stats.attempts,
            streak: stats.streak,
            best_streak: stats.best_streak,
        },
        due: trainer.due_count(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_a_known_abbreviation() {
        let (ok, text) = expand("fn:main>pr", "rust", "", false);
        assert!(ok);
        assert!(text.contains("fn main()"));
        assert!(text.contains("println!"));
    }

    #[test]
    fn keep_mode_emits_snippet_syntax() {
        let (ok, text) = expand("pr", "rust", "", true);
        assert!(ok);
        assert!(text.contains('$'), "snippet syntax has a tabstop: {text}");
    }

    #[test]
    fn an_unknown_trigger_reports_an_error() {
        let (ok, text) = expand("definitely_not_real", "rust", "", false);
        assert!(!ok);
        assert!(!text.is_empty());
    }

    #[test]
    fn data_driven_repeat_fills_from_the_repeat_spec() {
        let (ok, text) = expand("match:cmd>arm*@Msg", "rust", "Msg=Quit,Move,Write", false);
        assert!(ok, "text: {text}");
        assert_eq!(
            text,
            "match cmd {\n    Quit => ,\n    Move => ,\n    Write => ,\n}"
        );
        // A malformed spec line is ignored rather than failing the expansion.
        let (ok, _) = expand("pr", "rust", "no-equals-sign", false);
        assert!(ok);
    }

    #[test]
    fn indent_falls_back_for_unknown_language() {
        assert_eq!(indent_for("klingon"), IndentUnit::Spaces(4));
    }

    #[test]
    fn snapshot_tracks_the_trainer_across_phases() {
        let config = Config {
            language: "rust".to_string(),
            mode: Mode::Lessons,
            level: Level::from_number(3),
            seed: 1,
            now: 0,
        };
        let mut trainer = Trainer::new(engine(), config, ReviewState::default());

        let intro = snapshot(&trainer);
        assert_eq!(intro.phase, "intro");
        assert!(intro.blurb.is_some());

        trainer.handle(Event::Advance);
        let answering = snapshot(&trainer);
        assert_eq!(answering.phase, "answering");
        assert!(answering.direction.is_some());

        trainer.handle(Event::Reveal);
        let feedback = snapshot(&trainer);
        assert_eq!(feedback.phase, "feedback");
        assert!(feedback.feedback.is_some_and(|f| f.revealed));
    }
}
