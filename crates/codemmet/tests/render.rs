//! End-to-end tests for `codemmet render` (file mode): real process, piped
//! stdin, real exit codes. See `FILE_MODE_PLAN.md`.

use std::io::Write as _;
use std::process::{Command, Output, Stdio};

mod common;
use common::scratch_dir;

/// A `Command` for the compiled `codemmet` binary.
fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_codemmet"))
}

/// Run the binary with `args` and `input` piped to stdin, pointing
/// `--config` at an empty scratch dir so the developer's real user snippets
/// never leak into assertions.
fn run_stdin(name: &str, args: &[&str], input: &str) -> Output {
    let dir = scratch_dir(name);
    let mut child = bin()
        .args(args)
        .args(["--config", dir.to_str().expect("scratch path is UTF-8")])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("binary should spawn");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(input.as_bytes())
        .expect("stdin should accept the source");
    child.wait_with_output().expect("binary should finish")
}

/// Render `input` with `codemmet render` plus `extra` flags.
fn render(name: &str, extra: &[&str], input: &str) -> Output {
    let mut args = vec!["render"];
    args.extend_from_slice(extra);
    run_stdin(name, &args, input)
}

fn code(output: &Output) -> i32 {
    output.status.code().expect("process should not be killed")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be UTF-8")
}

// ---------------------------------------------------------------------------
// The flagship, and the format basics
// ---------------------------------------------------------------------------

#[test]
fn render_flagship_sketch_is_exact() {
    let source = "#lang rust\n\
                  #repeat Msg=Quit,Move,Write\n\
                  \n\
                  use:std::fmt\n\
                  \n\
                  enum:Msg>variant*@Msg\n\
                  \n\
                  impl:Msg>fn:handle\n\
                  match:self>arm*@Msg\n";
    let out = render("flagship", &["--no-project"], source);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(
        stdout(&out),
        "use std::fmt;\n\
         \n\
         enum Msg {\n    Quit,\n    Move,\n    Write,\n}\n\
         \n\
         impl Msg {\n    fn handle(&self) {\n\n    }\n}\n\
         match self {\n    Quit => ,\n    Move => ,\n    Write => ,\n}\n"
    );
}

#[test]
fn render_drops_comments_and_leading_blanks_keeps_interior_blanks() {
    let source = "# a header note\n\n\n#lang rust\n\npr\n\n\npr\n";
    let out = render("blanks", &["--no-project"], source);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    // Blanks before any output vanish; the two interior blanks survive.
    assert_eq!(stdout(&out), "println!(\"\");\n\n\nprintln!(\"\");\n");
}

#[test]
fn render_leading_indent_is_per_line_base_indent() {
    let source = "#lang rust\nfori10>pr\n    pr\n\tpr\n";
    let out = render("indent", &["--no-project"], source);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(
        stdout(&out),
        "for i in 0..10 {\n    println!(\"\");\n}\n    println!(\"\");\n\tprintln!(\"\");\n"
    );
}

#[test]
fn render_right_trims_emitted_lines() {
    // `impl:Msg>fn:handle` leaves `$0` alone on an indented line; stripped,
    // that line must be empty rather than whitespace-only.
    let out = render(
        "trim",
        &["--no-project", "--lang", "rust"],
        "impl:Msg>fn:handle\n",
    );
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(
        stdout(&out),
        "impl Msg {\n    fn handle(&self) {\n\n    }\n}\n"
    );
}

#[test]
fn render_tolerates_crlf_input() {
    let out = render("crlf", &["--no-project"], "#lang rust\r\npr\r\n");
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out), "println!(\"\");\n");
}

#[test]
fn render_empty_input_is_empty_output() {
    let out = render("empty", &["--no-project"], "");
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out), "");
}

// ---------------------------------------------------------------------------
// Directives
// ---------------------------------------------------------------------------

#[test]
fn render_lang_directive_beats_lang_flag() {
    let out = render(
        "lang-priority",
        &["--no-project", "--lang", "go"],
        "#lang rust\npr\n",
    );
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out), "println!(\"\");\n");
}

#[test]
fn render_repeat_directive_applies_from_its_line_down() {
    let source = "#lang rust\narm*@E\n#repeat E=A,B\narm*@E\n";
    let out = render("repeat-down", &["--no-project"], source);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    // Before the directive: the single-copy no-data fallback; after: filled.
    assert_eq!(stdout(&out), "pattern => ,\nA => ,\nB => ,\n");
}

