//! codemmet — Emmet-style abbreviation expansion for programming languages.
//!
//! One binary serving both the CLI (`expand`, `list`, `check`) and, from M2,
//! the LSP server (`lsp`) and `wrap`. See `PLAN.md`.

mod cli;
mod lsp;
#[cfg(feature = "train")]
mod train;

use std::process::ExitCode;

fn main() -> ExitCode {
    cli::run()
}
