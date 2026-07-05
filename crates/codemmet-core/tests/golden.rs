//! Golden snapshot suite over the builtin packs — the living spec.
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
//! Uses [`Registry::with_builtins`] + [`Engine::with_registry`] so the suite
//! never reads the developer's real config dir.

use std::sync::OnceLock;

use codemmet_core::Engine;
use codemmet_core::context::{ExpansionContext, IndentUnit, Scope};
use codemmet_core::lang::registry::Registry;
use codemmet_core::snippet::render::{CursorPos, TabstopMode};

/// The shared engine over builtin packs only (never `Engine::new()`).
fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        Engine::with_registry(Registry::with_builtins().expect("builtin packs should load"))
    })
}

/// The context every golden case renders with: no base indent, 4 spaces.
fn ctx() -> ExpansionContext {
    ExpansionContext::plain(String::new(), IndentUnit::Spaces(4))
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
    case_in(lang, abbr, &ctx())
}

/// Both modes of one case in an explicit context, for one snapshot.
fn case_in(lang: &str, abbr: &str, context: &ExpansionContext) -> String {
    format!(
        "== keep ==\n{}\n== strip ==\n{}",
        expand_in(lang, abbr, context, TabstopMode::Keep),
        expand_in(lang, abbr, context, TabstopMode::Strip),
    )
}

/// The `Display` of a failing expansion, for error-path snapshots.
fn expand_err(lang: &str, abbr: &str) -> String {
    engine()
        .expand(abbr, lang, &ctx(), TabstopMode::Keep)
        .expect_err("golden abbreviation should fail")
        .to_string()
}

macro_rules! golden {
    ($name:ident, $lang:literal, $abbr:literal) => {
        #[test]
        fn $name() {
            insta::assert_snapshot!(stringify!($name), case($lang, $abbr));
        }
    };
}

macro_rules! golden_err {
    ($name:ident, $lang:literal, $abbr:literal) => {
        #[test]
        fn $name() {
            insta::assert_snapshot!(stringify!($name), expand_err($lang, $abbr));
        }
    };
}

// ===== Rust =====

golden!(rust_fn_main_fori10_pr, "rust", "fn:main>fori10>pr");
golden!(rust_fn, "rust", "fn");
golden!(rust_fn_parse, "rust", "fn:parse");
golden!(rust_rfn_load, "rust", "rfn:load");
golden!(
    rust_main_letm_count_fori10_prd,
    "rust",
    "main>letm:count+fori10>prd"
);
golden!(rust_fori10, "rust", "fori10");
golden!(rust_forix, "rust", "forix");
golden!(rust_fori_idx_100, "rust", "fori:idx,100");
golden!(rust_fore_line, "rust", "fore:line");
golden!(rust_while, "rust", "while");
golden!(rust_wl, "rust", "wl");
golden!(rust_loop_ifl, "rust", "loop>ifl");
golden!(rust_if_el, "rust", "if+el");
golden!(rust_if_elif_el, "rust", "if+elif+el");
golden!(rust_match_cmd_arm3, "rust", "match:cmd>arm*3");
golden!(rust_matchres, "rust", "matchres");
golden!(rust_struct_point, "rust", "struct:Point");
golden!(rust_enum_op_arm, "rust", "enum:Op>arm");
golden!(rust_trait_render_fn_draw, "rust", "trait:Render>fn:draw");
golden!(rust_impl_point_fn_new, "rust", "impl:Point>fn:new");
golden!(rust_implt, "rust", "implt");
golden!(
    rust_mod_tests_test_roundtrip,
    "rust",
    "mod:tests>test:roundtrip"
);
golden!(rust_derive_struct_point, "rust", "derive+struct:Point");
golden!(rust_let_letm_letq, "rust", "let:x+letm:y+letq:z");
golden!(rust_const_max, "rust", "const:MAX");
golden!(rust_use_std_fmt, "rust", "use:\"std::fmt\"");
golden!(
    rust_tmod_test_it_works_aeq,
    "rust",
    "tmod>test:it_works>aeq"
);
golden!(rust_vec, "rust", "vec");
golden!(rust_hmap, "rust", "hmap");
golden!(rust_cl, "rust", "cl");
golden!(rust_doc_todo, "rust", "doc+todo");
golden!(rust_epr, "rust", "epr");

// ===== Python =====

