//! Composition and rendering: abbreviation AST + compiled snippets → text.

use std::collections::HashMap;
use std::fmt::Write as _;

use std::collections::BTreeMap;

use crate::abbr::{AbbrNode, Abbreviation, Repeat};
use crate::config::loader::{
    CompiledComposite, CompiledSnippet, SnippetSet, parse_scope_name, substitute_markers,
};
use crate::config::schema::ParamKind;
use crate::context::{Collection, ExpansionContext, Item, Scope};
use crate::snippet::ir::{Span, Template};

/// Maximum number of composed lines per expansion.
///
/// Repeat counts multiply through nesting (`if*100>if*100>…`), so without a
/// budget a few dozen bytes of abbreviation can demand billions of lines.
/// The composer checks the running total after every template instantiation
/// and fails fast with [`RenderError::TooLarge`].
const MAX_COMPOSED_LINES: usize = 10_000;

/// Maximum nesting depth of composite (alias) expansions.
///
/// Definitional cycles are already rejected when a pack is compiled
/// ([`SnippetSet::validate_composites`](crate::config::loader)), so a valid set
/// cannot recurse forever; this is a backstop against a pathologically deep —
/// but acyclic — chain of composites, and against caller-authored nesting like
/// `classp>classp>classp>…`.
const MAX_COMPOSITE_DEPTH: u32 = 16;

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
    /// A caller selected an option name a composite does not define.
    #[error("composite `{trigger}` has no option `{option}`{}", available(options))]
    UnknownOption {
        /// The composite whose option lookup failed.
        trigger: String,
        /// The option name the caller asked for.
        option: String,
        /// The option names the composite does define, sorted.
        options: Vec<String>,
    },
    /// A composite's expansion could not be re-parsed or was misused (e.g. a
    /// data-driven repeat on a composite).
    #[error("cannot expand composite `{trigger}`: {message}")]
    Composite {
        /// The composite that failed to expand.
        trigger: String,
        /// What went wrong.
        message: String,
    },
    /// Composite expansion nested deeper than [`MAX_COMPOSITE_DEPTH`].
    #[error("composite expansion nested too deep (more than {limit} levels)")]
    CompositeTooDeep {
        /// The depth cap that was exceeded (`MAX_COMPOSITE_DEPTH`).
        limit: u32,
    },
}

/// Render the `(one, two)` option-name hint for [`RenderError::UnknownOption`].
fn available(options: &[String]) -> String {
    if options.is_empty() {
        String::new()
    } else {
        format!(" (options: {})", options.join(", "))
    }
}

/// Render a parsed abbreviation against a language's snippet set.
pub fn render(
    abbr: &Abbreviation,
    set: &SnippetSet,
    ctx: &ExpansionContext,
    mode: TabstopMode,
) -> Result<Expansion, RenderError> {
    // Normal expansion: a leaf's `${child}` slot degrades to the final cursor.
    let (lines, placeholders) = compose(abbr, set, ctx, false)?;
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
    // Keep the `${child}` slot so the selection injects into the construct's
    // body (e.g. a `try` body, not its `except` clause where `$0` sits).
    let (lines, placeholders) = compose(abbr, set, ctx, true)?;
    let lines = inject_selection(lines, selection);
    let lines = renumber(lines, &placeholders);
    Ok(materialize(&lines, ctx, mode))
}

