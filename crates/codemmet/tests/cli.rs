//! End-to-end tests for the `codemmet` binary: real process, real exit codes.

// Expected outputs like `${2:i}` look like format args to this lint; they
// are LSP snippet syntax, not formatting mistakes.
#![allow(clippy::literal_string_with_formatting_args)]

use std::io::Write as _;
use std::process::{Command, Output, Stdio};

mod common;
use common::scratch_dir;

/// A `Command` for the compiled `codemmet` binary.
fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_codemmet"))
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
fn check_warns_about_misnamed_pack_files() {
    let dir = scratch_dir("check-misnamed");
    std::fs::write(
        dir.join("rs.toml"),
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
    assert!(stdout(&out).contains("ok:"), "stdout was: {}", stdout(&out));
    let err = stderr(&out);
    assert!(
        err.contains("does not match any language pack name"),
        "stderr was: {err}"
    );
    assert!(err.contains("rs.toml"), "stderr was: {err}");
}

#[test]
fn check_correctly_named_pack_emits_no_warning() {
    let dir = scratch_dir("check-well-named");
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
    assert_eq!(stderr(&out), "");
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

#[test]
fn project_pack_auto_loads_and_no_project_disables_it() {
    // A project pack under `.codemmet/snippets` overrides the builtin `pr`.
    let user = scratch_dir("proj-user");
    let project = scratch_dir("proj-root");
    let snippets = project.join(".codemmet").join("snippets");
    std::fs::create_dir_all(&snippets).expect("project snippets dir should be creatable");
    std::fs::write(
        snippets.join("rust.toml"),
        "[meta]\nlanguage = \"rust\"\n[snippets.pr]\nbody = \"PROJECT!($0);\"\n",
    )
    .expect("project pack should be writable");
    let nested = project.join("src");
    std::fs::create_dir_all(&nested).expect("nested dir should be creatable");
    let user_arg = user.to_str().expect("scratch path is UTF-8");

    // Run from a nested subdir: discovery walks up to the project pack.
    let out = bin()
        .current_dir(&nested)
        .args(["expand", "--lang", "rust", "pr", "--config", user_arg])
        .output()
        .expect("binary should run");
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out), "PROJECT!();\n", "project pack should win");

    // --no-project falls back to the builtin println!.
    let out = bin()
        .current_dir(&nested)
        .args([
            "expand",
            "--lang",
            "rust",
            "--no-project",
            "pr",
            "--config",
            user_arg,
        ])
        .output()
        .expect("binary should run");
    assert_eq!(
        stdout(&out),
        "println!(\"\");\n",
        "--no-project ignores the pack"
    );

    std::fs::remove_dir_all(&project).ok();
    std::fs::remove_dir_all(&user).ok();
}

/// Run `train` with `stdin` piped in, isolated from real user/project packs.
#[cfg(feature = "train")]
fn run_train(name: &str, args: &[&str], stdin_text: &str) -> Output {
    let dir = scratch_dir(name);
    let mut child = bin()
        .args(["train", "--no-project"])
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
        .expect("stdin is piped")
        .write_all(stdin_text.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("process should complete")
}

#[test]
#[cfg(feature = "train")]
fn train_runs_a_session_and_summarizes() {
    // Quitting immediately still prints a session summary and exits cleanly.
    let out = run_train(
        "train-quit",
        &["--lang", "rust", "--level", "1", "--seed", "1"],
        ":quit\n",
    );
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert!(
        stdout(&out).contains("session:"),
        "expected a summary, got:\n{}",
        stdout(&out)
    );
}

#[test]
#[cfg(feature = "train")]
fn train_rejects_an_out_of_range_level() {
    let out = run_train("train-bad-level", &["--level", "9"], "");
    assert_eq!(code(&out), 3);
    assert!(stderr(&out).contains("between 1 and 7"));
}

#[test]
fn expand_data_driven_repeat_fills_arms_from_flag() {
    let out = run(
        "expand-repeat",
        &[
            "expand",
            "--lang",
            "rust",
            "--no-project",
            "match:Msg>arm*",
            "--repeat",
            "Msg=Quit,Move,Write",
        ],
    );
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(
        stdout(&out),
        "match Msg {\n    Quit => ,\n    Move => ,\n    Write => ,\n}\n"
    );
}

#[test]
fn expand_data_driven_repeat_falls_back_without_data() {
    let out = run(
        "expand-repeat-fallback",
        &["expand", "--lang", "rust", "--no-project", "match:X>arm*"],
    );
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out), "match X {\n    pattern => ,\n}\n");
}

