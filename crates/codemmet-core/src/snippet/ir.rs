//! The snippet intermediate representation.
//!
//! Template bodies parse into lines with *relative indent depth*; braces are
//! plain text and the renderer owns indentation, so brace languages and
//! indentation languages (Python) share one pipeline.

/// A parsed snippet body: what a TOML `body` string compiles to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    /// The template's lines, in order.
    pub lines: Vec<IrLine>,
}

/// One line of a template with its indent depth relative to the snippet root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrLine {
    /// Indent depth relative to the snippet's first line (which is depth 0).
    pub depth: u32,
    /// The line's content, in order. Empty for a blank line.
    pub spans: Vec<Span>,
}

/// A run of content within a line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Span {
    /// Literal text, exactly as it should appear.
    Text(String),
    /// A numbered tabstop from `$n` / `${n:placeholder}` in the body.
    ///
    /// Indices are template-local; the renderer renumbers globally. A
    /// repeated index within one template becomes a mirror at render time.
    Tabstop {
        /// Template-local tabstop index (never 0; `$0` is [`Span::FinalCursor`]).
        index: u32,
        /// Placeholder text, if the body used `${n:placeholder}`.
        placeholder: Option<String>,
    },
    /// A named parameter reference from `${name}` in the body.
    ///
    /// Renders as literal text when the abbreviation supplied an argument,
    /// otherwise as a tabstop whose placeholder is the param's default.
    /// Repeated references to one name become mirrors.
    Param {
        /// The parameter name; must exist in the snippet's `params` list.
        name: String,
    },
    /// `${i}`: the 1-based repetition index of the enclosing node.
    ///
    /// Renders the copy number when the node is repeated with `*n` (`1`, `2`,
    /// …), and `1` when the node is not repeated.
    RepeatIndex,
    /// `${child}`: where `>`-nested children render, one indent level deeper.
    ///
    /// At most one per template. For a leaf node this degrades to the final
    /// cursor position.
    ChildSlot,
    /// `$0`: the final cursor position after all tabstops.
    FinalCursor,
}
