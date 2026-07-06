//! Completion: one snippet item for the whole abbreviation, plus
//! trigger-prefix discovery items for the segment being typed.

use codemmet_core::Engine;
use codemmet_core::abbr::AbbrNode;
use codemmet_core::context::{ContextProvider, DocSnapshot, ExpansionContext};
use codemmet_core::lang::registry::LanguageSnippets;
use codemmet_core::snippet::render::TabstopMode;
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionTextEdit,
    Documentation, InsertTextFormat, MarkupContent, MarkupKind, Range, TextEdit,
};

use super::line_index::{LineIndex, PositionEncoding};
use super::{CompletionMode, extract};

/// Maximum number of trigger-prefix discovery items offered.
const MAX_PREFIX_ITEMS: usize = 50;

/// Compute completion items for `offset` in `text`, or `None` for a null
/// response (no candidate, gated off, unknown language, or a cursor inside
/// a string literal or comment).
pub fn compute(
    engine: &Engine,
    provider: &mut dyn ContextProvider,
    text: &str,
    language_id: &str,
    offset: usize,
    mode: CompletionMode,
    encoding: PositionEncoding,
) -> Option<Vec<CompletionItem>> {
    if mode == CompletionMode::Off {
        return None;
    }
    let lang = engine.registry().language(language_id)?;
    let (start, candidate) = extract::candidate_at(text, offset)?;
    if mode == CompletionMode::OperatorsOnly && !extract::has_operator(candidate) {
        return None;
    }
    let doc = DocSnapshot {
        language: lang.profile.id.to_owned(),
        text: text.to_owned(),
    };
    let mut ctx = provider.context_at(&doc, offset);
    if ctx.in_string_or_comment {
        // No items at all inside strings/comments, prefix discovery included.
        return None;
    }
    // LSP snippet clients (VS Code, Neovim's `vim.snippet`) re-indent snippet
    // continuation lines from the insertion position, so a server-side base
    // indent would be applied twice. Render with an empty base indent;
    // nesting depth still uses the provider's indent unit.
    ctx.base_indent = String::new();

    let mut items = Vec::new();
    let index = LineIndex::new(text);
    let range = Range {
        start: index.position(text, start, encoding),
        end: index.position(text, offset, encoding),
    };
    if let Some(item) = expansion_item(engine, lang, candidate, range, &ctx) {
        items.push(item);
    }
    items.extend(prefix_items(lang, candidate));

    if items.is_empty() { None } else { Some(items) }
}

/// The single whole-abbreviation snippet item, when `candidate` parses and
/// every word resolves and renders.
fn expansion_item(
    engine: &Engine,
    lang: &LanguageSnippets,
    candidate: &str,
    range: Range,
    ctx: &ExpansionContext,
) -> Option<CompletionItem> {
    let ast = codemmet_core::abbr::parse(candidate).ok()?;
    if !ast.0.iter().all(|node| resolves(lang, node)) {
        return None;
    }

    let keep = engine
        .expand(candidate, lang.profile.id, ctx, TabstopMode::Keep)
        .ok()?;
    let strip = engine
        .expand(candidate, lang.profile.id, ctx, TabstopMode::Strip)
        .ok()?;
    let first_line = strip.body.lines().next().unwrap_or_default().to_owned();

    Some(CompletionItem {
        label: candidate.to_owned(),
        label_details: Some(CompletionItemLabelDetails {
            detail: None,
            description: Some(first_line),
        }),
        kind: Some(CompletionItemKind::SNIPPET),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("```{}\n{}\n```", lang.profile.id, keep.body),
        })),
        sort_text: Some(format!("0{candidate}")),
        filter_text: Some(candidate.to_owned()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        text_edit: Some(CompletionTextEdit::Edit(TextEdit {
            range,
            new_text: keep.body,
        })),
        ..CompletionItem::default()
    })
}

/// Whether every word in the node's subtree resolves to a trigger.
fn resolves(lang: &LanguageSnippets, node: &AbbrNode) -> bool {
    lang.set.resolve_word(&node.word).is_some()
        && node.children.iter().all(|child| resolves(lang, child))
}