golden!(py_def_handler_try, "python", "def:handler>try");
golden!(py_def, "python", "def");
golden!(py_adef_fetch, "python", "adef:fetch");
golden!(py_main, "python", "main");
golden!(py_ifmain_pr, "python", "ifmain>pr");
golden!(py_fori10_pr, "python", "fori10>pr");
golden!(py_forix, "python", "forix");
golden!(py_fore_row, "python", "fore:row");
golden!(py_while_if_el, "python", "while>if+el");
golden!(py_match_cmd_arm2, "python", "match:cmd>arm*2");
golden!(
    py_class_point_init_str_repr,
    "python",
    "class:Point>init+str+repr"
);
golden!(
    py_class_config_prop_name,
    "python",
    "class:Config>prop:name"
);
golden!(py_try_fin, "python", "try+fin");
golden!(py_with_pr, "python", "with>pr");
golden!(py_imp_os, "python", "imp:os");
golden!(py_from_pathlib, "python", "from:pathlib");
golden!(py_lam, "python", "lam");
golden!(py_lc, "python", "lc");
golden!(py_dc, "python", "dc");
golden!(py_test_parse_aeq, "python", "test:parse>aeq");
golden!(py_raise, "python", "raise");
golden!(py_doc, "python", "doc");
golden!(py_todo, "python", "todo");
golden!(py_prd, "python", "prd");

// ===== Contextual variants: composition scope (pure `>` nesting) =====

golden!(rust_impl_point_fn_new_ret, "rust", "impl:Point>fn:new>ret");
golden!(rust_impl_point_rfn_load, "rust", "impl:Point>rfn:load");
golden!(rust_impl_point_afn_run, "rust", "impl:Point>afn:run");
golden!(rust_fn_outer_fn_inner, "rust", "fn:outer>fn:inner");
golden!(rust_match_x_arm2, "rust", "match:x>arm*2");
golden!(py_class_config_def_load, "python", "class:Config>def:load");
golden!(py_class_a_adef_run, "python", "class:A>adef:run");

// ===== Contextual variants: buffer context =====

/// A context mimicking a cursor inside an `impl` block's body.
fn type_body_ctx() -> ExpansionContext {
    let mut context = ctx();
    context.scopes = vec![Scope::Root, Scope::TypeBody];
    context.raw_node_kinds = vec![
        "source_file".to_owned(),
        "impl_item".to_owned(),
        "declaration_list".to_owned(),
    ];
    context
}

#[test]
fn rust_fn_area_in_type_body_buffer() {
    insta::assert_snapshot!(
        "rust_fn_area_in_type_body_buffer",
        case_in("rust", "fn:area", &type_body_ctx())
    );
}

#[test]
fn ts_variant_fires_on_raw_node_kind() {
    use codemmet_core::config::loader::{CompiledSnippet, CompiledVariant, SnippetSet};
    use codemmet_core::snippet::render::render;
    use codemmet_core::snippet::template::parse_template;

    let set = SnippetSet::from_compiled([CompiledSnippet {
        trigger: "pr".to_owned(),
        description: None,
        params: Vec::new(),
        template: parse_template("println!(\"$0\");\n").expect("fixture body should parse"),
        scope: None,
        variants: vec![CompiledVariant {
            context: "ts:impl_item".to_owned(),
            template: parse_template("self.log(\"$0\");\n").expect("fixture body should parse"),
        }],
    }]);
    let abbr = codemmet_core::abbr::parse("pr").expect("abbreviation should parse");

    let fired = render(&abbr, &set, &type_body_ctx(), TabstopMode::Keep)
        .expect("abbreviation should render");
    assert_eq!(fired.body, "self.log(\"$0\");");

    // Without `impl_item` among the raw kinds, the default body renders.
    let plain = render(&abbr, &set, &ctx(), TabstopMode::Keep).expect("abbreviation should render");
    assert_eq!(plain.body, "println!(\"$0\");");
}

// ===== Error paths (snapshot the Display of the error) =====

golden_err!(err_unknown_trigger_with_suggestion, "rust", "forj");
golden_err!(err_unknown_trigger_no_suggestion, "rust", "zzzzz");
golden_err!(err_parse_dangling_child, "rust", "fn:main>");
golden_err!(err_too_many_args, "rust", "fn:a,b");
golden_err!(err_unknown_language, "cobol", "fn");
// `pr` declares no params, so a message argument cannot bind. Snapshotted as
// an error so the suite documents (rather than hides) this pack limitation.
golden_err!(err_rust_pr_with_message_arg, "rust", "pr:\"hello, world\"");

// ===== Cross-pack invariants =====

/// Shared-concept triggers the docs promise in every builtin pack.
const SHARED_TRIGGERS: &[&str] = &[
    "fori", "fore", "if", "el", "elif", "while", "match", "arm", "ret", "pr", "prd", "test",
    "main", "doc", "todo", "aeq",
];

#[test]
fn builtin_pack_consistency() {
    for lang in ["rust", "python"] {
        let set = &engine()
            .registry()
            .language(lang)
            .expect("builtin language should resolve")
            .set;
        for trigger in SHARED_TRIGGERS {
            assert!(
                set.get(trigger).is_some(),
                "pack `{lang}` is missing shared trigger `{trigger}`"
            );
        }
    }
}

#[test]
fn every_builtin_trigger_expands() {
    for lang in engine().registry().languages() {
        let id = lang.profile.id;
        for snippet in lang.set.iter() {
            for mode in [TabstopMode::Keep, TabstopMode::Strip] {
                let result = engine().expand(&snippet.trigger, id, &ctx(), mode);
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
