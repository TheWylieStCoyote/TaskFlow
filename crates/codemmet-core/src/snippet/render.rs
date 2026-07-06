//! Composition and rendering: abbreviation AST + compiled snippets → text.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::abbr::{AbbrNode, Abbreviation};
use crate::config::loader::{CompiledSnippet, SnippetSet, parse_scope_name};
use crate::config::schema::ParamKind;
use crate::context::{ExpansionContext, Scope};
use crate::snippet::ir::{Span, Template};

/// Maximum number of composed lines per expansion.
///
/// Repeat counts multiply through nesting (`if*100>if*100>…`), so without a
/// budget a few dozen bytes of abbreviation can demand billions of lines.
/// The composer checks the running total after every template instantiation
/// and fails fast with [`RenderError::TooLarge`].
const MAX_COMPOSED_LINES: usize = 10_000;

/// How tabstops appear in the rendered output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabstopMode {
    /// Emit LSP snippet syntax (`${1:placeholder}`, `$0`).
    Keep,
    /// Emit plain text: placeholders become their text, markers vanish.
    Strip,
}

/// Cursor position within rendered plain text (0-based line/character).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPos {
    /// 0-based line within the expansion body.
    pub line: u32,
    /// 0-based UTF-8 byte column within that line.
    pub character: u32,
}

/// A rendered expansion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expansion {
    /// The expansion text. LSP snippet syntax when `is_snippet` is true.
    pub body: String,
    /// Whether `body` uses LSP snippet syntax ([`TabstopMode::Keep`]).
    pub is_snippet: bool,
    /// Where the cursor lands, for [`TabstopMode::Strip`] output.
    pub cursor: Option<CursorPos>,
}

/// A failure while resolving or composing an abbreviation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RenderError {
    /// A WORD did not resolve to any registered trigger.
    #[error("unknown trigger `{word}`{}", suggestion.as_deref().map(|s| format!(" (did you mean `{s}`?)")).unwrap_or_default())]
    UnknownTrigger {
        /// The unresolved WORD from the abbreviation.
        word: String,
        /// Closest registered trigger by edit distance, if any is close.
        suggestion: Option<String>,
    },
    /// Arguments could not be bound to the snippet's parameters.
    #[error("cannot bind arguments for `{trigger}`: {message}")]
    ArgBinding {
        /// The trigger whose arguments failed to bind.
        trigger: String,
        /// What went wrong.
        message: String,
    },
    /// Composition exceeded the expansion size budget.
    #[error("expansion too large (more than {limit} lines)")]
    TooLarge {
        /// The line budget that was exceeded (`MAX_COMPOSED_LINES`).
        limit: usize,
    },
}

/// Render a parsed abbreviation against a language's snippet set.
pub fn render(
    abbr: &Abbreviation,
    set: &SnippetSet,
    ctx: &ExpansionContext,
    mode: TabstopMode,
) -> Result<Expansion, RenderError> {
    let (lines, placeholders) = compose(abbr, set, ctx)?;
    let lines = renumber(lines, &placeholders);
    Ok(materialize(&lines, ctx, mode))
}

/// Render an abbreviation with `selection` wrapped at its final cursor.
///
/// Composition, tabstop renumbering, and materialization are identical to
/// [`render`]; the only difference is one IR rewrite in between. The
/// selection is dedented (common literal whitespace prefix of its non-empty
/// lines) and injected as plain literal text where the surviving final
/// cursor would land: the last `$0` candidate in document order, or — like
/// the renumbering fallback — the end of the last line when the composition
/// carries none. A fresh final cursor follows the selection's last
/// character, so `$0` (keep mode) or [`Expansion::cursor`] (strip mode)
/// sits immediately after the wrapped text.
///
/// The selection's first line splices inline at the injection point;
/// subsequent lines become new lines at the injection line's depth (blank
/// selection lines stay blank). Because the selection enters the IR as
/// literal text, keep-mode escaping, strip-mode passthrough, mirrors, and
/// base-indent handling all behave exactly as in normal rendering.
pub fn render_with_selection(
    abbr: &Abbreviation,
    set: &SnippetSet,
    ctx: &ExpansionContext,
    mode: TabstopMode,
    selection: &str,
) -> Result<Expansion, RenderError> {
    let (lines, placeholders) = compose(abbr, set, ctx)?;
    let lines = inject_selection(lines, selection);
    let lines = renumber(lines, &placeholders);
    Ok(materialize(&lines, ctx, mode))
}

/// Compose an abbreviation into the line IR plus its placeholder table.
fn compose(
    abbr: &Abbreviation,
    set: &SnippetSet,
    ctx: &ExpansionContext,
) -> Result<(Vec<Line>, Vec<Option<String>>), RenderError> {
    let mut composer = Composer {
        set,
        raw_node_kinds: &ctx.raw_node_kinds,
        placeholders: Vec::new(),
        composed_lines: 0,
    };
    // A top-level node lives in the buffer's innermost scope.
    let scope = ctx.scopes.last().copied().unwrap_or(Scope::Root);
    let mut lines = Vec::new();
    for node in &abbr.0 {
        lines.extend(composer.compose_node(node, scope)?);
    }
    Ok((lines, composer.placeholders))
}

/// A tabstop identity: an index into the composer's placeholder table.
///
/// Every tabstop the user can jump to gets exactly one identity; mirrors of
/// it reference the same identity.
type StopId = usize;

/// One line of composed output, before global renumbering.
#[derive(Debug)]
struct Line {
    /// Indent depth in `indent_unit`s, relative to the expansion root.
    depth: u32,
    /// The line's spans, in order.
    spans: Vec<CSpan>,
}

/// A span after instantiation (params bound, identities allocated).
#[derive(Debug)]
enum CSpan {
    /// Literal text, unescaped.
    Text(String),
    /// An occurrence of a tabstop identity; `mirror` for repeats.
    Stop { id: StopId, mirror: bool },
    /// A `$0` candidate; renumbering keeps only the last one.
    Final,
    /// A `${child}` slot; eliminated by [`Composer::compose_node`].
    Slot,
}