#[test]
fn render_repeat_directive_shadows_repeat_flag() {
    let out = render(
        "repeat-precedence",
        &["--no-project", "--lang", "rust", "--repeat", "E=A,B"],
        "#repeat E=X\narm*@E\n",
    );
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out), "X => ,\n");
}

#[test]
fn render_repeat_flag_seeds_collections() {
    let out = render(
        "repeat-seed",
        &["--no-project", "--lang", "rust", "--repeat", "E=A,B"],
        "arm*@E\n",
    );
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out), "A => ,\nB => ,\n");
}

#[test]
fn render_repeat_redeclaration_shadows_from_its_line() {
    let source = "#repeat E=A\narm*@E\n#repeat E=B,C\narm*@E\n";
    let out = render("repeat-shadow", &["--no-project", "--lang", "rust"], source);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out), "A => ,\nB => ,\nC => ,\n");
}

// ---------------------------------------------------------------------------
// Error paths: FILE:LINE diagnostics, exit codes, atomic (empty) stdout
// ---------------------------------------------------------------------------

#[test]
fn render_unknown_directive_is_a_line_numbered_parse_error() {
    let out = render(
        "unknown-directive",
        &["--no-project"],
        "#lang rust\n#repaet E=A\n",
    );
    assert_eq!(code(&out), 1);
    assert_eq!(stdout(&out), "", "stdout must stay empty on failure");
    let err = stderr(&out);
    assert!(err.contains("<stdin>:2:"), "stderr: {err}");
    assert!(err.contains("unknown directive `#repaet`"), "stderr: {err}");
}

#[test]
fn render_mid_file_failure_emits_zero_bytes() {
    // Line 1 expands fine; line 3 fails — nothing may reach stdout, so a
    // `>> main.rs` append leaves the target untouched.
    let out = render(
        "atomicity",
        &["--no-project", "--lang", "rust"],
        "pr\n\nzzznope\n",
    );
    assert_eq!(code(&out), 2);
    assert_eq!(stdout(&out), "", "stdout must stay empty on failure");
    let err = stderr(&out);
    assert!(err.contains("<stdin>:3:"), "stderr: {err}");
    assert!(err.contains("unknown trigger `zzznope`"), "stderr: {err}");
}

#[test]
fn render_abbreviation_parse_error_exits_1_with_line() {
    let out = render(
        "parse-error",
        &["--no-project", "--lang", "rust"],
        "pr\n(((\n",
    );
    assert_eq!(code(&out), 1);
    assert_eq!(stdout(&out), "");
    assert!(
        stderr(&out).contains("<stdin>:2:"),
        "stderr: {}",
        stderr(&out)
    );
}

#[test]
fn render_misplaced_and_duplicate_lang_exit_1() {
    let misplaced = render(
        "lang-late",
        &["--no-project", "--lang", "rust"],
        "pr\n#lang rust\n",
    );
    assert_eq!(code(&misplaced), 1);
    assert!(
        stderr(&misplaced).contains("<stdin>:2: `#lang` must appear before the first abbreviation"),
        "stderr: {}",
        stderr(&misplaced)
    );
    let duplicate = render("lang-dup", &["--no-project"], "#lang rust\n#lang go\npr\n");
    assert_eq!(code(&duplicate), 1);
    assert!(
        stderr(&duplicate).contains("<stdin>:2: duplicate `#lang`"),
        "stderr: {}",
        stderr(&duplicate)
    );
}

#[test]
fn render_unknown_lang_directive_exits_3() {
    let out = render("lang-unknown", &["--no-project"], "#lang cobol\npr\n");
    assert_eq!(code(&out), 3);
    assert!(
        stderr(&out).contains("<stdin>:1: unknown language `cobol`"),
        "stderr: {}",
        stderr(&out)
    );
}

#[test]
fn render_without_any_language_exits_3() {
    let out = render("lang-none", &["--no-project"], "pr\n");
    assert_eq!(code(&out), 3);
    assert!(
        stderr(&out).contains("cannot resolve a language"),
        "stderr: {}",
        stderr(&out)
    );
}