/// Plain word items for registered triggers the final segment is a prefix of.
fn prefix_items(lang: &LanguageSnippets, candidate: &str) -> Vec<CompletionItem> {
    let Some(segment) = extract::final_segment(candidate) else {
        return Vec::new();
    };
    lang.set
        .iter()
        .filter(|snippet| snippet.trigger.starts_with(segment))
        .take(MAX_PREFIX_ITEMS)
        .map(|snippet| CompletionItem {
            label: snippet.trigger.clone(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: snippet.description.clone(),
            sort_text: Some(format!("1{}", snippet.trigger)),
            ..CompletionItem::default()
        })
        .collect()
}

#[cfg(test)]
// Expected texts like `${1:i}` are LSP snippet syntax, not format strings.
#[allow(clippy::literal_string_with_formatting_args)]
mod tests {
    use codemmet_core::context::{NullContextProvider, WrapSelection};
    use codemmet_core::lang::registry::Registry;
    use lsp_types::Position;

    use super::*;

    fn engine() -> Engine {
        Engine::with_registry(Registry::with_builtins().expect("builtins should load"))
    }

    fn complete(text: &str, language: &str, mode: CompletionMode) -> Option<Vec<CompletionItem>> {
        compute(
            &engine(),
            &mut NullContextProvider,
            text,
            language,
            text.len(),
            mode,
            PositionEncoding::Utf8,
        )
    }

    /// A provider that always reports a string/comment position.
    struct InStringProvider;

    impl ContextProvider for InStringProvider {
        fn context_at(&mut self, doc: &DocSnapshot, offset: usize) -> ExpansionContext {
            let mut ctx = NullContextProvider.context_at(doc, offset);
            ctx.in_string_or_comment = true;
            ctx
        }

        fn validate_wrap(
            &mut self,
            _doc: &DocSnapshot,
            range: std::ops::Range<usize>,
        ) -> WrapSelection {
            WrapSelection::Valid { range }
        }
    }

    #[test]
    fn full_abbreviation_yields_one_snippet_item() {
        let text = "fn main() {\n    fori10";
        let items = complete(text, "rust", CompletionMode::Always).expect("items expected");
        let snippets: Vec<_> = items
            .iter()
            .filter(|i| i.kind == Some(CompletionItemKind::SNIPPET))
            .collect();
        assert_eq!(snippets.len(), 1);
        let item = snippets[0];
        assert_eq!(item.label, "fori10");
        assert_eq!(item.sort_text.as_deref(), Some("0fori10"));
        let Some(CompletionTextEdit::Edit(edit)) = &item.text_edit else {
            panic!("expected a plain text edit");
        };
        assert_eq!(edit.range.start, Position::new(1, 4));
        assert_eq!(edit.range.end, Position::new(1, 10));
        // Continuation lines carry only relative depth: the snippet client
        // re-indents them from the insertion position, so no base indent.
        assert_eq!(edit.new_text, "for ${1:i} in 0..10 {\n    $0\n}");
    }

    #[test]
    fn prefix_discovery_lists_triggers() {
        let items = complete("fo", "rust", CompletionMode::Always).expect("items expected");
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"fori"), "labels: {labels:?}");
        assert!(labels.contains(&"fore"), "labels: {labels:?}");
        assert!(
            items
                .iter()
                .all(|i| i.kind == Some(CompletionItemKind::KEYWORD))
        );
    }

    #[test]
    fn unknown_language_and_empty_candidate_yield_none() {
        assert!(complete("fori10", "cobol", CompletionMode::Always).is_none());
        assert!(complete("   ", "rust", CompletionMode::Always).is_none());
        assert!(complete("zzzz", "rust", CompletionMode::Always).is_none());
    }

    #[test]
    fn operators_only_gates_bare_words() {
        assert!(complete("fori10", "rust", CompletionMode::OperatorsOnly).is_none());
        let items =
            complete("fn:main>pr", "rust", CompletionMode::OperatorsOnly).expect("items expected");
        assert!(
            items
                .iter()
                .any(|i| i.kind == Some(CompletionItemKind::SNIPPET))
        );
    }

    #[test]
    fn off_mode_yields_none() {
        assert!(complete("fori10", "rust", CompletionMode::Off).is_none());
    }

    #[test]
    fn string_or_comment_context_suppresses_all_items() {
        // `fori10` alone would yield a snippet item and prefix discovery;
        // a provider reporting a string/comment position suppresses both.
        let items = compute(
            &engine(),
            &mut InStringProvider,
            "fori10",
            "rust",
            6,
            CompletionMode::Always,
            PositionEncoding::Utf8,
        );
        assert!(items.is_none());
    }

    #[test]
    fn tab_indented_line_uses_tab_unit() {
        let text = "\tfori10>pr";
        let items = complete(text, "rust", CompletionMode::Always).expect("items expected");
        let snippet = items
            .iter()
            .find(|i| i.kind == Some(CompletionItemKind::SNIPPET))
            .expect("snippet item expected");
        let Some(CompletionTextEdit::Edit(edit)) = &snippet.text_edit else {
            panic!("expected a plain text edit");
        };
        // One tab of relative depth; the cursor line's own tab is not baked in.
        assert!(edit.new_text.contains("\n\tprintln!"), "{}", edit.new_text);
        assert!(!edit.new_text.contains("\n\t\t"), "{}", edit.new_text);
    }

    #[test]
    fn glued_arg_that_cannot_bind_offers_no_snippet_item() {
        // `pr10` resolves (`pr` + glued `10`) but has no int param to bind.
        let items = complete("pr10", "rust", CompletionMode::Always);
        assert!(
            items
                .into_iter()
                .flatten()
                .all(|i| i.kind != Some(CompletionItemKind::SNIPPET))
        );
    }
}
