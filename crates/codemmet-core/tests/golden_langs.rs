//! Golden snapshot suite for the M4 language packs (C, C++, JavaScript,
//! TypeScript, Go), mirroring `tests/golden.rs`.
//!
//! Every case renders in BOTH tabstop modes into one snapshot:
//!
//! ```text
//! == keep ==
//! <LSP snippet syntax>
//! == strip ==
//! <plain text with `‸` marking the final cursor position>
//! ```
//!
//! Unlike `golden.rs`, cases render with each language's own default indent
//! (4 spaces for C/C++, 2 for JS/TS, tabs for Go), so the snapshots also
//! pin the per-language indentation defaults.

use std::sync::OnceLock;

use codemmet_core::Engine;
use codemmet_core::context::ExpansionContext;
use codemmet_core::lang::registry::Registry;
use codemmet_core::snippet::render::{CursorPos, TabstopMode};

/// The shared engine over builtin packs only (never `Engine::new()`).
fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        Engine::with_registry(Registry::with_builtins().expect("builtin packs should load"))
    })
}

/// The context a case renders with: no base indent, the language's default
/// indent unit.
fn ctx_for(lang: &str) -> ExpansionContext {
    let profile = codemmet_core::lang::profile::find(lang).expect("golden language should exist");
    ExpansionContext::plain(String::new(), profile.default_indent.clone())
}

/// Insert the cursor marker `‸` at `cursor` (byte column within its line).
fn insert_marker(body: &str, cursor: CursorPos) -> String {
    let line_no = usize::try_from(cursor.line).expect("line fits in usize");
    let col = usize::try_from(cursor.character).expect("column fits in usize");
    let mut out = String::with_capacity(body.len() + '‸'.len_utf8());
    for (i, line) in body.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        if i == line_no {
            out.push_str(&line[..col]);
            out.push('‸');
            out.push_str(&line[col..]);
        } else {
            out.push_str(line);
        }
    }
    out
}

/// Expand `abbr` for `lang` in `context`; in strip mode the cursor renders
/// as `‸`.
fn expand_in(lang: &str, abbr: &str, context: &ExpansionContext, mode: TabstopMode) -> String {
    let expansion = engine()
        .expand(abbr, lang, context, mode)
        .expect("golden abbreviation should expand");
    match (mode, expansion.cursor) {
        (TabstopMode::Strip, Some(cursor)) => insert_marker(&expansion.body, cursor),
        _ => expansion.body,
    }
}

/// Both modes of one case, formatted for a single snapshot.
fn case(lang: &str, abbr: &str) -> String {
    let context = ctx_for(lang);
    format!(
        "== keep ==\n{}\n== strip ==\n{}",
        expand_in(lang, abbr, &context, TabstopMode::Keep),
        expand_in(lang, abbr, &context, TabstopMode::Strip),
    )
}

/// The keep-mode expansion text, for non-snapshot assertions.
fn keep(lang: &str, abbr: &str) -> String {
    expand_in(lang, abbr, &ctx_for(lang), TabstopMode::Keep)
}

macro_rules! golden {
    ($name:ident, $lang:literal, $abbr:literal) => {
        #[test]
        fn $name() {
            insta::assert_snapshot!(stringify!($name), case($lang, $abbr));
        }
    };
}

// ===== C =====

golden!(c_fn_sum_fori10_prf, "c", "fn:sum>fori10>prf:\"%d\\n\",i");
golden!(c_main_pr, "c", "main>pr");
golden!(c_if_elif_el, "c", "if+elif+el");
golden!(c_switch_op_case2_default, "c", "switch:op>case*2+default");
golden!(c_struct_point, "c", "struct:Point");
golden!(c_guard_util_h, "c", "guard:UTIL_H");
golden!(c_malloc_buf_free_buf, "c", "mallocbuf+free:buf");
golden!(c_dowhile, "c", "dowhile");
golden!(c_fn_check_aeq, "c", "fn:check>aeq");