/// How one declared parameter is filled for a node.
#[derive(Debug)]
enum Binding {
    /// The abbreviation supplied a value: renders as literal text.
    Bound(String),
    /// No value: renders as a tabstop with the default as placeholder.
    Free {
        /// The parameter's declared default, if any.
        default: Option<String>,
    },
}

/// Depth-first composer: resolves triggers and builds the merged line IR.
struct Composer<'a> {
    /// The active language's triggers.
    set: &'a SnippetSet,
    /// Raw syntax-node kinds at the expansion point (`ts:` variant matching).
    raw_node_kinds: &'a [String],
    /// Placeholder text per allocated tabstop identity.
    placeholders: Vec<Option<String>>,
    /// Running count of instantiated lines, checked against
    /// [`MAX_COMPOSED_LINES`]. Splicing only ever merges lines, so this is
    /// an upper bound on the final line count.
    composed_lines: usize,
}

impl Composer<'_> {
    /// Allocate a fresh tabstop identity with the given placeholder.
    fn alloc(&mut self, placeholder: Option<String>) -> StopId {
        self.placeholders.push(placeholder);
        self.placeholders.len() - 1
    }

    /// Compose one abbreviation node (including `*n` copies and children).
    ///
    /// `scope` is the node's effective innermost scope: the buffer scope for
    /// a top-level node, or the enclosing snippet's declared `scope` (a
    /// scope-less parent is transparent) for a `>`-nested one. It picks the
    /// context variant to instantiate.
    fn compose_node(&mut self, node: &AbbrNode, scope: Scope) -> Result<Vec<Line>, RenderError> {
        let (snippet, remainder) =
            self.set
                .resolve_word(&node.word)
                .ok_or_else(|| RenderError::UnknownTrigger {
                    word: node.word.clone(),
                    suggestion: suggest(self.set, &node.word),
                })?;
        let bindings = bind_args(snippet, &node.args, remainder)?;
        let template = select_template(snippet, scope, self.raw_node_kinds);
        // Children live in the snippet's declared scope; a scope-less
        // snippet passes its own effective scope through unchanged.
        let child_scope = snippet
            .scope
            .as_deref()
            .and_then(parse_scope_name)
            .unwrap_or(scope);

        let mut out = Vec::new();
        for _ in 0..node.repeat {
            let mut lines = self.instantiate(template, &bindings);
            // Fail fast: repeat counts multiply through nesting, so the
            // budget must trip during composition, not on a finished Vec.
            self.composed_lines = self.composed_lines.saturating_add(lines.len());
            if self.composed_lines > MAX_COMPOSED_LINES {
                return Err(RenderError::TooLarge {
                    limit: MAX_COMPOSED_LINES,
                });
            }
            if node.children.is_empty() {
                // A leaf's child slot degrades to a final-cursor candidate.
                for line in &mut lines {
                    for span in &mut line.spans {
                        if matches!(span, CSpan::Slot) {
                            *span = CSpan::Final;
                        }
                    }
                }
            } else {
                // A parent with children drops its own final cursors.
                for line in &mut lines {
                    line.spans.retain(|s| !matches!(s, CSpan::Final));
                }
                let mut children = Vec::new();
                for child in &node.children {
                    children.extend(self.compose_node(child, child_scope)?);
                }
                lines = splice_children(lines, children);
            }
            out.extend(lines);
        }
        Ok(out)
    }

    /// Instantiate one copy of a template with the node's bindings.
    ///
    /// Every unbound parameter and every template-local tabstop index gets a
    /// fresh identity on first occurrence; repeats become mirrors.
    fn instantiate(&mut self, template: &Template, bindings: &[(String, Binding)]) -> Vec<Line> {
        let mut param_ids: HashMap<&str, StopId> = HashMap::new();
        let mut index_ids: HashMap<u32, StopId> = HashMap::new();

        let mut lines = Vec::with_capacity(template.lines.len());
        for ir_line in &template.lines {
            let mut spans = Vec::with_capacity(ir_line.spans.len());
            for span in &ir_line.spans {
                spans.push(match span {
                    Span::Text(t) => CSpan::Text(t.clone()),
                    Span::Param { name } => self.instantiate_param(name, bindings, &mut param_ids),
                    Span::Tabstop { index, placeholder } => {
                        if let Some(&id) = index_ids.get(index) {
                            CSpan::Stop { id, mirror: true }
                        } else {
                            let id = self.alloc(placeholder.clone());
                            index_ids.insert(*index, id);
                            CSpan::Stop { id, mirror: false }
                        }
                    }
                    Span::ChildSlot => CSpan::Slot,
                    Span::FinalCursor => CSpan::Final,
                });
            }
            lines.push(Line {
                depth: ir_line.depth,
                spans,
            });
        }
        lines
    }

    /// Instantiate one `${name}` reference.
    fn instantiate_param<'b>(
        &mut self,
        name: &'b str,
        bindings: &[(String, Binding)],
        param_ids: &mut HashMap<&'b str, StopId>,
    ) -> CSpan {
        let binding = bindings.iter().find(|(n, _)| n == name).map(|(_, b)| b);
        match binding {
            Some(Binding::Bound(value)) => CSpan::Text(value.clone()),
            other => {
                // `None` means the body references an undeclared name; treat
                // it as a free parameter with no default.
                let default = match other {
                    Some(Binding::Free { default }) => default.clone(),
                    _ => None,
                };
                if let Some(&id) = param_ids.get(name) {
                    CSpan::Stop { id, mirror: true }
                } else {
                    let id = self.alloc(default);
                    param_ids.insert(name, id);
                    CSpan::Stop { id, mirror: false }
                }
            }
        }
    }
}

