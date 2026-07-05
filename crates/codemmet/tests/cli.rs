//! End-to-end tests for the `codemmet` binary: real process, real exit codes.

// Expected outputs like `${2:i}` look like format args to this lint; they
// are LSP snippet syntax, not formatting mistakes.
#![allow(clippy::literal_string_with_formatting_args)]

use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

/// A `Command` for the compiled `codemmet` binary.
fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_codemmet"))
}

/// A unique, existing scratch directory for one test.
fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("codemmet-cli-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir should be creatable");
    dir
}

/// Run the binary with `args`, pointing `--config` at an empty scratch dir so
/// the developer's real user snippets never leak into assertions.
fn run(name: &str, args: &[&str]) -> Output {
    let dir = scratch_dir(name);
    bin()
        .args(args)
        .args(["--config", dir.to_str().expect("scratch path is UTF-8")])
        .output()
        .expect("binary should run")
}

/// The process's exit code, defaulting the assertion message to the output.
fn code(output: &Output) -> i32 {
    output.status.code().expect("process should not be killed")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be UTF-8")
}

#[test]
fn expand_strip_is_default_and_exact() {
    let out = run(
        "expand-strip",
        &["expand", "--lang", "rust", "fn:main>fori10>pr"],
    );
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(
        stdout(&out),
        "fn main() {\n    for i in 0..10 {\n        println!(\"\");\n    }\n}\n"
    );
    assert_eq!(stderr(&out), "");
}

#[test]
fn expand_keep_emits_snippet_syntax() {
    let out = run(
        "expand-keep",
        &[
            "expand",
            "--lang",
            "rust",
            "--tabstops",
            "keep",
            "fn:main>fori10>pr",
        ],
    );
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(
        stdout(&out),
        "fn main($1) {\n    for ${2:i} in 0..10 {\n        println!(\"$0\");\n    }\n}\n"
    );
}

#[test]
fn expand_reads_abbreviation_from_stdin_dash() {
    let dir = scratch_dir("expand-stdin");
    let mut child = bin()
        .args(["expand", "--lang", "rust", "-"])
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
        .write_all(b"fori10>pr\n")
        .expect("stdin should accept the abbreviation");
    let out = child.wait_with_output().expect("binary should finish");
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out), "for i in 0..10 {\n    println!(\"\");\n}\n");
}

#[test]
fn expand_json_reports_text_cursor_and_snippet_flag() {
    let out = run("expand-json", &["expand", "--lang", "rust", "--json", "pr"]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    let text = stdout(&out);
    assert_eq!(text.lines().count(), 1, "JSON should be one line: {text}");
    let value: serde_json::Value = serde_json::from_str(&text).expect("stdout should be JSON");
    assert_eq!(value["text"], "println!(\"\");");
    assert_eq!(value["is_snippet"], false);
    assert_eq!(value["cursor"]["line"], 0);
    assert_eq!(value["cursor"]["character"], 10);
}

#[test]
fn expand_json_keep_has_null_cursor() {
    let out = run(
        "expand-json-keep",
        &[
            "expand",
            "--lang",
            "rust",
            "--json",
            "--tabstops",
            "keep",
            "pr",
        ],
    );
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    let value: serde_json::Value =
        serde_json::from_str(&stdout(&out)).expect("stdout should be JSON");
    assert_eq!(value["text"], "println!(\"$0\");");
    assert_eq!(value["is_snippet"], true);
    assert!(value["cursor"].is_null());
}

#[test]
fn expand_honors_indent_and_base_indent() {
    let out = run(
        "expand-indent",
        &[
            "expand",
            "--lang",
            "rust",
            "--indent",
            "\t",
            "--base-indent",
            "  ",
            "fn:main>pr",
        ],
    );
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out), "fn main() {\n  \tprintln!(\"\");\n  }\n");
}

#[test]
fn expand_parse_error_exits_1() {
    let out = run("expand-parse-err", &["expand", "--lang", "rust", ">pr"]);
    assert_eq!(code(&out), 1);
    assert_eq!(stdout(&out), "", "diagnostics must not reach stdout");
    let err = stderr(&out);
    assert!(err.contains("`>pr`"), "stderr was: {err}");
}