// ===== C++ (inherits C) =====

golden!(cpp_class_point_fn_area, "cpp", "class:Point>fn:area");
golden!(cpp_ns_geo_class_shape, "cpp", "ns:geo>class:Shape");
golden!(cpp_tpl_fn_max, "cpp", "tpl+fn:max");
golden!(cpp_fore_item_cout, "cpp", "fore:item>cout");
golden!(cpp_try_pr, "cpp", "try>pr");
golden!(cpp_vec_map, "cpp", "vec+map");
golden!(cpp_uptr, "cpp", "uptr");
golden!(cpp_enum_color, "cpp", "enum:Color");
golden!(cpp_lam, "cpp", "lam");

// ===== JavaScript =====

// The `fn` method variant fires via composition scope inside `class`.
golden!(js_class_app_fn_render, "javascript", "class:App>fn:render");
golden!(js_afn_load_try, "javascript", "afn:load>try");
golden!(js_fn_init_fori10_pr, "javascript", "fn:init>fori10>pr");
golden!(
    js_arrow_handle_fore_pr,
    "javascript",
    "arrow:handle>fore:item>pr"
);
golden!(
    js_switch_cmd_case2_default,
    "javascript",
    "switch:cmd>case*2+default"
);
golden!(
    js_class_user_ctor_get_set,
    "javascript",
    "class:User>ctor+get:name+set:name"
);
golden!(
    js_imp_react_impd_utils,
    "javascript",
    "imp:react+impd:\"./utils\""
);
golden!(js_test_parse_aeq, "javascript", "test:parse>aeq");
golden!(js_prom_aw, "javascript", "prom>aw");

// ===== TypeScript (inherits JavaScript) =====

golden!(ts_int_user, "typescript", "int:User");
// Overridden typed `fn` (replaces the JS body wholesale).
golden!(ts_fn_parse, "typescript", "fn:parse");
// Inherited `class` + the overridden `fn`'s re-declared type-body variant.
golden!(ts_class_api_fn_send, "typescript", "class:Api>fn:send");
golden!(
    ts_typ_result_enum_color,
    "typescript",
    "typ:Result+enum:Color"
);
golden!(ts_fng_identity, "typescript", "fng:identity");
golden!(ts_afn_fetch_try, "typescript", "afn:fetch>try");
golden!(ts_impt_types, "typescript", "impt:\"./types\"");

// ===== Go =====

golden!(go_fn_run_iferr, "go", "fn:run>iferr");
golden!(go_test_parse_aeq, "go", "test:Parse>aeq");
golden!(go_meth_string, "go", "meth:String");
golden!(go_fore_item_pr, "go", "fore:item>pr");
golden!(go_main_fori10_pr, "go", "main>fori10>pr");
golden!(go_switch_op_case2_default, "go", "switch:op>case*2+default");
golden!(
    go_struct_server_iface_handler,
    "go",
    "struct:Server+iface:Handler"
);
golden!(go_fn_serve_defer_go_pr, "go", "fn:serve>defer+go>pr");
golden!(go_iferr, "go", "iferr");

// ===== Inheritance invariants (asserted via expansion text, not snapshots) =====

#[test]
fn cpp_inherits_c_and_overrides() {
    let cpp = &engine()
        .registry()
        .language("cpp")
        .expect("cpp should resolve")
        .set;
    // Inherited from the C builtin pack.
    assert!(cpp.get("fori").is_some(), "cpp should inherit C's fori");
    assert!(cpp.get("guard").is_some(), "cpp should inherit C's guard");
    // Added by the C++ pack.
    assert!(cpp.get("class").is_some(), "cpp should define class");
    // Overridden triggers use the CHILD pack's body.
    assert!(
        keep("cpp", "enum:Color").contains("enum class Color"),
        "cpp enum should be the scoped-enum override"
    );
    assert!(
        keep("cpp", "struct:Point").starts_with("struct Point"),
        "cpp struct should be the plain-struct override"
    );
    // Deliberately NOT overridden: printing via pr stays printf.
    assert!(keep("cpp", "pr").contains("printf"));
    // C itself renders the parent bodies.
    assert!(keep("c", "enum:Color").contains("typedef enum"));
}

