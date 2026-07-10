#![no_main]
//! Coverage-guided fuzzing of the `.codemmet` file-format line classifier —
//! file mode's untrusted-input surface. The invariant is that classification
//! never panics, on any line of any input.
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        for (_line_no, classified) in codemmet_core::filefmt::lines(input) {
            let _ = classified;
        }
    }
});
