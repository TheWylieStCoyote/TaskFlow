#![no_main]
//! Coverage-guided fuzzing of the whole expansion pipeline (parse + resolve +
//! compose + render). It must never panic; the engine's size budget keeps a
//! legitimate huge `*n` repeat from exhausting memory.
use libfuzzer_sys::fuzz_target;
use std::sync::OnceLock;

use codemmet_core::context::{ExpansionContext, IndentUnit};
use codemmet_core::engine::Engine;
use codemmet_core::lang::registry::Registry;
use codemmet_core::snippet::render::TabstopMode;

static ENGINE: OnceLock<Engine> = OnceLock::new();

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };
    let engine =
        ENGINE.get_or_init(|| Engine::with_registry(Registry::with_builtins().expect("builtins")));
    let ctx = ExpansionContext::plain(String::new(), IndentUnit::Spaces(4));
    let _ = engine.expand(input, "rust", &ctx, TabstopMode::Strip);
    let _ = engine.expand(input, "rust", &ctx, TabstopMode::Keep);
});