#[test]
fn typescript_inherits_javascript_and_overrides() {
    let ts = &engine()
        .registry()
        .language("typescript")
        .expect("typescript should resolve")
        .set;
    // Inherited from the JavaScript builtin pack.
    assert!(ts.get("arrow").is_some(), "ts should inherit js's arrow");
    assert!(ts.get("prom").is_some(), "ts should inherit js's prom");
    // Added by the TypeScript pack.
    assert!(ts.get("int").is_some(), "ts should define int");
    // Overridden `fn` uses the typed TS body...
    let ts_fn = keep("typescript", "fn:parse");
    assert!(ts_fn.contains("void"), "ts fn should be typed: {ts_fn}");
    // ...including the re-declared type-body method variant.
    let ts_method = keep("typescript", "class:Api>fn:send");
    assert!(
        ts_method.contains("send($") && ts_method.contains("void"),
        "ts method variant should be typed shorthand: {ts_method}"
    );
    // JavaScript itself keeps the untyped body.
    assert!(!keep("javascript", "fn:parse").contains("void"));
}

// ===== Cross-pack invariants =====

/// The shared trigger vocabulary, restricted to what each language's pack
/// can idiomatically express (kept explicit per language).
fn shared_triggers(lang: &str) -> &'static [&'static str] {
    match lang {
        "c" => &[
            "fn", "main", "fori", "while", "if", "el", "elif", "switch", "case", "struct", "ret",
            "pr", "doc", "todo", "aeq",
        ],
        // C's set plus what C++ adds; `main`/`while` etc. arrive via inheritance.
        "cpp" => &[
            "fn", "main", "fori", "fore", "while", "if", "el", "elif", "switch", "case", "struct",
            "class", "ret", "pr", "try", "lam", "doc", "todo", "aeq",
        ],
        "javascript" => &[
            "fn", "fori", "fore", "while", "if", "el", "elif", "switch", "case", "class", "ret",
            "pr", "prd", "imp", "try", "test", "doc", "todo", "aeq",
        ],
        // JavaScript's set plus the TS-only type formers.
        "typescript" => &[
            "fn", "fori", "fore", "while", "if", "el", "elif", "switch", "case", "class", "int",
            "typ", "enum", "ret", "pr", "prd", "imp", "try", "test", "doc", "todo", "aeq",
        ],
        // Go has no `while` (forc/for) and no exceptions (iferr instead of try).
        "go" => &[
            "fn", "main", "fori", "fore", "if", "el", "elif", "switch", "case", "struct", "let",
            "const", "ret", "pr", "prd", "imp", "test", "doc", "todo", "aeq",
        ],
        other => panic!("no shared-trigger list for `{other}`"),
    }
}

#[test]
fn m4_pack_consistency() {
    for lang in ["c", "cpp", "javascript", "typescript", "go"] {
        let set = &engine()
            .registry()
            .language(lang)
            .expect("builtin language should resolve")
            .set;
        for trigger in shared_triggers(lang) {
            assert!(
                set.get(trigger).is_some(),
                "pack `{lang}` is missing shared trigger `{trigger}`"
            );
        }
    }
}

#[test]
fn every_trigger_expands_in_every_language() {
    for lang in engine().registry().languages() {
        let id = lang.profile.id;
        let context = ctx_for(id);
        for snippet in lang.set.iter() {
            for mode in [TabstopMode::Keep, TabstopMode::Strip] {
                let result = engine().expand(&snippet.trigger, id, &context, mode);
                assert!(
                    result.is_ok(),
                    "bare trigger `{}` in `{id}` ({mode:?}) failed: {}",
                    snippet.trigger,
                    result.expect_err("checked is_ok above")
                );
            }
        }
    }
}