/// Pick the template to instantiate for a node in `scope`.
///
/// The first matching variant wins; a variant matches when its context is
/// the scope's normalized name, or when it is `ts:<kind>` and `<kind>` is
/// one of the buffer's raw node kinds. No match falls back to the default
/// template. Selection changes only the template — params and description
/// always come from the snippet itself.
fn select_template<'s>(
    snippet: &'s CompiledSnippet,
    scope: Scope,
    raw_node_kinds: &[String],
) -> &'s Template {
    snippet
        .variants
        .iter()
        .find(|variant| {
            variant.context.strip_prefix("ts:").map_or_else(
                || parse_scope_name(&variant.context) == Some(scope),
                |kind| raw_node_kinds.iter().any(|k| k == kind),
            )
        })
        .map_or(&snippet.template, |variant| &variant.template)
}

/// Insert composed child lines at the parent's `${child}` slot.
///
/// Child depths shift by the slot line's depth. Spans before the slot prefix
/// the first child line; spans after it suffix the last child line. Without
/// a slot, children append after the parent's lines at the parent's depth.
fn splice_children(mut lines: Vec<Line>, mut children: Vec<Line>) -> Vec<Line> {
    let slot = lines.iter().enumerate().find_map(|(li, line)| {
        line.spans
            .iter()
            .position(|s| matches!(s, CSpan::Slot))
            .map(|si| (li, si))
    });
    let Some((li, si)) = slot else {
        lines.append(&mut children);
        return lines;
    };

    let slot_line = lines.remove(li);
    let slot_depth = slot_line.depth;
    let mut rest = slot_line.spans;
    let mut suffix = rest.split_off(si + 1);
    rest.pop(); // the slot itself
    let mut prefix = rest;

    for child in &mut children {
        child.depth += slot_depth;
    }
    if children.is_empty() {
        prefix.append(&mut suffix);
        children.push(Line {
            depth: slot_depth,
            spans: prefix,
        });
    } else {
        if let Some(first) = children.first_mut() {
            prefix.append(&mut first.spans);
            first.spans = prefix;
        }
        if let Some(last) = children.last_mut() {
            last.spans.append(&mut suffix);
        }
    }
    lines.splice(li..li, children);
    lines
}

/// Replace the surviving final-cursor candidate with the dedented selection
/// followed by a fresh final cursor.
///
/// The injection point is the last [`CSpan::Final`] in document order — the
/// exact occurrence [`renumber`] would keep as `$0` — or, when none exists,
/// the end of the last line (mirroring `renumber`'s append fallback). The
/// selection's first line splices inline between the spans surrounding the
/// injection point; subsequent lines become new [`Line`]s at the injection
/// line's depth, with blank selection lines as empty-span (truly blank)
/// lines.
fn inject_selection(mut lines: Vec<Line>, selection: &str) -> Vec<Line> {
    let target = lines.iter().enumerate().rev().find_map(|(li, line)| {
        line.spans
            .iter()
            .rposition(|s| matches!(s, CSpan::Final))
            .map(|si| (li, si, true))
    });
    let (li, si, had_final) = target.unwrap_or_else(|| {
        if lines.is_empty() {
            lines.push(Line {
                depth: 0,
                spans: Vec::new(),
            });
        }
        let li = lines.len() - 1;
        (li, lines[li].spans.len(), false)
    });

    let line = lines.remove(li);
    let depth = line.depth;
    let mut spans = line.spans;
    let suffix = spans.split_off(si + usize::from(had_final));
    if had_final {
        spans.pop(); // the final cursor being replaced
    }

    let dedented = dedent(selection);
    let mut replacement = Vec::new();
    for (i, sel_line) in dedented.split('\n').enumerate() {
        if i > 0 {
            replacement.push(Line { depth, spans });
            spans = Vec::new();
        }
        if !sel_line.is_empty() {
            spans.push(CSpan::Text(sel_line.to_owned()));
        }
    }
    spans.push(CSpan::Final);
    spans.extend(suffix);
    replacement.push(Line { depth, spans });

    lines.splice(li..li, replacement);
    lines
}

/// Strip the common leading whitespace of `text`'s non-empty lines.
///
/// The common prefix is literal (byte-for-byte, tabs and spaces are not
/// width-equated). Blank and whitespace-only lines do not contribute to the
/// prefix; the strip applies only to lines that actually carry it.
fn dedent(text: &str) -> String {
    let mut common: Option<&str> = None;
    for line in text.split('\n') {
        if line.trim().is_empty() {
            continue;
        }
        let stripped = line.trim_start_matches([' ', '\t']);
        let indent = &line[..line.len() - stripped.len()];
        common = Some(common.map_or(indent, |prev| {
            let len = prev
                .bytes()
                .zip(indent.bytes())
                .take_while(|(a, b)| a == b)
                .count();
            &prev[..len]
        }));
    }
    let common = common.unwrap_or("");
    let lines: Vec<&str> = text
        .split('\n')
        .map(|line| line.strip_prefix(common).unwrap_or(line))
        .collect();
    lines.join("\n")
}

/// Bind a node's explicit and glued arguments to a snippet's parameters.
fn bind_args(
    snippet: &CompiledSnippet,
    args: &[String],
    remainder: &str,
) -> Result<Vec<(String, Binding)>, RenderError> {
    let err = |message: String| RenderError::ArgBinding {
        trigger: snippet.trigger.clone(),
        message,
    };
    if args.len() > snippet.params.len() {
        return Err(err(format!(
            "too many arguments: {} given, {} declared",
            args.len(),
            snippet.params.len()
        )));
    }

    let mut bindings: Vec<(String, Binding)> = snippet
        .params
        .iter()
        .map(|p| {
            (
                p.name.clone(),
                Binding::Free {
                    default: p.default.clone(),
                },
            )
        })
        .collect();
    for (binding, arg) in bindings.iter_mut().zip(args) {
        binding.1 = Binding::Bound(arg.clone());
    }

    if !remainder.is_empty() {
        let (kind, kind_word) = if remainder.bytes().all(|b| b.is_ascii_digit()) {
            (ParamKind::Int, "int")
        } else if is_identifier(remainder) {
            (ParamKind::Ident, "ident")
        } else {
            return Err(err(format!(
                "glued `{remainder}` is neither an integer nor an identifier"
            )));
        };

        let mut already_bound: Option<&str> = None;
        let mut bound = false;
        for (param, binding) in snippet.params.iter().zip(&mut bindings) {
            if param.kind != kind {
                continue;
            }
            if matches!(binding.1, Binding::Free { .. }) {
                binding.1 = Binding::Bound(remainder.to_owned());
                bound = true;
                break;
            }
            already_bound.get_or_insert(&param.name);
        }
        if !bound {
            let message = already_bound.map_or_else(
                || format!("no {kind_word} parameter to bind glued `{remainder}`"),
                |name| format!("parameter `{name}` already bound"),
            );
            return Err(err(message));
        }
    }
    Ok(bindings)
}