#[test]
fn render_malformed_repeat_directive_exits_1() {
    let out = render(
        "repeat-bad",
        &["--no-project", "--lang", "rust"],
        "#repeat nonsense\npr\n",
    );
    assert_eq!(code(&out), 1);
    assert!(
        stderr(&out).contains("<stdin>:1: #repeat"),
        "stderr: {}",
        stderr(&out)
    );
}

#[test]
fn render_tabstops_keep_is_rejected() {
    let out = render(
        "tabstops-keep",
        &["--no-project", "--lang", "rust", "--tabstops", "keep"],
        "pr\n",
    );
    assert_eq!(code(&out), 3);
    assert!(
        stderr(&out).contains("--tabstops keep is not supported in file mode"),
        "stderr: {}",
        stderr(&out)
    );
    let strip = render(
        "tabstops-strip",
        &["--no-project", "--lang", "rust", "--tabstops", "strip"],
        "pr\n",
    );
    assert_eq!(code(&strip), 0, "stderr: {}", stderr(&strip));
    assert_eq!(stdout(&strip), "println!(\"\");\n");
}

// ---------------------------------------------------------------------------
// CLI wiring: bare pipe, positional file, filename inference
// ---------------------------------------------------------------------------

#[test]
fn bare_codemmet_with_piped_stdin_is_the_filter() {
    // A truly bare argv is the trigger, so `--config` isolation is not
    // available; isolate via the environment instead (config lookup under
    // the scratch dir, cwd outside any project pack).
    let dir = scratch_dir("bare-pipe");
    let mut child = bin()
        .env("XDG_CONFIG_HOME", &dir)
        .env("HOME", &dir)
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("binary should spawn");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(b"#lang rust\nfori10>pr\n")
        .expect("stdin should accept the source");
    let out = child.wait_with_output().expect("binary should finish");
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out), "for i in 0..10 {\n    println!(\"\");\n}\n");
}

#[test]
fn positional_codemmet_file_renders_with_inferred_language() {
    let dir = scratch_dir("positional");
    let path = dir.join("main.rs.codemmet");
    std::fs::write(&path, "fori10>pr\n").expect("fixture should write");
    let path = path.to_str().expect("scratch path is UTF-8");
    // Both spellings: the top-level positional and the subcommand.
    for (name, args) in [
        ("top-level", vec![path]),
        ("subcommand", vec!["render", path]),
    ] {
        let out = run_stdin(&format!("positional-{name}"), &args, "");
        assert_eq!(code(&out), 0, "{name} stderr: {}", stderr(&out));
        assert_eq!(
            stdout(&out),
            "for i in 0..10 {\n    println!(\"\");\n}\n",
            "{name}"
        );
    }
}

#[test]
fn positional_without_codemmet_extension_is_a_usage_error() {
    let out = run_stdin("positional-bad", &["expnad"], "");
    assert_eq!(code(&out), 2);
    assert_eq!(stdout(&out), "");
    assert!(
        stderr(&out).contains("neither a subcommand nor a `.codemmet` file"),
        "stderr: {}",
        stderr(&out)
    );
}

#[test]
fn inference_with_unknown_inner_extension_exits_3() {
    let dir = scratch_dir("infer-unknown");
    let path = dir.join("main.foo.codemmet");
    std::fs::write(&path, "pr\n").expect("fixture should write");
    let out = run_stdin(
        "infer-unknown",
        &["render", path.to_str().expect("scratch path is UTF-8")],
        "",
    );
    assert_eq!(code(&out), 3);
    assert!(
        stderr(&out).contains("unknown language `foo`"),
        "stderr: {}",
        stderr(&out)
    );
}

#[test]
fn render_diagnostics_name_the_source_file() {
    let dir = scratch_dir("diag-name");
    let path = dir.join("bad.rs.codemmet");
    std::fs::write(&path, "pr\nzzznope\n").expect("fixture should write");
    let path = path.to_str().expect("scratch path is UTF-8");
    let out = run_stdin("diag-name", &["render", path], "");
    assert_eq!(code(&out), 2);
    assert!(
        stderr(&out).contains(&format!("{path}:2:")),
        "stderr: {}",
        stderr(&out)
    );
}

#[test]
fn render_missing_file_exits_3() {
    let out = run_stdin(
        "missing-file",
        &["render", "/nonexistent/x.rs.codemmet"],
        "",
    );
    assert_eq!(code(&out), 3);
    assert!(
        stderr(&out).contains("cannot read"),
        "stderr: {}",
        stderr(&out)
    );
}