#[test]
fn expand_unknown_trigger_exits_2_with_suggestion() {
    let out = run(
        "expand-unknown-trigger",
        &["expand", "--lang", "rust", "forj"],
    );
    assert_eq!(code(&out), 2);
    assert_eq!(stdout(&out), "", "diagnostics must not reach stdout");
    let err = stderr(&out);
    assert!(err.contains("unknown trigger `forj`"), "stderr was: {err}");
    assert!(err.contains("did you mean"), "stderr was: {err}");
}

#[test]
fn expand_unknown_language_exits_3() {
    let out = run("expand-unknown-lang", &["expand", "--lang", "cobol", "pr"]);
    assert_eq!(code(&out), 3);
    assert!(
        stderr(&out).contains("unknown language `cobol`"),
        "stderr was: {}",
        stderr(&out)
    );
}

#[test]
fn expand_bad_indent_exits_3() {
    let out = run(
        "expand-bad-indent",
        &["expand", "--lang", "rust", "--indent", "ab", "pr"],
    );
    assert_eq!(code(&out), 3);
    assert!(
        stderr(&out).contains("--indent"),
        "stderr was: {}",
        stderr(&out)
    );
}

#[test]
fn list_plain_is_sorted_with_descriptions() {
    let out = run("list-plain", &["list", "--lang", "rust"]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    let text = stdout(&out);
    let triggers: Vec<&str> = text
        .lines()
        .map(|line| line.split_whitespace().next().unwrap_or(""))
        .collect();
    let mut sorted = triggers.clone();
    sorted.sort_unstable();
    assert_eq!(triggers, sorted, "listing should be sorted");
    let fori = text
        .lines()
        .find(|line| line.starts_with("fori"))
        .expect("fori should be listed");
    assert!(
        fori.contains("for loop over integer range"),
        "line was: {fori}"
    );
}

#[test]
fn list_json_is_an_array_of_trigger_objects() {
    let out = run("list-json", &["list", "--lang", "rust", "--json"]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    let value: serde_json::Value =
        serde_json::from_str(&stdout(&out)).expect("stdout should be JSON");
    let items = value.as_array().expect("output should be a JSON array");
    assert!(!items.is_empty());
    let pr = items
        .iter()
        .find(|item| item["trigger"] == "pr")
        .expect("pr should be listed");
    assert_eq!(pr["description"], "println! to stdout");
}

#[test]
fn expand_uses_config_override_directory() {
    let dir = scratch_dir("expand-override");
    std::fs::write(
        dir.join("rust.toml"),
        "[meta]\nlanguage = \"rust\"\n\n[snippets.zz]\nbody = \"zz($0)\"\n",
    )
    .expect("override pack should be writable");
    let out = bin()
        .args(["expand", "--lang", "rust", "zz"])
        .args(["--config", dir.to_str().expect("scratch path is UTF-8")])
        .output()
        .expect("binary should run");
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out), "zz()\n");
}

#[test]
fn check_valid_override_dir_exits_0() {
    let dir = scratch_dir("check-valid");
    std::fs::write(
        dir.join("rust.toml"),
        "[meta]\nlanguage = \"rust\"\n\n[snippets.zz]\nbody = \"zz($0)\"\n",
    )
    .expect("override pack should be writable");
    let out = bin()
        .args([
            "check",
            "--config",
            dir.to_str().expect("scratch path is UTF-8"),
        ])
        .output()
        .expect("binary should run");
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("7 languages"), "stdout was: {text}");
    assert!(text.contains("triggers"), "stdout was: {text}");
}

#[test]
fn check_invalid_toml_exits_3() {
    let dir = scratch_dir("check-invalid");
    std::fs::write(dir.join("rust.toml"), "not toml [").expect("override pack should be writable");
    let out = bin()
        .args([
            "check",
            "--config",
            dir.to_str().expect("scratch path is UTF-8"),
        ])
        .output()
        .expect("binary should run");
    assert_eq!(code(&out), 3);
    assert!(
        stderr(&out).contains("invalid snippet pack"),
        "stderr was: {}",
        stderr(&out)
    );
}

#[test]
fn check_missing_dir_exits_0_with_note() {
    let dir =
        std::env::temp_dir().join(format!("codemmet-cli-check-missing-{}", std::process::id()));
    let out = bin()
        .args([
            "check",
            "--config",
            dir.to_str().expect("scratch path is UTF-8"),
        ])
        .output()
        .expect("binary should run");
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert!(
        stdout(&out).contains("no user snippets at"),
        "stdout was: {}",
        stdout(&out)
    );
}
