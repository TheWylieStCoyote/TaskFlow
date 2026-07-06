#![no_main]
//! Coverage-guided fuzzing of the abbreviation parser — the untrusted-input
//! surface. The invariant is simply that it never panics.
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = codemmet_core::abbr::parse(input);
    }
});