/// Whether a glued remainder looks like an identifier.
fn is_identifier(s: &str) -> bool {
    let mut bytes = s.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// The nearest registered trigger within Levenshtein distance 2, if any.
///
/// Ties break to the lexicographically first trigger (the set iterates in
/// sorted order).
fn suggest(set: &SnippetSet, word: &str) -> Option<String> {
    let mut best: Option<(usize, &str)> = None;
    for snippet in set.iter() {
        let distance = levenshtein(word, &snippet.trigger);
        if distance <= 2 && best.is_none_or(|(d, _)| distance < d) {
            best = Some((distance, &snippet.trigger));
        }
    }
    best.map(|(_, trigger)| trigger.to_owned())
}

/// Levenshtein edit distance over bytes (triggers are ASCII).
fn levenshtein(a: &str, b: &str) -> usize {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for (i, &ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let substitute = prev[j] + usize::from(ca != cb);
            cur[j + 1] = substitute.min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev.last().copied().unwrap_or_default()
}

/// One line of renumbered output, ready to materialize.
#[derive(Debug)]
struct NumberedLine {
    /// Indent depth in `indent_unit`s.
    depth: u32,
    /// The line's spans, in order.
    spans: Vec<NSpan>,
}

/// A span with its final global tabstop number.
#[derive(Debug)]
enum NSpan {
    /// Literal text, unescaped.
    Text(String),
    /// A tabstop occurrence. `mirror` occurrences render as bare `$n` in
    /// keep mode; the placeholder is the identity's, used in strip mode.
    Stop {
        number: u32,
        placeholder: Option<String>,
        mirror: bool,
    },
    /// The single surviving `$0`.
    Final,
}

/// Renumber tabstops globally in document order; exactly one `$0` survives.
///
/// Distinct identities get `1..n` by first occurrence. Every final-cursor
/// candidate except the last demotes to a plain numbered stop. If no
/// candidate exists at all, one is appended to the last line.
fn renumber(mut lines: Vec<Line>, placeholders: &[Option<String>]) -> Vec<NumberedLine> {
    let final_count = lines
        .iter()
        .flat_map(|l| &l.spans)
        .filter(|s| matches!(s, CSpan::Final))
        .count();
    if final_count == 0 {
        if let Some(last) = lines.last_mut() {
            last.spans.push(CSpan::Final);
        } else {
            lines.push(Line {
                depth: 0,
                spans: vec![CSpan::Final],
            });
        }
    }
    let last_final = final_count.saturating_sub(1);

    let mut numbers: HashMap<StopId, u32> = HashMap::new();
    let mut next: u32 = 1;
    let mut finals_seen = 0usize;

    lines
        .into_iter()
        .map(|line| NumberedLine {
            depth: line.depth,
            spans: line
                .spans
                .into_iter()
                .map(|span| match span {
                    CSpan::Text(t) => NSpan::Text(t),
                    CSpan::Stop { id, mirror } => {
                        let number = *numbers.entry(id).or_insert_with(|| {
                            let n = next;
                            next += 1;
                            n
                        });
                        NSpan::Stop {
                            number,
                            placeholder: placeholders.get(id).cloned().flatten(),
                            mirror,
                        }
                    }
                    CSpan::Final if finals_seen == last_final => {
                        finals_seen += 1;
                        NSpan::Final
                    }
                    CSpan::Final => {
                        finals_seen += 1;
                        let number = next;
                        next += 1;
                        NSpan::Stop {
                            number,
                            placeholder: None,
                            mirror: false,
                        }
                    }
                    // `compose_node` eliminates every slot.
                    CSpan::Slot => NSpan::Text(String::new()),
                })
                .collect(),
        })
        .collect()
}

/// Render the numbered IR to text with the context's indentation.
fn materialize(lines: &[NumberedLine], ctx: &ExpansionContext, mode: TabstopMode) -> Expansion {
    let unit = ctx.indent_unit.as_str();
    let mut body = String::new();
    let mut cursor = None;

    for (line_no, line) in lines.iter().enumerate() {
        let mut rendered = String::new();
        if !line.spans.is_empty() {
            // The caller inserts at a position that already carries the base
            // indent, so the first line omits it.
            if line_no > 0 {
                rendered.push_str(&ctx.base_indent);
            }
            for _ in 0..line.depth {
                rendered.push_str(&unit);
            }
            for span in &line.spans {
                write_span(&mut rendered, span, mode, line_no, &mut cursor);
            }
        }
        if line_no > 0 {
            body.push('\n');
        }
        body.push_str(&rendered);
    }

    Expansion {
        body,
        is_snippet: mode == TabstopMode::Keep,
        cursor,
    }
}

/// Append one span's text to the line being rendered.
fn write_span(
    out: &mut String,
    span: &NSpan,
    mode: TabstopMode,
    line_no: usize,
    cursor: &mut Option<CursorPos>,
) {
    match (mode, span) {
        (_, NSpan::Text(t)) => match mode {
            TabstopMode::Keep => push_escaped_text(out, t),
            TabstopMode::Strip => out.push_str(t),
        },
        (
            TabstopMode::Keep,
            NSpan::Stop {
                number,
                placeholder: Some(p),
                mirror: false,
            },
        ) => {
            let _ = write!(out, "${{{number}:");
            push_escaped_placeholder(out, p);
            out.push('}');
        }
        (TabstopMode::Keep, NSpan::Stop { number, .. }) => {
            let _ = write!(out, "${number}");
        }
        (TabstopMode::Keep, NSpan::Final) => out.push_str("$0"),
        (TabstopMode::Strip, NSpan::Stop { placeholder, .. }) => {
            if let Some(p) = placeholder {
                out.push_str(p);
            }
        }
        (TabstopMode::Strip, NSpan::Final) => {
            *cursor = Some(CursorPos {
                line: u32::try_from(line_no).unwrap_or(u32::MAX),
                character: u32::try_from(out.len()).unwrap_or(u32::MAX),
            });
        }
    }
}

/// Escape top-level literal text for LSP snippet syntax.
fn push_escaped_text(out: &mut String, text: &str) {
    for c in text.chars() {
        if matches!(c, '$' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
}

/// Escape placeholder text for LSP snippet syntax.
fn push_escaped_placeholder(out: &mut String, text: &str) {
    for c in text.chars() {
        if matches!(c, '$' | '\\' | '}') {
            out.push('\\');
        }
        out.push(c);
    }
}

#[cfg(test)]
// Snippet bodies like `${1:pattern}` are fixture data, not format strings.
#[allow(clippy::literal_string_with_formatting_args)]
mod tests {
    use super::*;
    use crate::abbr::parse;
    use crate::config::loader::CompiledVariant;
    use crate::config::schema::ParamDef;
    use crate::context::IndentUnit;
    use crate::snippet::template::parse_template;

    fn param(name: &str, kind: ParamKind, default: Option<&str>) -> ParamDef {
        ParamDef {
            name: name.to_owned(),
            kind,
            default: default.map(str::to_owned),
        }
    }

    fn snip(trigger: &str, params: Vec<ParamDef>, body: &str) -> CompiledSnippet {
        CompiledSnippet {
            trigger: trigger.to_owned(),
            description: None,
            params,
            template: parse_template(body).expect("fixture body should parse"),
            scope: None,
            variants: Vec::new(),
        }
    }

    /// Attach a declared `${child}` scope to a fixture snippet.
    fn scoped(mut snippet: CompiledSnippet, scope: &str) -> CompiledSnippet {
        snippet.scope = Some(scope.to_owned());
        snippet
    }

    /// Attach a context variant to a fixture snippet.
    fn with_variant(mut snippet: CompiledSnippet, context: &str, body: &str) -> CompiledSnippet {
        snippet.variants.push(CompiledVariant {
            context: context.to_owned(),
            template: parse_template(body).expect("fixture variant should parse"),
        });
        snippet
    }

    /// Fixtures mirroring the real Rust/Python packs.
    fn fixture_set() -> SnippetSet {
        SnippetSet::from_compiled([
            scoped(
                with_variant(
                    snip(
                        "fn",
                        vec![param("name", ParamKind::Ident, None)],
                        "fn ${name}($1) {\n\t${child}\n}\n",
                    ),
                    "type-body",
                    "fn ${name}(&self$1) {\n\t${child}\n}\n",
                ),
                "fn-body",
            ),
            scoped(
                snip(
                    "impl",
                    vec![param("name", ParamKind::Ident, Some("Type"))],
                    "impl ${name} {\n\t${child}\n}\n",
                ),
                "type-body",
            ),
            snip(
                "fori",
                vec![
                    param("var", ParamKind::Ident, Some("i")),
                    param("end", ParamKind::Int, Some("10")),
                ],
                "for ${var} in 0..${end} {\n\t${child}\n}\n",
            ),
            snip("pr", vec![], "println!(\"$0\");\n"),
            snip(
                "def",
                vec![param("name", ParamKind::Ident, None)],
                "def ${name}($1):\n\t${child}\n",
            ),
            snip("arm", vec![], "${1:pattern} => $0,\n"),
            snip("if", vec![], "if $1 {\n\t${child}\n}\n"),
            snip("el", vec![], "else {\n\t${child}\n}\n"),
            snip(
                "prop",
                vec![param("name", ParamKind::Ident, Some("value"))],
                "@property\ndef ${name}(self):\n\treturn self._${name}\n",
            ),
            snip(
                "let",
                vec![param("name", ParamKind::Ident, None)],
                "let ${name} = $1;\n",
            ),
            snip("cash", vec![], "let x = \"\\$\";$0\n"),
            snip("ph", vec![], "${1:a\\}b}$0\n"),
            snip("gap", vec![], "a();$1\n\nb();$0\n"),
            snip("stmt", vec![], "stmt();\n"),
            snip("wrap", vec![], "do(${child});\n"),
        ])
    }

    fn ctx() -> ExpansionContext {
        ExpansionContext::plain(String::new(), IndentUnit::Spaces(4))
    }

    fn expand(abbr: &str, ctx: &ExpansionContext, mode: TabstopMode) -> Expansion {
        let abbr = parse(abbr).expect("abbreviation should parse");
        render(&abbr, &fixture_set(), ctx, mode).expect("abbreviation should render")
    }

    fn expand_err(abbr: &str) -> RenderError {
        let abbr = parse(abbr).expect("abbreviation should parse");
        render(&abbr, &fixture_set(), &ctx(), TabstopMode::Keep)
            .expect_err("abbreviation should fail to render")
    }

    #[test]
    fn leaf_keep() {
        let e = expand("pr", &ctx(), TabstopMode::Keep);
        assert_eq!(e.body, "println!(\"$0\");");
        assert!(e.is_snippet);
        assert_eq!(e.cursor, None);
    }

    #[test]
    fn leaf_strip_records_cursor_inside_quotes() {
        let e = expand("pr", &ctx(), TabstopMode::Strip);
        assert_eq!(e.body, "println!(\"\");");
        assert!(!e.is_snippet);
        assert_eq!(
            e.cursor,
            Some(CursorPos {
                line: 0,
                character: 10
            })
        );
    }

    #[test]
    fn full_nesting_keep() {
        let e = expand("fn:main>fori10>pr", &ctx(), TabstopMode::Keep);
        assert_eq!(
            e.body,
            "fn main($1) {\n    for ${2:i} in 0..10 {\n        println!(\"$0\");\n    }\n}"
        );
    }

    #[test]
    fn full_nesting_strip_cursor() {
        let e = expand("fn:main>fori10>pr", &ctx(), TabstopMode::Strip);
        assert_eq!(
            e.body,
            "fn main() {\n    for i in 0..10 {\n        println!(\"\");\n    }\n}"
        );
        assert_eq!(
            e.cursor,
            Some(CursorPos {
                line: 2,
                character: 18
            })
        );
    }

    #[test]
    fn glued_int_binds_first_free_int_param() {
        let e = expand("fori10", &ctx(), TabstopMode::Keep);
        assert_eq!(e.body, "for ${1:i} in 0..10 {\n    $0\n}");
    }

    #[test]
    fn glued_ident_binds_first_free_ident_param() {
        let e = expand("forix", &ctx(), TabstopMode::Keep);
        assert_eq!(e.body, "for x in 0..${1:10} {\n    $0\n}");
    }

    #[test]
    fn explicit_arg_is_literal_and_tabstop_survives() {
        let e = expand("fn:main", &ctx(), TabstopMode::Keep);
        assert_eq!(e.body, "fn main($1) {\n    $0\n}");
    }

    #[test]
    fn too_many_args_error() {
        assert_eq!(
            expand_err("fn:a,b"),
            RenderError::ArgBinding {
                trigger: "fn".to_owned(),
                message: "too many arguments: 2 given, 1 declared".to_owned(),
            }
        );
    }

    #[test]
    fn glued_with_no_matching_param_error() {
        assert_eq!(
            expand_err("pr10"),
            RenderError::ArgBinding {
                trigger: "pr".to_owned(),
                message: "no int parameter to bind glued `10`".to_owned(),
            }
        );
    }

    #[test]
    fn glued_when_param_already_bound_error() {
        // `fori5` with both params explicitly bound: `5` has nowhere to go.
        assert_eq!(
            expand_err("fori5:x,10"),
            RenderError::ArgBinding {
                trigger: "fori".to_owned(),
                message: "parameter `end` already bound".to_owned(),
            }
        );
    }

    #[test]
    fn unknown_trigger_with_suggestion() {
        assert_eq!(
            expand_err("forj"),
            RenderError::UnknownTrigger {
                word: "forj".to_owned(),
                suggestion: Some("fori".to_owned()),
            }
        );
    }

    #[test]
    fn unknown_trigger_without_suggestion() {
        assert_eq!(
            expand_err("zzzzz"),
            RenderError::UnknownTrigger {
                word: "zzzzz".to_owned(),
                suggestion: None,
            }
        );
    }

    #[test]
    fn repeat_copies_are_independent_and_last_final_wins() {
        let e = expand("arm*3", &ctx(), TabstopMode::Keep);
        assert_eq!(
            e.body,
            "${1:pattern} => $2,\n${3:pattern} => $4,\n${5:pattern} => $0,"
        );
    }

    #[test]
    fn repeat_renders_children_inside_each_copy() {
        let e = expand("fori*2>pr", &ctx(), TabstopMode::Keep);
        assert_eq!(
            e.body,
            "for ${1:i} in 0..${2:10} {\n    println!(\"$3\");\n}\n\
             for ${4:i} in 0..${5:10} {\n    println!(\"$0\");\n}"
        );
    }

    #[test]
    fn siblings_concatenate() {
        let e = expand("if+el", &ctx(), TabstopMode::Keep);
        assert_eq!(e.body, "if $1 {\n    $2\n}\nelse {\n    $0\n}");
    }

    #[test]
    fn unbound_repeated_param_becomes_tabstop_plus_mirror() {
        let e = expand("prop", &ctx(), TabstopMode::Keep);
        assert_eq!(
            e.body,
            "@property\ndef ${1:value}(self):\n    return self._$1$0"
        );
    }

    #[test]
    fn bound_repeated_param_is_literal_everywhere() {
        let e = expand("prop:x", &ctx(), TabstopMode::Keep);
        assert_eq!(e.body, "@property\ndef x(self):\n    return self._x$0");
    }

    #[test]
    fn mirror_renders_identity_placeholder_in_strip_mode() {
        let e = expand("prop", &ctx(), TabstopMode::Strip);
        assert_eq!(
            e.body,
            "@property\ndef value(self):\n    return self._value"
        );
    }

    #[test]
    fn base_indent_applies_to_all_lines_but_first() {
        let base = ExpansionContext::plain("    ".to_owned(), IndentUnit::Spaces(4));
        let e = expand("fori10", &base, TabstopMode::Keep);
        assert_eq!(e.body, "for ${1:i} in 0..10 {\n        $0\n    }");
    }

    #[test]
    fn empty_lines_stay_empty() {
        let base = ExpansionContext::plain("    ".to_owned(), IndentUnit::Spaces(4));
        let e = expand("gap", &base, TabstopMode::Keep);
        assert_eq!(e.body, "a();$1\n\n    b();$0");
    }

    #[test]
    fn python_leaf_child_slot_becomes_final_cursor() {
        let e = expand("def", &ctx(), TabstopMode::Keep);
        assert_eq!(e.body, "def $1($2):\n    $0");
    }

    #[test]
    fn literal_dollar_escaped_in_keep_plain_in_strip() {
        let keep = expand("cash", &ctx(), TabstopMode::Keep);
        assert_eq!(keep.body, "let x = \"\\$\";$0");
        let strip = expand("cash", &ctx(), TabstopMode::Strip);
        assert_eq!(strip.body, "let x = \"$\";");
        assert_eq!(
            strip.cursor,
            Some(CursorPos {
                line: 0,
                character: 12
            })
        );
    }

    #[test]
    fn placeholder_brace_is_escaped_in_keep_mode() {
        let e = expand("ph", &ctx(), TabstopMode::Keep);
        assert_eq!(e.body, "${1:a\\}b}$0");
        let strip = expand("ph", &ctx(), TabstopMode::Strip);
        assert_eq!(strip.body, "a}b");
    }

    #[test]
    fn template_without_final_cursor_gets_one_appended() {
        let e = expand("let", &ctx(), TabstopMode::Keep);
        assert_eq!(e.body, "let $1 = $2;$0");
    }

    #[test]
    fn children_without_slot_append_as_siblings() {
        let e = expand("stmt>pr", &ctx(), TabstopMode::Keep);
        assert_eq!(e.body, "stmt();\nprintln!(\"$0\");");
    }

    #[test]
    fn inline_child_slot_prefixes_and_suffixes() {
        let e = expand("wrap>pr", &ctx(), TabstopMode::Keep);
        assert_eq!(e.body, "do(println!(\"$0\"););");
    }

    #[test]
    fn tab_indent_unit() {
        let tabs = ExpansionContext::plain(String::new(), IndentUnit::Tab);
        let e = expand("fori10>pr", &tabs, TabstopMode::Keep);
        assert_eq!(e.body, "for ${1:i} in 0..10 {\n\tprintln!(\"$0\");\n}");
    }

    /// A context whose innermost buffer scope is `scope`.
    fn ctx_in(scope: Scope) -> ExpansionContext {
        let mut ctx = ctx();
        ctx.scopes = vec![Scope::Root, scope];
        ctx
    }

    #[test]
    fn variant_fires_from_buffer_scope() {
        let e = expand("fn:area", &ctx_in(Scope::TypeBody), TabstopMode::Keep);
        assert_eq!(e.body, "fn area(&self$1) {\n    $0\n}");
    }

    #[test]
    fn variant_does_not_fire_at_root() {
        let e = expand("fn:area", &ctx(), TabstopMode::Keep);
        assert_eq!(e.body, "fn area($1) {\n    $0\n}");
    }

    #[test]
    fn parent_declared_scope_selects_child_variant() {
        let e = expand("impl:Point>fn:new", &ctx(), TabstopMode::Keep);
        assert_eq!(
            e.body,
            "impl Point {\n    fn new(&self$1) {\n        $0\n    }\n}"
        );
    }

    #[test]
    fn declared_scope_overrides_buffer_scope_for_children() {
        // Inside a type body the outer `fn` takes &self, but its declared
        // "fn-body" scope shields the nested `fn` from the variant.
        let e = expand(
            "fn:outer>fn:inner",
            &ctx_in(Scope::TypeBody),
            TabstopMode::Keep,
        );
        assert_eq!(
            e.body,
            "fn outer(&self$1) {\n    fn inner($2) {\n        $0\n    }\n}"
        );
    }

    #[test]
    fn scope_less_parent_is_transparent() {
        // `if` declares no scope, so `fn` under it still sees TypeBody.
        let e = expand("impl:Point>if>fn:helper", &ctx(), TabstopMode::Keep);
        assert_eq!(
            e.body,
            "impl Point {\n    if $1 {\n        fn helper(&self$2) {\n            $0\n        }\n    }\n}"
        );
    }

    #[test]
    fn ts_variant_matches_raw_node_kinds() {
        let set = SnippetSet::from_compiled([with_variant(
            snip("pr", vec![], "println!(\"$0\");\n"),
            "ts:impl_item",
            "self.log(\"$0\");\n",
        )]);
        let mut ctx = ctx();
        ctx.raw_node_kinds = vec!["source_file".to_owned(), "impl_item".to_owned()];
        let abbr = parse("pr").expect("abbreviation should parse");
        let e = render(&abbr, &set, &ctx, TabstopMode::Keep).expect("should render");
        assert_eq!(e.body, "self.log(\"$0\");");

        // Without the raw kind the default template renders.
        let plain =
            render(&abbr, &set, &ctx_in(Scope::Root), TabstopMode::Keep).expect("should render");
        assert_eq!(plain.body, "println!(\"$0\");");
    }

    #[test]
    fn first_matching_variant_wins() {
        let set = SnippetSet::from_compiled([with_variant(
            with_variant(snip("pr", vec![], "default$0\n"), "type-body", "first$0\n"),
            "ts:impl_item",
            "second$0\n",
        )]);
        let mut ctx = ctx_in(Scope::TypeBody);
        ctx.raw_node_kinds = vec!["impl_item".to_owned()];
        let abbr = parse("pr").expect("abbreviation should parse");
        let e = render(&abbr, &set, &ctx, TabstopMode::Keep).expect("should render");
        assert_eq!(e.body, "first$0");
    }

    #[test]
    fn empty_scopes_default_to_root() {
        let mut ctx = ctx();
        ctx.scopes.clear();
        let e = expand("fn:area", &ctx, TabstopMode::Keep);
        assert_eq!(e.body, "fn area($1) {\n    $0\n}");
    }

    #[test]
    fn nested_repeats_over_budget_fail_fast() {
        // ~10^6 composed lines without the budget; must fail quickly.
        let abbr = parse("if*100>if*100>if*100").expect("abbreviation should parse");
        let err = render(&abbr, &fixture_set(), &ctx(), TabstopMode::Keep)
            .expect_err("over-budget expansion should fail");
        assert_eq!(
            err,
            RenderError::TooLarge {
                limit: MAX_COMPOSED_LINES
            }
        );
    }

    #[test]
    fn large_but_under_budget_expansion_renders() {
        // 10 * (3 + 100) = 1,030 composed lines: well under the budget.
        let e = expand("if*10>stmt*100", &ctx(), TabstopMode::Keep);
        assert_eq!(e.body.lines().count(), 10 * 102);
    }

    /// Wrap `selection` into `abbr` against the shared fixture set.
    fn wrap_expand(
        abbr: &str,
        selection: &str,
        ctx: &ExpansionContext,
        mode: TabstopMode,
    ) -> Expansion {
        let abbr = parse(abbr).expect("abbreviation should parse");
        render_with_selection(&abbr, &fixture_set(), ctx, mode, selection)
            .expect("wrap fixture should render")
    }

    #[test]
    fn wrap_single_line_into_block_keep() {
        let e = wrap_expand("fori10", "println!(\"x\");", &ctx(), TabstopMode::Keep);
        assert_eq!(e.body, "for ${1:i} in 0..10 {\n    println!(\"x\");$0\n}");
        assert!(e.is_snippet);
    }

    #[test]
    fn wrap_single_line_into_block_strip() {
        let e = wrap_expand("fori10", "println!(\"x\");", &ctx(), TabstopMode::Strip);
        assert_eq!(e.body, "for i in 0..10 {\n    println!(\"x\");\n}");
        assert_eq!(
            e.cursor,
            Some(CursorPos {
                line: 1,
                character: 18
            })
        );
    }

    #[test]
    fn wrap_multi_line_selection_dedents_and_reindents_at_injection_depth() {
        let e = wrap_expand(
            "fori10",
            "    a();\n        b();\n    c();",
            &ctx(),
            TabstopMode::Keep,
        );
        assert_eq!(
            e.body,
            "for ${1:i} in 0..10 {\n    a();\n        b();\n    c();$0\n}"
        );
    }

    #[test]
    fn wrap_continuation_lines_get_base_indent_in_inline_construct() {
        // The old bin-layer walker dropped the hanging indent here: `pr`
        // puts `$0` mid-line at depth 0, and only materialize knows the
        // base indent that later selection lines must carry.
        let base = ExpansionContext::plain("    ".to_owned(), IndentUnit::Spaces(4));
        let e = wrap_expand("pr", "a();\nb();", &base, TabstopMode::Keep);
        assert_eq!(e.body, "println!(\"a();\n    b();$0\");");
    }

    #[test]
    fn wrap_base_indent_applies_to_block_continuation_lines() {
        let base = ExpansionContext::plain("    ".to_owned(), IndentUnit::Spaces(4));
        let e = wrap_expand("fori10", "a();\nb();", &base, TabstopMode::Keep);
        assert_eq!(
            e.body,
            "for ${1:i} in 0..10 {\n        a();\n        b();$0\n    }"
        );
    }

    #[test]
    fn wrap_selection_into_inline_construct_mid_line() {
        let e = wrap_expand("pr", "x + y", &ctx(), TabstopMode::Keep);
        assert_eq!(e.body, "println!(\"x + y$0\");");
    }

    #[test]
    fn wrap_selection_dollars_and_backslashes_escape_in_keep_pass_in_strip() {
        let keep = wrap_expand(
            "fori10",
            "let s = \"$HOME\\\\\";",
            &ctx(),
            TabstopMode::Keep,
        );
        assert_eq!(
            keep.body,
            "for ${1:i} in 0..10 {\n    let s = \"\\$HOME\\\\\\\\\";$0\n}"
        );
        let strip = wrap_expand(
            "fori10",
            "let s = \"$HOME\\\\\";",
            &ctx(),
            TabstopMode::Strip,
        );
        assert_eq!(
            strip.body,
            "for i in 0..10 {\n    let s = \"$HOME\\\\\";\n}"
        );
    }

    #[test]
    fn wrap_mirrors_render_placeholder_text_in_strip_mode() {
        // The condition mirrors into the body; a reverse snippet parser
        // (the old bin-layer walker) loses the mirror's text because bare
        // `$n` carries no placeholder. The IR path renders it.
        let set = SnippetSet::from_compiled([snip(
            "wl",
            vec![param("cond", ParamKind::Ident, Some("ready"))],
            "while ${cond} {\n\tlog(${cond});\n\t${child}\n}\n",
        )]);
        let abbr = parse("wl").expect("abbreviation should parse");
        let e = render_with_selection(&abbr, &set, &ctx(), TabstopMode::Strip, "step();")
            .expect("wrap fixture should render");
        assert_eq!(e.body, "while ready {\n    log(ready);\n    step();\n}");
        assert_eq!(
            e.cursor,
            Some(CursorPos {
                line: 2,
                character: 11
            })
        );
        let keep = render_with_selection(&abbr, &set, &ctx(), TabstopMode::Keep, "step();")
            .expect("wrap fixture should render");
        assert_eq!(
            keep.body,
            "while ${1:ready} {\n    log($1);\n    step();$0\n}"
        );
    }

    #[test]
    fn wrap_without_final_cursor_falls_back_to_end_of_last_line() {
        let e = wrap_expand("stmt", "x();", &ctx(), TabstopMode::Keep);
        assert_eq!(e.body, "stmt();x();$0");
    }

    #[test]
    fn wrap_blank_selection_lines_stay_blank() {
        let e = wrap_expand("fori10", "a();\n\nb();", &ctx(), TabstopMode::Keep);
        assert_eq!(e.body, "for ${1:i} in 0..10 {\n    a();\n\n    b();$0\n}");
    }

    #[test]
    fn wrap_earlier_final_candidates_still_demote() {
        let e = wrap_expand("arm*3", "x", &ctx(), TabstopMode::Keep);
        assert_eq!(
            e.body,
            "${1:pattern} => $2,\n${3:pattern} => $4,\n${5:pattern} => x$0,"
        );
    }

    #[test]
    fn wrap_enforces_composition_budget() {
        let abbr = parse("if*100>if*100>if*100").expect("abbreviation should parse");
        let err = render_with_selection(&abbr, &fixture_set(), &ctx(), TabstopMode::Keep, "x")
            .expect_err("over-budget wrap should fail");
        assert_eq!(
            err,
            RenderError::TooLarge {
                limit: MAX_COMPOSED_LINES
            }
        );
    }

    #[test]
    fn dedent_strips_common_literal_prefix_only() {
        assert_eq!(dedent("    a\n      b\n    c"), "a\n  b\nc");
        assert_eq!(dedent("\ta\n\t\tb"), "a\n\tb");
        assert_eq!(dedent("a\n    b"), "a\n    b");
        assert_eq!(dedent("    a\n\n    b"), "a\n\nb");
        // Tabs and spaces never width-equate: no common prefix here.
        assert_eq!(dedent("\ta\n    b"), "\ta\n    b");
    }
}
