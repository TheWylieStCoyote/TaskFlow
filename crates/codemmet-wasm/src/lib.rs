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

use codemmet_core::context::{ExpansionContext, IndentUnit};
use codemmet_core::engine::Engine;
use codemmet_core::lang::registry::Registry;
use codemmet_core::snippet::render::TabstopMode;

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

/// Expand `abbr` for `lang`.
///
/// `keep` selects snippet syntax (`1`) or plain stripped text (`0`). Returns a
/// pointer to a freshly allocated UTF-8 buffer whose length is [`cm_last_len`];
/// the caller must [`cm_free`] it.
///
/// # Safety
/// `abbr`/`lang` must point to `abbr_len`/`lang_len` readable UTF-8 bytes (as
/// written after [`cm_alloc`]).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cm_expand(
    abbr: *const u8,
    abbr_len: usize,
    lang: *const u8,
    lang_len: usize,
    keep: u32,
) -> *mut u8 {
    let abbr = unsafe { str_from(abbr, abbr_len) };
    let lang = unsafe { str_from(lang, lang_len) };
    let (ok, text) = expand(&abbr, &lang, keep != 0);

    LAST_OK.store(u32::from(ok), Ordering::Relaxed);
    let mut boxed = text.into_bytes().into_boxed_slice();
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
fn expand(abbr: &str, lang: &str, keep: bool) -> (bool, String) {
    let mode = if keep {
        TabstopMode::Keep
    } else {
        TabstopMode::Strip
    };
    let ctx = ExpansionContext::plain(String::new(), indent_for(lang));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_a_known_abbreviation() {
        let (ok, text) = expand("fn:main>pr", "rust", false);
        assert!(ok);
        assert!(text.contains("fn main()"));
        assert!(text.contains("println!"));
    }

    #[test]
    fn keep_mode_emits_snippet_syntax() {
        let (ok, text) = expand("pr", "rust", true);
        assert!(ok);
        assert!(text.contains('$'), "snippet syntax has a tabstop: {text}");
    }

    #[test]
    fn an_unknown_trigger_reports_an_error() {
        let (ok, text) = expand("definitely_not_real", "rust", false);
        assert!(!ok);
        assert!(!text.is_empty());
    }

    #[test]
    fn indent_falls_back_for_unknown_language() {
        assert_eq!(indent_for("klingon"), IndentUnit::Spaces(4));
    }
}