#[test]
fn expand_enum_with_variants_defines_and_data_fills() {
    // The definition side, symmetric with `match … > arm*`: `variant` nests in
    // `enum` and fills from a collection (falling back to a `Name` tabstop).
    let scaffold = run(
        "expand-variant-scaffold",
        &[
            "expand",
            "--lang",
            "rust",
            "--no-project",
            "enum:Color>variant*3",
        ],
    );
    assert_eq!(code(&scaffold), 0, "stderr: {}", stderr(&scaffold));
    assert_eq!(
        stdout(&scaffold),
        "enum Color {\n    Name,\n    Name,\n    Name,\n}\n"
    );

    let filled = run(
        "expand-variant-filled",
        &[
            "expand",
            "--lang",
            "rust",
            "--no-project",
            "enum:Color>variant*@Src",
            "--repeat",
            "Src=Red,Green,Blue",
        ],
    );
    assert_eq!(code(&filled), 0, "stderr: {}", stderr(&filled));
    assert_eq!(
        stdout(&filled),
        "enum Color {\n    Red,\n    Green,\n    Blue,\n}\n"
    );
}

// ---- capture-from-example (`codemmet capture`) --------------------------

/// Capture code into the `--config` (user) dir, then expand the new trigger
/// back: the round-trip reproduces the original.
#[test]
fn capture_into_user_dir_round_trips() {
    let src = "fn make() -> Widget {\n    Widget::new()\n}";
    let captured = run(
        "capture-roundtrip",
        &[
            "capture",
            "--lang",
            "rust",
            "--name",
            "mkw",
            "--user",
            "--no-project",
            src,
        ],
    );
    assert_eq!(code(&captured), 0, "stderr: {}", stderr(&captured));
    // `Widget` repeats, so it is generalized into one parameter.
    assert!(
        stdout(&captured).contains("captured `mkw` (1 parameter(s): Widget)"),
        "stdout: {}",
        stdout(&captured)
    );

    // Same scratch dir (keyed by test name): expand the captured trigger back.
    let expanded = run(
        "capture-roundtrip",
        &["expand", "--lang", "rust", "--no-project", "mkw"],
    );
    assert_eq!(code(&expanded), 0, "stderr: {}", stderr(&expanded));
    assert_eq!(
        stdout(&expanded),
        "fn make() -> Widget {\n    Widget::new()\n}\n"
    );
}

/// A glued identifier argument overrides the captured parameter's default, so
/// the same pattern stamps out a differently-named variant.
#[test]
fn capture_then_expand_with_glued_argument() {
    let src = "struct Widget {\n    id: u32,\n}\nimpl Widget {}";
    let captured = run(
        "capture-glued",
        &[
            "capture",
            "--lang",
            "rust",
            "--name",
            "mkw",
            "--user",
            "--no-project",
            src,
        ],
    );
    assert_eq!(code(&captured), 0, "stderr: {}", stderr(&captured));

    let expanded = run(
        "capture-glued",
        &["expand", "--lang", "rust", "--no-project", "mkwGadget"],
    );
    assert_eq!(code(&expanded), 0, "stderr: {}", stderr(&expanded));
    assert_eq!(
        stdout(&expanded),
        "struct Gadget {\n    id: u32,\n}\nimpl Gadget {}\n"
    );
}

/// Capturing an existing trigger conflicts (exit 3) unless `--force` is given.
#[test]
fn capture_conflict_needs_force() {
    let base = &[
        "capture",
        "--lang",
        "rust",
        "--name",
        "dup",
        "--user",
        "--no-project",
        "let a = a;",
    ];
    assert_eq!(code(&run("capture-conflict", base)), 0);

    let conflict = run("capture-conflict", base);
    assert_eq!(code(&conflict), 3, "stdout: {}", stdout(&conflict));
    assert!(
        stderr(&conflict).contains("already exists"),
        "stderr: {}",
        stderr(&conflict)
    );

    let forced = run(
        "capture-conflict",
        &[
            "capture",
            "--lang",
            "rust",
            "--name",
            "dup",
            "--user",
            "--no-project",
            "--force",
            "let b = b;",
        ],
    );
    assert_eq!(code(&forced), 0, "stderr: {}", stderr(&forced));
}

/// A pack written by `capture` passes `codemmet check`.
#[test]
fn captured_pack_passes_check() {
    let captured = run(
        "capture-check",
        &[
            "capture",
            "--lang",
            "rust",
            "--name",
            "logit",
            "--user",
            "--no-project",
            "--description",
            "log helper",
            "tracing::info!(\"x\");",
        ],
    );
    assert_eq!(code(&captured), 0, "stderr: {}", stderr(&captured));

    let checked = run("capture-check", &["check", "--no-project"]);
    assert_eq!(code(&checked), 0, "stderr: {}", stderr(&checked));
    assert!(
        stdout(&checked).starts_with("ok:"),
        "stdout: {}",
        stdout(&checked)
    );
}

/// Capturing an unknown language is a config error (exit 3).
#[test]
fn capture_unknown_language_is_config_error() {
    let out = run(
        "capture-unknown",
        &[
            "capture",
            "--lang",
            "cobol",
            "--name",
            "x",
            "--user",
            "--no-project",
            "y",
        ],
    );
    assert_eq!(code(&out), 3);
    assert!(
        stderr(&out).contains("unknown language"),
        "stderr: {}",
        stderr(&out)
    );
}
