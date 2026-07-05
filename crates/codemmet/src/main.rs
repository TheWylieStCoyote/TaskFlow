//! codemmet — Emmet-style abbreviation expansion for programming languages.
//!
//! One binary serving both the CLI (`expand`, `wrap`, `list`, `check`) and
//! the LSP server (`lsp`). Subcommands land in M1/M2; see `PLAN.md`.

use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "codemmet", version, about)]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
    println!(
        "codemmet {}: subcommands arrive in M1 — see PLAN.md",
        env!("CARGO_PKG_VERSION")
    );
}