/// Compose an abbreviation into the line IR plus its placeholder table.
///
/// `keep_child_slot` leaves a leaf node's `${child}` slot in place instead of
/// degrading it to a final cursor; wrap uses this so the selection can be
/// injected into the body slot.
fn compose(
    abbr: &Abbreviation,
    set: &SnippetSet,
    ctx: &ExpansionContext,
    keep_child_slot: bool,
) -> Result<(Vec<Line>, Vec<Option<String>>), RenderError> {
    let mut composer = Composer {
        set,
        raw_node_kinds: &ctx.raw_node_kinds,
        placeholders: Vec::new(),
        composed_lines: 0,
        keep_child_slot,
        collections: &ctx.collections,
        composite_depth: 0,
    };
    // A top-level node lives in the buffer's innermost scope.
    let scope = ctx.scopes.last().copied().unwrap_or(Scope::Root);
    let mut lines = Vec::new();
    for node in &abbr.0 {
        lines.extend(composer.compose_node(node, scope, None)?);
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
    /// Keep a leaf's `${child}` slot instead of degrading it to a final
    /// cursor (wrap injects the selection into that slot).
    keep_child_slot: bool,
    /// Named collections in scope, for data-driven repeats.
    collections: &'a BTreeMap<String, Collection>,
    /// Current composite-expansion nesting depth, capped at
    /// [`MAX_COMPOSITE_DEPTH`].
    composite_depth: u32,
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
    fn compose_node(
        &mut self,
        node: &AbbrNode,
        scope: Scope,
        subject: Option<&str>,
    ) -> Result<Vec<Line>, RenderError> {
        let (snippet, remainder) =
            self.set
                .resolve_word(&node.word)
                .ok_or_else(|| RenderError::UnknownTrigger {
                    word: node.word.clone(),
                    suggestion: suggest(self.set, &node.word),
                })?;
        // A composite resolves to an abbreviation, not a template body: expand
        // it (recursively) and merge its selected options + the caller's
        // freeform children before composing the result.
        if let Some(composite) = &snippet.composite {
            return self.expand_composite(node, snippet, composite, remainder, scope, subject);
        }
        let bindings = bind_args(snippet, &node.args, remainder)?;
        let template = select_template(snippet, scope, self.raw_node_kinds);
        // Children live in the snippet's declared scope; a scope-less
        // snippet passes its own effective scope through unchanged.
        let child_scope = snippet
            .scope
            .as_deref()
            .and_then(parse_scope_name)
            .unwrap_or(scope);
        // The subject for this node's children: this node's first argument if
        // it names a known collection (e.g. `match:Msg` → `Msg`), else the
        // inherited subject. This is what a child's bare `*` repeats over.
        let child_subject = node
            .args
            .first()
            .filter(|a| self.collections.contains_key(a.as_str()))
            .map_or(subject, |a| Some(a.as_str()));

        // One entry per copy: `Some(item)` for a data-driven repeat, `None`
        // for a literal `*n` copy or a data-driven repeat with no resolvable
        // collection (which degrades to a single copy).
        let items = self.resolve_items(&node.repeat, subject);
        let mut out = Vec::new();
        for (copy, item) in items.iter().enumerate() {
            // The repetition index is 1-based (`${i}` renders 1, 2, …).
            let index = u32::try_from(copy + 1).unwrap_or(u32::MAX);
            let mut lines = self.instantiate(template, &bindings, index, item.as_ref());
            // Fail fast: repeat counts multiply through nesting, so the
            // budget must trip during composition, not on a finished Vec.
            self.composed_lines = self.composed_lines.saturating_add(lines.len());
            if self.composed_lines > MAX_COMPOSED_LINES {
                return Err(RenderError::TooLarge {
                    limit: MAX_COMPOSED_LINES,
                });
            }
            if node.children.is_empty() {
                // A leaf's child slot degrades to a final-cursor candidate —
                // unless we're keeping it for a wrap injection.
                if !self.keep_child_slot {
                    for line in &mut lines {
                        for span in &mut line.spans {
                            if matches!(span, CSpan::Slot) {
                                *span = CSpan::Final;
                            }
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
                    children.extend(self.compose_node(child, child_scope, child_subject)?);
                }
                lines = splice_children(lines, children);
            }
            out.extend(lines);
        }
        Ok(out)
    }

    /// Expand a composite node: substitute its params, splice in the selected
    /// options and the caller's freeform `>`-children, then re-expand the
    /// resulting abbreviation and compose it at the node's position.
    ///
    /// Argument split: the composite's declared params bind first (glued and
    /// explicit args, exactly as a normal snippet); any *remaining* explicit
    /// args select options by name. A literal `*n` repeats the whole
    /// expansion; a data-driven repeat on a composite is rejected.
    fn expand_composite(
        &mut self,
        node: &AbbrNode,
        snippet: &CompiledSnippet,
        composite: &CompiledComposite,
        remainder: &str,
        scope: Scope,
        subject: Option<&str>,
    ) -> Result<Vec<Line>, RenderError> {
        if node.repeat.is_data_driven() {
            return Err(RenderError::Composite {
                trigger: snippet.trigger.clone(),
                message: "a data-driven repeat (`*@name` or `*`) is not supported on a composite"
                    .to_owned(),
            });
        }
        let count = match node.repeat {
            Repeat::Count(n) => n,
            Repeat::Source(_) | Repeat::Subject => 1,
        };

        // Declared params bind first; remaining explicit args select options.
        let param_count = node.args.len().min(snippet.params.len());
        let (param_args, option_args) = node.args.split_at(param_count);
        let bindings = bind_args(snippet, param_args, remainder)?;

        let mut selected: Vec<&str> = Vec::with_capacity(option_args.len());
        for name in option_args {
            match composite.options.get(name) {
                Some(fragment) => selected.push(fragment.as_str()),
                None => {
                    return Err(RenderError::UnknownOption {
                        trigger: snippet.trigger.clone(),
                        option: name.clone(),
                        options: composite.options.keys().cloned().collect(),
                    });
                }
            }
        }

        if self.composite_depth >= MAX_COMPOSITE_DEPTH {
            return Err(RenderError::CompositeTooDeep {
                limit: MAX_COMPOSITE_DEPTH,
            });
        }
        self.composite_depth += 1;
        let result = self.compose_composite_copies(
            node, snippet, composite, &bindings, &selected, count, scope, subject,
        );
        self.composite_depth -= 1;
        result
    }

    /// Compose `count` independent copies of a composite's merged expansion.
    #[allow(clippy::too_many_arguments)]
    fn compose_composite_copies(
        &mut self,
        node: &AbbrNode,
        snippet: &CompiledSnippet,
        composite: &CompiledComposite,
        bindings: &[(String, Binding)],
        selected: &[&str],
        count: u32,
        scope: Scope,
        subject: Option<&str>,
    ) -> Result<Vec<Line>, RenderError> {
        let expand = |fragment: &str| expand_fragment(fragment, bindings, &snippet.trigger);

        let mut out = Vec::new();
        for _ in 0..count {
            let mut base = expand(&composite.expands)?;
            // Extras merge as siblings of the base pieces: selected options
            // first (in selection order), then the caller's freeform children.
            let mut extras: Vec<AbbrNode> = Vec::new();
            for fragment in selected {
                extras.extend(expand(fragment)?);
            }
            extras.extend(node.children.iter().cloned());
            merge_extras(&mut base, extras);

            for top in &base {
                out.extend(self.compose_node(top, scope, subject)?);
            }
        }
        Ok(out)
    }

    /// Instantiate one copy of a template with the node's bindings.
    ///
    /// `index` is the 1-based repetition index for this copy; it renders
    /// [`Span::RepeatIndex`] and replaces `${i}` in argument values and
    /// placeholder defaults. Every unbound parameter and every template-local
    /// tabstop index gets a fresh identity on first occurrence; repeats become
    /// mirrors.
    /// One entry per rendered copy for `repeat`: the data-driven items, or
    /// `None`s for a literal `*n`. A data-driven repeat whose collection is
    /// missing or empty degrades to a single `None` copy (the `${value}`
    /// placeholders become tabstops), so the user still gets a scaffold.
    fn resolve_items(&self, repeat: &Repeat, subject: Option<&str>) -> Vec<Option<Item>> {
        let items_of = |name: &str| match self.collections.get(name) {
            Some(collection) if !collection.items.is_empty() => {
                collection.items.iter().cloned().map(Some).collect()
            }
            _ => vec![None],
        };
        match repeat {
            Repeat::Count(n) => (0..*n).map(|_| None).collect(),
            Repeat::Source(name) => items_of(name),
            Repeat::Subject => subject.map_or_else(|| vec![None], items_of),
        }
    }

    fn instantiate(
        &mut self,
        template: &Template,
        bindings: &[(String, Binding)],
        index: u32,
        item: Option<&Item>,
    ) -> Vec<Line> {
        let mut param_ids: HashMap<&str, StopId> = HashMap::new();
        let mut index_ids: HashMap<u32, StopId> = HashMap::new();

        let mut lines = Vec::with_capacity(template.lines.len());
        for ir_line in &template.lines {
            let mut spans = Vec::with_capacity(ir_line.spans.len());
            for span in &ir_line.spans {
                spans.push(match span {
                    Span::Text(t) => CSpan::Text(t.clone()),
                    Span::Param { name } => {
                        self.instantiate_param(name, bindings, index, &mut param_ids)
                    }
                    Span::Tabstop {
                        index: ti,
                        placeholder,
                    } => {
                        if let Some(&id) = index_ids.get(ti) {
                            CSpan::Stop { id, mirror: true }
                        } else {
                            let id = self.alloc(placeholder.clone());
                            index_ids.insert(*ti, id);
                            CSpan::Stop { id, mirror: false }
                        }
                    }
                    Span::RepeatIndex => CSpan::Text(index.to_string()),
                    Span::ItemValue { member, default } => item.map_or_else(
                        // No item: leave a tabstop carrying the default.
                        || CSpan::Stop {
                            id: self.alloc(default.clone()),
                            mirror: false,
                        },
                        |it| {
                            let text = member.as_ref().map_or_else(
                                || it.value.clone(),
                                |key| {
                                    it.members
                                        .get(key)
                                        .cloned()
                                        .or_else(|| default.clone())
                                        .unwrap_or_default()
                                },
                            );
                            CSpan::Text(text)
                        },
                    ),
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

    /// Instantiate one `${name}` reference for copy `index`.
    fn instantiate_param<'b>(
        &mut self,
        name: &'b str,
        bindings: &[(String, Binding)],
        index: u32,
        param_ids: &mut HashMap<&'b str, StopId>,
    ) -> CSpan {
        let binding = bindings.iter().find(|(n, _)| n == name).map(|(_, b)| b);
        match binding {
            Some(Binding::Bound(value)) => CSpan::Text(sub_repeat_index(value, index)),
            other => {
                // `None` means the body references an undeclared name; treat
                // it as a free parameter with no default.
                let default = match other {
                    Some(Binding::Free { default }) => {
                        default.as_deref().map(|d| sub_repeat_index(d, index))
                    }
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

/// Replace the repetition-index variable `${i}` in `text` with `index`.
///
/// A no-op (cheap clone) when `${i}` is absent, so it is safe to call on every
/// argument value.
fn sub_repeat_index(text: &str, index: u32) -> String {
    if text.contains("${i}") {
        text.replace("${i}", &index.to_string())
    } else {
        text.to_owned()
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

/// Replace the injection point with the dedented selection followed by a fresh
/// final cursor.
///
/// The injection point is the first [`CSpan::Slot`] (the construct's `${child}`
/// body, so the selection lands there — e.g. a `try` body rather than its
/// `except` clause), else the last [`CSpan::Final`] in document order (the `$0`
/// [`renumber`] would keep), else the end of the last line. The selection's
/// first line splices inline between the spans surrounding the injection point;
/// subsequent lines become new [`Line`]s at the injection line's depth, with
/// blank selection lines as empty-span (truly blank) lines.
fn inject_selection(mut lines: Vec<Line>, selection: &str) -> Vec<Line> {
    let slot = lines.iter().enumerate().find_map(|(li, line)| {
        line.spans
            .iter()
            .position(|s| matches!(s, CSpan::Slot))
            .map(|si| (li, si))
    });
    let target = slot.or_else(|| {
        lines.iter().enumerate().rev().find_map(|(li, line)| {
            line.spans
                .iter()
                .rposition(|s| matches!(s, CSpan::Final))
                .map(|si| (li, si))
        })
    });
    let (li, si, replace) = target.map_or_else(
        || {
            if lines.is_empty() {
                lines.push(Line {
                    depth: 0,
                    spans: Vec::new(),
                });
            }
            let li = lines.len() - 1;
            (li, lines[li].spans.len(), false)
        },
        |(li, si)| (li, si, true),
    );

    let line = lines.remove(li);
    let depth = line.depth;
    let mut spans = line.spans;
    let suffix = spans.split_off(si + usize::from(replace));
    if replace {
        spans.pop(); // the slot or final cursor being replaced
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

/// Merge a composite's extra pieces (options + freeform children) into its
/// base expansion at the single insertion point.
///
/// The insertion point is the children of the **last top-level node** of the
/// base abbreviation, so `class:Name>(init+str+repr)` gains the extras as
/// further siblings of the base methods inside the class body. When the base
/// is a single leaf node (`class:Name`) the extras simply become its children.
fn merge_extras(base: &mut [AbbrNode], extras: Vec<AbbrNode>) {
    if extras.is_empty() {
        return;
    }
    if let Some(last) = base.last_mut() {
        last.children.extend(extras);
    }
}

/// Expand one composite fragment (the base expansion or a selected option) into
/// abbreviation nodes, **passing the composite's arguments through** to the
/// sub-triggers.
///
/// A `${name}` marker in the fragment is an *argument position* for a
/// sub-trigger (e.g. `class:${name}`):
///
/// - **bound** (the caller supplied it) → the value is emitted as that argument
///   (`classp:Point` → `class:Point`);
/// - **unbound** → the argument is **dropped**, so the sub-trigger's own
///   parameter default surfaces as a tabstop (`classp` → `class` →
///   `class ${1:Name}`). The declared default is *not* substituted as text.
///
/// A marker that is not a whole argument (embedded in a word or in longer arg
/// text) has no argument slot to drop, so it falls back to value-or-default
/// text substitution.
///
/// Markers are resolved on the parsed tree (not the string) so an unbound
/// argument can simply be removed; because the abbreviation grammar has no
/// empty positional slot, an unbound argument *before* a later argument in the
/// same list is a positional gap and is rejected with a clear error.
fn expand_fragment(
    fragment: &str,
    bindings: &[(String, Binding)],
    trigger: &str,
) -> Result<Vec<AbbrNode>, RenderError> {
    let markers = ArgMarkers::new(bindings.len());
    // Substitute `${name}` with a parse-safe positional marker first: an
    // omitted argument cannot be written literally (`class:` does not parse),
    // so the drop must happen after parsing.
    let text = substitute_markers(fragment, |name| {
        bindings
            .iter()
            .position(|(n, _)| n == name)
            .map_or_else(String::new, |index| markers.marker(index).to_owned())
    });
    let mut nodes = crate::abbr::parse(&text)
        .map(|abbr| abbr.0)
        .map_err(|err| RenderError::Composite {
            trigger: trigger.to_owned(),
            message: format!("expansion `{fragment}` does not parse: {err}"),
        })?;
    resolve_markers(&mut nodes, bindings, &markers, trigger)?;
    Ok(nodes)
}

/// Resolve the argument markers left by [`expand_fragment`] in place.
fn resolve_markers(
    nodes: &mut [AbbrNode],
    bindings: &[(String, Binding)],
    markers: &ArgMarkers,
    trigger: &str,
) -> Result<(), RenderError> {
    for node in nodes.iter_mut() {
        // A marker in the WORD (or glued into it) has no argument slot to drop,
        // so it degrades to value-or-default text.
        if markers.contains_any(&node.word) {
            node.word = markers.substitute_text(&node.word, bindings);
        }

        // Map each argument to its resolved value, or `None` to drop it.
        let mut new_args: Vec<String> = Vec::with_capacity(node.args.len());
        let mut dropped: Option<usize> = None;
        for arg in &node.args {
            if let Some(index) = markers.index_of(arg) {
                match &bindings[index].1 {
                    Binding::Bound(value) => {
                        if let Some(param) = dropped {
                            return Err(positional_gap(trigger, &bindings[param].0));
                        }
                        new_args.push(value.clone());
                    }
                    Binding::Free { .. } => dropped = Some(index),
                }
            } else {
                if let Some(param) = dropped {
                    return Err(positional_gap(trigger, &bindings[param].0));
                }
                new_args.push(if markers.contains_any(arg) {
                    markers.substitute_text(arg, bindings)
                } else {
                    arg.clone()
                });
            }
        }
        node.args = new_args;

        resolve_markers(&mut node.children, bindings, markers, trigger)?;
    }
    Ok(())
}

/// The positional-gap error: an unbound argument sits before a later one.
fn positional_gap(trigger: &str, param: &str) -> RenderError {
    RenderError::Composite {
        trigger: trigger.to_owned(),
        message: format!(
            "unbound parameter `{param}` leaves a positional gap before a later \
             argument; declare optional params last, or pass `{param}`"
        ),
    }
}

/// Parse-safe placeholders standing in for `${name}` argument positions between
/// substitution and marker resolution, one per declared parameter (by index).
struct ArgMarkers {
    by_index: Vec<String>,
}

impl ArgMarkers {
    /// Build markers for `count` parameters. The tokens use only WORD-safe
    /// characters so they survive parsing in argument *and* word positions, and
    /// are distinctive enough not to collide with real abbreviation text.
    fn new(count: usize) -> Self {
        Self {
            by_index: (0..count).map(|i| format!("CMMTARG{i}CMMT")).collect(),
        }
    }

    /// The marker token for parameter `index`.
    fn marker(&self, index: usize) -> &str {
        &self.by_index[index]
    }

    /// The parameter index whose marker is exactly `arg`, if any.
    fn index_of(&self, arg: &str) -> Option<usize> {
        self.by_index.iter().position(|m| m == arg)
    }

    /// Whether `text` contains any marker as a substring.
    fn contains_any(&self, text: &str) -> bool {
        self.by_index.iter().any(|m| text.contains(m.as_str()))
    }

    /// Replace embedded markers with value-or-default text (the non-argument
    /// fallback).
    fn substitute_text(&self, text: &str, bindings: &[(String, Binding)]) -> String {
        let mut out = text.to_owned();
        for (index, marker) in self.by_index.iter().enumerate() {
            if out.contains(marker) {
                let replacement = match &bindings[index].1 {
                    Binding::Bound(value) => value.clone(),
                    Binding::Free { default } => default.clone().unwrap_or_default(),
                };
                out = out.replace(marker, &replacement);
            }
        }
        out
    }
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
            composite: None,
        }
    }

    /// Attach a declared `${child}` scope to a fixture snippet.
    fn scoped(mut snippet: CompiledSnippet, scope: &str) -> CompiledSnippet {
        snippet.scope = Some(scope.to_owned());
        snippet
    }

    /// Build a composite fixture snippet.
    fn composite(
        trigger: &str,
        params: Vec<ParamDef>,
        expands: &str,
        options: &[(&str, &str)],
    ) -> CompiledSnippet {
        CompiledSnippet {
            trigger: trigger.to_owned(),
            description: None,
            params,
            template: Template { lines: Vec::new() },
            scope: None,
            variants: Vec::new(),
            composite: Some(CompiledComposite {
                expands: expands.to_owned(),
                options: options
                    .iter()
                    .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                    .collect(),
            }),
        }
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
            snip("arm", vec![], "${value:pattern} => $0,\n"),
            scoped(
                snip(
                    "match",
                    vec![param("expr", ParamKind::Ident, Some("expr"))],
                    "match ${expr} {\n\t${child}\n}\n",
                ),
                "match-body",
            ),
            snip("fielddef", vec![], "${value}: ${value.type},\n"),
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
            snip("field", vec![], "field${i}: ${1:Type},\n"),
            snip("cash", vec![], "let x = \"\\$\";$0\n"),
            snip("ph", vec![], "${1:a\\}b}$0\n"),
            snip("gap", vec![], "a();$1\n\nb();$0\n"),
            snip("stmt", vec![], "stmt();\n"),
            snip("wrap", vec![], "do(${child});\n"),
            // A construct with both a `${child}` body and a later `$0` (like a
            // try/catch): wrap should target the child body, not `$0`.
            snip("tc", vec![], "try {\n\t${child}\n} catch {\n\t$0\n}\n"),
            // Composites (alias snippets). `cls` builds an `impl` with a base
            // `fn new`; options and freeform children merge into the body.
            composite(
                "cls",
                vec![param("name", ParamKind::Ident, Some("X"))],
                "impl:${name}>(fn:new)",
                &[("log", "stmt"), ("pair", "stmt+stmt")],
            ),
            // A composite that references another composite (`cls`).
            composite("nest", vec![], "cls:Inner", &[]),
            // Two params passed through to one sub-trigger's arg list, for the
            // pass-through / positional-gap tests.
            composite(
                "rng",
                vec![
                    param("var", ParamKind::Ident, Some("i")),
                    param("end", ParamKind::Int, Some("10")),
                ],
                "fori:${var},${end}",
                &[],
            ),
            // A deliberately self-referential composite: `from_compiled`
            // skips validation, so this reaches the render-time depth cap.
            composite("loopy", vec![], "loopy", &[]),
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
    fn repeat_index_in_body_numbers_each_copy() {
        // `${i}` renders the 1-based copy index; tabstops still renumber
        // globally and independently per copy.
        let e = expand("field*3", &ctx(), TabstopMode::Keep);
        assert_eq!(
            e.body,
            "field1: ${1:Type},\nfield2: ${2:Type},\nfield3: ${3:Type},$0"
        );
    }

    #[test]
    fn repeat_index_without_repeat_is_one() {
        let e = expand("field", &ctx(), TabstopMode::Strip);
        assert_eq!(e.body, "field1: Type,");
    }

    #[test]
    fn repeat_index_substitutes_into_argument_values() {
        // `${i}` inside an explicit argument is replaced per copy.
        let e = expand("let:x${i}*3", &ctx(), TabstopMode::Strip);
        assert_eq!(e.body, "let x1 = ;\nlet x2 = ;\nlet x3 = ;");
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

    // --- composites (alias snippets) -----------------------------------------

    #[test]
    fn composite_unbound_param_leaves_a_tabstop() {
        // Bare `cls` → `impl:${name}` with `name` unbound: the argument is
        // dropped, so `impl`'s own `name` default surfaces as a tabstop
        // (`${1:Type}`) rather than being substituted as literal text.
        let e = expand("cls", &ctx(), TabstopMode::Keep);
        assert_eq!(
            e.body,
            "impl ${1:Type} {\n    fn new(&self$2) {\n        $0\n    }\n}"
        );
    }

    #[test]
    fn composite_param_passes_through_as_bound_argument() {
        // A supplied arg passes through as a literal argument to the sub-trigger.
        let e = expand("cls:Point", &ctx(), TabstopMode::Keep);
        assert_eq!(
            e.body,
            "impl Point {\n    fn new(&self$1) {\n        $0\n    }\n}"
        );
    }

    #[test]
    fn composite_both_params_unbound_pass_through_as_tabstops() {
        // `rng` → `fori:${var},${end}`, both unbound: both arguments drop, so
        // `fori`'s own `var`/`end` defaults become tabstops.
        let e = expand("rng", &ctx(), TabstopMode::Keep);
        assert_eq!(e.body, "for ${1:i} in 0..${2:10} {\n    $0\n}");
    }

    #[test]
    fn composite_trailing_unbound_param_drops() {
        // `rng:x` binds `var`; the trailing unbound `end` drops to its tabstop.
        let e = expand("rng:x", &ctx(), TabstopMode::Keep);
        assert_eq!(e.body, "for x in 0..${1:10} {\n    $0\n}");
    }

    #[test]
    fn composite_interior_positional_gap_is_error() {
        // Glued `5` binds the `int` param `end`, leaving `var` unbound *before*
        // it — a positional gap the grammar cannot express.
        assert_eq!(
            expand_err("rng5"),
            RenderError::Composite {
                trigger: "rng".to_owned(),
                message: "unbound parameter `var` leaves a positional gap before a later \
                          argument; declare optional params last, or pass `var`"
                    .to_owned(),
            }
        );
    }

    #[test]
    fn composite_glued_arg_binds_like_a_normal_param() {
        // A glued ident binds the composite's `name` param, exactly as it
        // would for a plain snippet.
        let e = expand("clsWidget", &ctx(), TabstopMode::Strip);
        assert!(e.body.starts_with("impl Widget {"), "{}", e.body);
    }

    #[test]
    fn composite_selected_option_merges_as_sibling() {
        let e = expand("cls:Point,log", &ctx(), TabstopMode::Strip);
        assert_eq!(
            e.body,
            "impl Point {\n    fn new(&self) {\n        \n    }\n    stmt();\n}"
        );
    }

    #[test]
    fn composite_freeform_child_merges_into_body() {
        let e = expand("cls:Point>pr", &ctx(), TabstopMode::Strip);
        assert_eq!(
            e.body,
            "impl Point {\n    fn new(&self) {\n        \n    }\n    println!(\"\");\n}"
        );
    }

    #[test]
    fn composite_option_then_freeform_child_both_merge_in_order() {
        // Base `fn new`, then the `pair` option (two `stmt`s), then the
        // freeform `pr` — all siblings inside the class body, in that order.
        let e = expand("cls:Point,pair>pr", &ctx(), TabstopMode::Strip);
        assert_eq!(
            e.body,
            "impl Point {\n    fn new(&self) {\n        \n    }\n    \
             stmt();\n    stmt();\n    println!(\"\");\n}"
        );
    }

    #[test]
    fn composite_repeat_produces_independent_copies() {
        let e = expand("cls:Point*2", &ctx(), TabstopMode::Keep);
        assert_eq!(
            e.body,
            "impl Point {\n    fn new(&self$1) {\n        $2\n    }\n}\n\
             impl Point {\n    fn new(&self$3) {\n        $0\n    }\n}"
        );
    }

    #[test]
    fn composite_references_another_composite() {
        // `nest` → `cls:Inner` → `impl Inner>(fn:new)`.
        let e = expand("nest", &ctx(), TabstopMode::Strip);
        assert_eq!(
            e.body,
            "impl Inner {\n    fn new(&self) {\n        \n    }\n}"
        );
    }

    #[test]
    fn composite_unknown_option_is_error() {
        assert_eq!(
            expand_err("cls:Point,bogus"),
            RenderError::UnknownOption {
                trigger: "cls".to_owned(),
                option: "bogus".to_owned(),
                options: vec!["log".to_owned(), "pair".to_owned()],
            }
        );
    }

    #[test]
    fn composite_rejects_data_driven_repeat() {
        assert_eq!(
            expand_err("cls*@X"),
            RenderError::Composite {
                trigger: "cls".to_owned(),
                message: "a data-driven repeat (`*@name` or `*`) is not supported on a composite"
                    .to_owned(),
            }
        );
    }

    #[test]
    fn composite_recursion_hits_the_depth_cap() {
        // `loopy` expands to itself; validation would normally reject this,
        // but the fixture bypasses it, so the render-time cap must fire.
        assert_eq!(
            expand_err("loopy"),
            RenderError::CompositeTooDeep {
                limit: MAX_COMPOSITE_DEPTH,
            }
        );
    }

    // --- data-driven repeats -------------------------------------------------

    fn ctx_with(name: &str, values: &[&str]) -> ExpansionContext {
        let mut ctx = ctx();
        ctx.collections.insert(
            name.to_string(),
            Collection {
                items: values.iter().map(|v| Item::new(*v)).collect(),
            },
        );
        ctx
    }

    #[test]
    fn data_driven_explicit_source_fills_one_copy_per_item() {
        let ctx = ctx_with("Msg", &["Quit", "Move", "Write"]);
        let e = expand("arm*@Msg", &ctx, TabstopMode::Strip);
        assert_eq!(e.body, "Quit => ,\nMove => ,\nWrite => ,");
    }

    #[test]
    fn data_driven_subject_inferred_from_parent() {
        let ctx = ctx_with("Msg", &["Quit", "Move"]);
        let e = expand("match:Msg>arm*", &ctx, TabstopMode::Strip);
        assert_eq!(e.body, "match Msg {\n    Quit => ,\n    Move => ,\n}");
    }

    #[test]
    fn data_driven_missing_collection_falls_back_to_one_copy() {
        // No `Nope` collection in context → a single copy with the tabstop.
        let e = expand("arm*@Nope", &ctx(), TabstopMode::Strip);
        assert_eq!(e.body, "pattern => ,");
    }

    #[test]
    fn data_driven_members_render_via_value_dot_key() {
        let mut ctx = ctx();
        let field = |name: &str, ty: &str| {
            let mut item = Item::new(name);
            item.members.insert("type".to_string(), ty.to_string());
            item
        };
        ctx.collections.insert(
            "Point".to_string(),
            Collection {
                items: vec![field("x", "f64"), field("y", "i32")],
            },
        );
        let e = expand("fielddef*@Point", &ctx, TabstopMode::Strip);
        assert_eq!(e.body, "x: f64,\ny: i32,");
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
    fn wrap_injects_into_child_body_not_the_final_cursor() {
        // `tc` has a `${child}` body and a separate `$0` (in the catch). The
        // selection must land in the body; `$0` stays in the catch.
        let e = wrap_expand("tc", "risky();", &ctx(), TabstopMode::Keep);
        assert_eq!(e.body, "try {\n    risky();$1\n} catch {\n    $0\n}");
        assert!(e.is_snippet);
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
