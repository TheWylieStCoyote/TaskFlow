//! Recursive descent parser for the filter DSL.
//!
//! Parses token streams into an AST (Abstract Syntax Tree) representation
//! that can be evaluated against tasks.
//!
//! # Parsing Algorithm
//!
//! This parser uses a **recursive descent** approach with explicit precedence handling.
//! Each grammar rule maps to a parsing function:
//!
//! ```text
//! Expression  ::= OrExpr
//! OrExpr      ::= AndExpr ("OR" AndExpr)*
//! AndExpr     ::= UnaryExpr ("AND" UnaryExpr)*
//! UnaryExpr   ::= "!" UnaryExpr | Primary
//! Primary     ::= "(" Expression ")" | Condition
//! Condition   ::= Identifier ":" Value
//! ```
//!
//! # Operator Precedence
//!
//! Operators are parsed with the following precedence (highest to lowest):
//!
//! | Precedence | Operator | Description | Associativity |
//! |------------|----------|-------------|---------------|
//! | 1 (highest) | `!` | NOT (negation) | Right |
//! | 2 | `AND` | Logical AND | Left |
//! | 3 (lowest) | `OR` | Logical OR | Left |
//!
//! Parentheses `()` override precedence.
//!
//! ## Precedence Examples
//!
//! ```text
//! Input: priority:high AND status:todo OR tags:bug
//! Parsed as: (priority:high AND status:todo) OR tags:bug
//!
//! Input: status:todo OR priority:high AND tags:bug
//! Parsed as: status:todo OR (priority:high AND tags:bug)
//!
//! Input: !status:done AND tags:bug
//! Parsed as: (!status:done) AND tags:bug
//!
//! Input: !(status:done OR status:cancelled)
//! Parsed as: !(status:done OR status:cancelled)
//! ```
//!
//! # Value Parsing
//!
//! Field values are parsed based on their type:
//!
//! | Field Type | Parsing Rules |
//! |------------|---------------|
//! | Priority | `none`, `low`, `medium`/`med`, `high`, `urgent` |
//! | Status | `todo`, `in_progress`/`in-progress`/`inprogress`, `blocked`, `done`/`completed`, `cancelled`/`canceled` |
//! | Date | Keywords (`today`, `tomorrow`), exact (`YYYY-MM-DD`), comparison (`<`/`>`), range (`..`) |
//! | Numeric | Exact number, comparison (`<`/`>`/`<=`/`>=`), range (`start..end`) |
//! | Text | Any string (quoted or unquoted) |
//!
//! # Range Syntax
//!
//! Date and numeric fields support range syntax:
//!
//! ```text
//! Full range:   2025-01-01..2025-12-31  (start to end, inclusive)
//! Open start:   2025-06-01..            (from date onward)
//! Open end:     ..2025-12-31            (up to date)
//! ```
//!
//! Ranges are **inclusive** on both ends.
//!
//! # Error Handling
//!
//! The parser produces detailed error messages with position information:
//!
//! - [`ParseError::EmptyExpression`] - No filter provided
//! - [`ParseError::UnknownField`] - Unrecognized field name
//! - [`ParseError::InvalidValue`] - Invalid value for field type
//! - [`ParseError::UnexpectedToken`] - Syntax error (wrong token)
//! - [`ParseError::UnexpectedEof`] - Premature end of input
//!
//! # Example
//!
//! ```
//! use taskflow::domain::filter_dsl::{parse, FilterExpr};
//!
//! // Simple condition
//! let expr = parse("priority:high").unwrap();
//!
//! // Boolean operators with precedence
//! let expr = parse("priority:high AND !status:done OR tags:urgent").unwrap();
//! // Parsed as: (priority:high AND (!status:done)) OR tags:urgent
//!
//! // Parentheses for grouping
//! let expr = parse("(status:todo OR status:in_progress) AND priority:high").unwrap();
//!
//! // Date range
//! let expr = parse("due:2025-01-01..2025-12-31").unwrap();
//! ```

use chrono::NaiveDate;

use crate::domain::{Priority, TaskStatus};

use super::ast::{
    Condition, CreatedFilter, DueFilter, FilterExpr, FilterField, FilterValue, HasField,
    NumericFilter, ScheduledFilter,
};
use super::error::{ParseError, ParseResult};
use super::lexer::{tokenize, Token, TokenWithSpan};

/// Parser for filter DSL expressions.
pub struct Parser {
    tokens: Vec<TokenWithSpan>,
    position: usize,
}

impl Parser {
    /// Create a new parser from a token stream.
    pub fn new(tokens: Vec<TokenWithSpan>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    /// Parse the token stream into a filter expression.
    pub fn parse(&mut self) -> ParseResult<FilterExpr> {
        if self.check(&Token::Eof) {
            return Err(ParseError::EmptyExpression);
        }

        let expr = self.parse_or_expr()?;
        self.expect_eof()?;
        Ok(expr)
    }

    /// Parse an OR expression (lowest precedence).
    fn parse_or_expr(&mut self) -> ParseResult<FilterExpr> {
        let mut left = self.parse_and_expr()?;

        while self.check(&Token::Or) {
            self.advance();
            let right = self.parse_and_expr()?;
            left = FilterExpr::Or(Box::new(left), Box::new(right));
        }

        Ok(left)
    }

    /// Parse an AND expression.
    fn parse_and_expr(&mut self) -> ParseResult<FilterExpr> {
        let mut left = self.parse_unary_expr()?;

        while self.check(&Token::And) {
            self.advance();
            let right = self.parse_unary_expr()?;
            left = FilterExpr::And(Box::new(left), Box::new(right));
        }

        Ok(left)
    }

    /// Parse a unary (NOT) expression.
    fn parse_unary_expr(&mut self) -> ParseResult<FilterExpr> {
        if self.check(&Token::Not) {
            self.advance();
            let expr = self.parse_unary_expr()?;
            return Ok(FilterExpr::Not(Box::new(expr)));
        }

        self.parse_primary()
    }

    /// Parse a primary expression (parenthesized or condition).
    fn parse_primary(&mut self) -> ParseResult<FilterExpr> {
        if self.check(&Token::LParen) {
            self.advance();
            let expr = self.parse_or_expr()?;
            self.expect(&Token::RParen)?;
            return Ok(expr);
        }

        let condition = self.parse_condition()?;
        Ok(FilterExpr::Condition(condition))
    }

    /// Parse a single condition (field:value).
    fn parse_condition(&mut self) -> ParseResult<Condition> {
        let (field_name, field_pos) = self.expect_identifier()?;
        self.expect(&Token::Colon)?;
        let (value_str, value_pos) = self.expect_value()?;

        let field = parse_field(&field_name, field_pos)?;
        let value = parse_value(field, &value_str, value_pos)?;

        Ok(Condition { field, value })
    }

    // ========================================================================
    // Helper methods
    // ========================================================================

    /// Check if the current token matches the expected type.
    fn check(&self, expected: &Token) -> bool {
        self.current()
            .is_some_and(|t| std::mem::discriminant(&t.token) == std::mem::discriminant(expected))
    }

    /// Get the current token.
    fn current(&self) -> Option<&TokenWithSpan> {
        self.tokens.get(self.position)
    }

    /// Advance to the next token.
    fn advance(&mut self) {
        if self.position < self.tokens.len() {
            self.position += 1;
        }
    }

    /// Expect a specific token type and advance.
    fn expect(&mut self, expected: &Token) -> ParseResult<()> {
        let current = self
            .current()
            .ok_or_else(|| ParseError::unexpected_eof(expected.display_name()))?;

        if std::mem::discriminant(&current.token) != std::mem::discriminant(expected) {
            return Err(ParseError::unexpected_token(
                expected.display_name(),
                current.token.display_name(),
                current.start,
            ));
        }

        self.advance();
        Ok(())
    }

    /// Expect end of input.
    fn expect_eof(&self) -> ParseResult<()> {
        if let Some(current) = self.current() {
            if !matches!(current.token, Token::Eof) {
                return Err(ParseError::unexpected_token(
                    "end of input",
                    current.token.display_name(),
                    current.start,
                ));
            }
        }
        Ok(())
    }

    /// Expect an identifier and return its value.
    fn expect_identifier(&mut self) -> ParseResult<(String, usize)> {
        let current = self
            .current()
            .ok_or_else(|| ParseError::unexpected_eof("field name"))?;

        match &current.token {
            Token::Identifier(name) => {
                let name = name.clone();
                let pos = current.start;
                self.advance();
                Ok((name, pos))
            }
            _ => Err(ParseError::unexpected_token(
                "field name",
                current.token.display_name(),
                current.start,
            )),
        }
    }

    /// Expect a value (identifier or quoted string) and return its value.
    fn expect_value(&mut self) -> ParseResult<(String, usize)> {
        let current = self
            .current()
            .ok_or_else(|| ParseError::unexpected_eof("value"))?;

        match &current.token {
            Token::Identifier(value) => {
                let value = value.clone();
                let pos = current.start;
                self.advance();
                Ok((value, pos))
            }
            Token::QuotedString(value) => {
                let value = value.clone();
                let pos = current.start;
                self.advance();
                Ok((value, pos))
            }
            _ => Err(ParseError::unexpected_token(
                "value",
                current.token.display_name(),
                current.start,
            )),
        }
    }
}

// ============================================================================
// Field and value parsing helpers
// ============================================================================

/// Parse a field name into a FilterField.
fn parse_field(name: &str, position: usize) -> ParseResult<FilterField> {
    match name.to_lowercase().as_str() {
        "priority" => Ok(FilterField::Priority),
        "status" => Ok(FilterField::Status),
        "tags" | "tag" => Ok(FilterField::Tags),
        "project" => Ok(FilterField::Project),
        "due" => Ok(FilterField::Due),
        "search" => Ok(FilterField::Search),
        "has" => Ok(FilterField::Has),
        "title" => Ok(FilterField::Title),
        "created" => Ok(FilterField::Created),
        "scheduled" => Ok(FilterField::Scheduled),
        "completed" => Ok(FilterField::Completed),
        "modified" | "updated" => Ok(FilterField::Modified),
        "estimate" | "est" => Ok(FilterField::Estimate),
        "actual" | "tracked" => Ok(FilterField::Actual),
        _ => Err(ParseError::unknown_field(name, position)),
    }
}

/// Parse a value string into a FilterValue based on the field type.
fn parse_value(field: FilterField, value: &str, position: usize) -> ParseResult<FilterValue> {
    match field {
        FilterField::Priority => {
            let priority = parse_priority(value).ok_or_else(|| {
                ParseError::invalid_value(
                    "priority",
                    value,
                    Some("Valid values: none, low, medium, high, urgent".to_string()),
                    position,
                )
            })?;
            Ok(FilterValue::Priority(priority))
        }
        FilterField::Status => {
            let status = parse_status(value).ok_or_else(|| {
                ParseError::invalid_value(
                    "status",
                    value,
                    Some("Valid values: todo, in_progress, blocked, done, cancelled".to_string()),
                    position,
                )
            })?;
            Ok(FilterValue::Status(status))
        }
        FilterField::Tags => Ok(FilterValue::Tag(value.to_string())),
        FilterField::Project => Ok(FilterValue::ProjectName(value.to_string())),
        FilterField::Due => {
            let due = parse_due(value, position)?;
            Ok(FilterValue::Due(due))
        }
        FilterField::Search => Ok(FilterValue::SearchText(value.to_string())),
        FilterField::Has => {
            let has = parse_has_field(value).ok_or_else(|| {
                ParseError::invalid_value(
                    "has",
                    value,
                    Some("Valid values: due, project, tags, estimate, description, recurrence, scheduled, dependencies, parent, tracked".to_string()),
                    position,
                )
            })?;
            Ok(FilterValue::Has(has))
        }
        FilterField::Title => Ok(FilterValue::TitleText(value.to_string())),
        FilterField::Created => {
            let created = parse_created(value, position)?;
            Ok(FilterValue::Created(created))
        }
        FilterField::Scheduled => {
            let scheduled = parse_scheduled(value, position)?;
            Ok(FilterValue::Scheduled(scheduled))
        }
        FilterField::Completed => {
            let completed = parse_created(value, position)?;
            Ok(FilterValue::Completed(completed))
        }
        FilterField::Modified => {
            let modified = parse_created(value, position)?;
            Ok(FilterValue::Modified(modified))
        }
        FilterField::Estimate => {
            let numeric = parse_numeric(value, position, "estimate")?;
            Ok(FilterValue::Estimate(numeric))
        }
        FilterField::Actual => {
            let numeric = parse_numeric(value, position, "actual")?;
            Ok(FilterValue::Actual(numeric))
        }
    }
}

/// Parse a priority value.
fn parse_priority(s: &str) -> Option<Priority> {
    match s.to_lowercase().as_str() {
        "none" => Some(Priority::None),
        "low" => Some(Priority::Low),
        "medium" | "med" => Some(Priority::Medium),
        "high" => Some(Priority::High),
        "urgent" => Some(Priority::Urgent),
        _ => None,
    }
}

/// Parse a status value.
fn parse_status(s: &str) -> Option<TaskStatus> {
    match s.to_lowercase().replace(['-', '_'], "").as_str() {
        "todo" => Some(TaskStatus::Todo),
        "inprogress" => Some(TaskStatus::InProgress),
        "blocked" => Some(TaskStatus::Blocked),
        "done" | "completed" => Some(TaskStatus::Done),
        "cancelled" | "canceled" => Some(TaskStatus::Cancelled),
        _ => None,
    }
}

/// Parse a date range from a string like "2025-01-01..2025-12-31".
/// Returns (start, end) where either can be None for open-ended ranges.
fn parse_date_range(s: &str) -> Option<(Option<NaiveDate>, Option<NaiveDate>)> {
    let (start, end) = s.split_once("..")?;

    let start_date = if start.is_empty() {
        None
    } else {
        Some(NaiveDate::parse_from_str(start, "%Y-%m-%d").ok()?)
    };

    let end_date = if end.is_empty() {
        None
    } else {
        Some(NaiveDate::parse_from_str(end, "%Y-%m-%d").ok()?)
    };

    // At least one side must be present
    if start_date.is_some() || end_date.is_some() {
        Some((start_date, end_date))
    } else {
        None
    }
}

/// Parse a numeric range from a string like "30..120".
/// Returns (start, end) where either can be None for open-ended ranges.
fn parse_numeric_range(s: &str) -> Option<(Option<u32>, Option<u32>)> {
    let (start, end) = s.split_once("..")?;

    let start_num = if start.is_empty() {
        None
    } else {
        Some(start.parse().ok()?)
    };

    let end_num = if end.is_empty() {
        None
    } else {
        Some(end.parse().ok()?)
    };

    // At least one side must be present
    if start_num.is_some() || end_num.is_some() {
        Some((start_num, end_num))
    } else {
        None
    }
}

/// Parse a due date filter value.
fn parse_due(s: &str, position: usize) -> ParseResult<DueFilter> {
    match s.to_lowercase().as_str() {
        "today" => Ok(DueFilter::Today),
        "tomorrow" => Ok(DueFilter::Tomorrow),
        "thisweek" | "this-week" | "this_week" => Ok(DueFilter::ThisWeek),
        "nextweek" | "next-week" | "next_week" => Ok(DueFilter::NextWeek),
        "overdue" => Ok(DueFilter::Overdue),
        "none" => Ok(DueFilter::None),
        _ => {
            // Try range: YYYY-MM-DD..YYYY-MM-DD or open-ended variants
            if let Some((start, end)) = parse_date_range(s) {
                return match (start, end) {
                    (Some(s), Some(e)) => {
                        if s > e {
                            Err(ParseError::invalid_value(
                                "due",
                                s.to_string(),
                                Some("Range end must be on or after start".to_string()),
                                position,
                            ))
                        } else {
                            Ok(DueFilter::Between(s, e))
                        }
                    }
                    (Some(s), None) => Ok(DueFilter::OnOrAfter(s)),
                    (None, Some(e)) => Ok(DueFilter::OnOrBefore(e)),
                    (None, None) => unreachable!(),
                };
            }
            // Try before date: <YYYY-MM-DD
            if let Some(date_str) = s.strip_prefix('<') {
                if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                    return Ok(DueFilter::Before(date));
                }
            }
            // Try after date: >YYYY-MM-DD
            if let Some(date_str) = s.strip_prefix('>') {
                if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                    return Ok(DueFilter::After(date));
                }
            }
            // Try exact date: YYYY-MM-DD
            if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                return Ok(DueFilter::On(date));
            }
            Err(ParseError::invalid_value(
                "due",
                s,
                Some("Valid values: today, tomorrow, thisweek, nextweek, overdue, none, YYYY-MM-DD, <YYYY-MM-DD, >YYYY-MM-DD, YYYY-MM-DD..YYYY-MM-DD".to_string()),
                position,
            ))
        }
    }
}

/// Parse a has field value.
fn parse_has_field(s: &str) -> Option<HasField> {
    match s.to_lowercase().as_str() {
        "due" => Some(HasField::Due),
        "project" => Some(HasField::Project),
        "tags" | "tag" => Some(HasField::Tags),
        "estimate" | "est" => Some(HasField::Estimate),
        "description" | "desc" => Some(HasField::Description),
        "recurrence" | "recurring" => Some(HasField::Recurrence),
        "scheduled" => Some(HasField::Scheduled),
        "dependencies" | "deps" | "blocked" => Some(HasField::Dependencies),
        "parent" | "subtask" => Some(HasField::Parent),
        "tracked" | "time" => Some(HasField::Tracked),
        _ => None,
    }
}

/// Parse a created date filter value.
fn parse_created(s: &str, position: usize) -> ParseResult<CreatedFilter> {
    match s.to_lowercase().as_str() {
        "today" => Ok(CreatedFilter::Today),
        "yesterday" => Ok(CreatedFilter::Yesterday),
        "thisweek" | "this-week" | "this_week" => Ok(CreatedFilter::ThisWeek),
        "lastweek" | "last-week" | "last_week" => Ok(CreatedFilter::LastWeek),
        _ => {
            // Try range: YYYY-MM-DD..YYYY-MM-DD or open-ended variants
            if let Some((start, end)) = parse_date_range(s) {
                return match (start, end) {
                    (Some(s), Some(e)) => {
                        if s > e {
                            Err(ParseError::invalid_value(
                                "created",
                                s.to_string(),
                                Some("Range end must be on or after start".to_string()),
                                position,
                            ))
                        } else {
                            Ok(CreatedFilter::Between(s, e))
                        }
                    }
                    (Some(s), None) => Ok(CreatedFilter::OnOrAfter(s)),
                    (None, Some(e)) => Ok(CreatedFilter::OnOrBefore(e)),
                    (None, None) => unreachable!(),
                };
            }
            // Try before date: <YYYY-MM-DD
            if let Some(date_str) = s.strip_prefix('<') {
                if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                    return Ok(CreatedFilter::Before(date));
                }
            }
            // Try after date: >YYYY-MM-DD
            if let Some(date_str) = s.strip_prefix('>') {
                if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                    return Ok(CreatedFilter::After(date));
                }
            }
            // Try exact date: YYYY-MM-DD
            if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                return Ok(CreatedFilter::On(date));
            }
            Err(ParseError::invalid_value(
                "created",
                s,
                Some("Valid values: today, yesterday, thisweek, lastweek, YYYY-MM-DD, <YYYY-MM-DD, >YYYY-MM-DD, YYYY-MM-DD..YYYY-MM-DD".to_string()),
                position,
            ))
        }
    }
}

/// Parse a scheduled date filter value.
fn parse_scheduled(s: &str, position: usize) -> ParseResult<ScheduledFilter> {
    match s.to_lowercase().as_str() {
        "today" => Ok(ScheduledFilter::Today),
        "tomorrow" => Ok(ScheduledFilter::Tomorrow),
        "thisweek" | "this-week" | "this_week" => Ok(ScheduledFilter::ThisWeek),
        "nextweek" | "next-week" | "next_week" => Ok(ScheduledFilter::NextWeek),
        "none" => Ok(ScheduledFilter::None),
        _ => {
            // Try range: YYYY-MM-DD..YYYY-MM-DD or open-ended variants
            if let Some((start, end)) = parse_date_range(s) {
                return match (start, end) {
                    (Some(s), Some(e)) => {
                        if s > e {
                            Err(ParseError::invalid_value(
                                "scheduled",
                                s.to_string(),
                                Some("Range end must be on or after start".to_string()),
                                position,
                            ))
                        } else {
                            Ok(ScheduledFilter::Between(s, e))
                        }
                    }
                    (Some(s), None) => Ok(ScheduledFilter::OnOrAfter(s)),
                    (None, Some(e)) => Ok(ScheduledFilter::OnOrBefore(e)),
                    (None, None) => unreachable!(),
                };
            }
            // Try before date: <YYYY-MM-DD
            if let Some(date_str) = s.strip_prefix('<') {
                if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                    return Ok(ScheduledFilter::Before(date));
                }
            }
            // Try after date: >YYYY-MM-DD
            if let Some(date_str) = s.strip_prefix('>') {
                if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                    return Ok(ScheduledFilter::After(date));
                }
            }
            // Try exact date: YYYY-MM-DD
            if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                return Ok(ScheduledFilter::On(date));
            }
            Err(ParseError::invalid_value(
                "scheduled",
                s,
                Some("Valid values: today, tomorrow, thisweek, nextweek, none, YYYY-MM-DD, <YYYY-MM-DD, >YYYY-MM-DD, YYYY-MM-DD..YYYY-MM-DD".to_string()),
                position,
            ))
        }
    }
}

/// Parse a numeric filter value (for estimate/actual).
fn parse_numeric(s: &str, position: usize, field_name: &str) -> ParseResult<NumericFilter> {
    let s_lower = s.to_lowercase();

    // Handle "none" keyword
    if s_lower == "none" {
        return Ok(NumericFilter::None);
    }

    // Try range: number..number or open-ended variants
    if let Some((start, end)) = parse_numeric_range(s) {
        return match (start, end) {
            (Some(s), Some(e)) => {
                if s > e {
                    Err(ParseError::invalid_value(
                        field_name,
                        s.to_string(),
                        Some("Range end must be greater than or equal to start".to_string()),
                        position,
                    ))
                } else {
                    Ok(NumericFilter::Between(s, e))
                }
            }
            (Some(s), None) => Ok(NumericFilter::GreaterOrEqual(s)),
            (None, Some(e)) => Ok(NumericFilter::LessOrEqual(e)),
            (None, None) => unreachable!(),
        };
    }

    // Try comparison operators: >=, <=, >, <
    if let Some(num_str) = s.strip_prefix(">=") {
        if let Ok(num) = num_str.parse::<u32>() {
            return Ok(NumericFilter::GreaterOrEqual(num));
        }
    }
    if let Some(num_str) = s.strip_prefix("<=") {
        if let Ok(num) = num_str.parse::<u32>() {
            return Ok(NumericFilter::LessOrEqual(num));
        }
    }
    if let Some(num_str) = s.strip_prefix('>') {
        if let Ok(num) = num_str.parse::<u32>() {
            return Ok(NumericFilter::GreaterThan(num));
        }
    }
    if let Some(num_str) = s.strip_prefix('<') {
        if let Ok(num) = num_str.parse::<u32>() {
            return Ok(NumericFilter::LessThan(num));
        }
    }

    // Try plain number (equals)
    if let Ok(num) = s.parse::<u32>() {
        return Ok(NumericFilter::Equals(num));
    }

    Err(ParseError::invalid_value(
        field_name,
        s,
        Some(
            "Valid values: number, >number, <number, >=number, <=number, number..number, none"
                .to_string(),
        ),
        position,
    ))
}

// ============================================================================
// Public API
// ============================================================================

/// Parse a filter DSL string into an expression AST.
///
/// # Errors
///
/// Returns a [`ParseError`] if the input cannot be parsed:
/// - [`ParseError::EmptyExpression`] if the input is empty
/// - [`ParseError::UnknownField`] for unrecognized field names
/// - [`ParseError::InvalidValue`] for invalid field values
/// - [`ParseError::UnexpectedToken`] for syntax errors
///
/// # Examples
///
/// ```
/// use taskflow::domain::filter_dsl::{parse, FilterExpr};
///
/// let expr = parse("priority:high AND !status:done").unwrap();
/// assert!(matches!(expr, FilterExpr::And(_, _)));
/// ```
#[must_use = "parsing returns a Result that should be used"]
pub fn parse(input: &str) -> ParseResult<FilterExpr> {
    let input = input.trim();
    if input.is_empty() {
        return Err(ParseError::EmptyExpression);
    }

    let tokens = tokenize(input)?;
    let mut parser = Parser::new(tokens);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_condition() {
        let expr = parse("priority:high").unwrap();
        assert!(matches!(expr, FilterExpr::Condition(_)));
    }

    #[test]
    fn test_parse_and_expression() {
        let expr = parse("priority:high AND status:todo").unwrap();
        assert!(matches!(expr, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_parse_or_expression() {
        let expr = parse("status:todo OR status:in_progress").unwrap();
        assert!(matches!(expr, FilterExpr::Or(_, _)));
    }

    #[test]
    fn test_parse_not_expression() {
        let expr = parse("!status:done").unwrap();
        assert!(matches!(expr, FilterExpr::Not(_)));
    }

    #[test]
    fn test_parse_parentheses() {
        let expr = parse("(priority:high OR priority:urgent) AND tags:bug").unwrap();
        if let FilterExpr::And(left, _) = expr {
            assert!(matches!(*left, FilterExpr::Or(_, _)));
        } else {
            panic!("Expected And expression");
        }
    }

    #[test]
    fn test_precedence_and_over_or() {
        // a OR b AND c should parse as a OR (b AND c)
        let expr = parse("status:todo OR priority:high AND tags:bug").unwrap();
        if let FilterExpr::Or(_, right) = expr {
            assert!(matches!(*right, FilterExpr::And(_, _)));
        } else {
            panic!("Expected Or expression with And on right");
        }
    }

    #[test]
    fn test_parse_quoted_string() {
        let expr = parse(r#"search:"hello world""#).unwrap();
        if let FilterExpr::Condition(cond) = expr {
            assert!(matches!(cond.value, FilterValue::SearchText(s) if s == "hello world"));
        } else {
            panic!("Expected Condition");
        }
    }

    #[test]
    fn test_parse_all_priorities() {
        for (input, expected) in [
            ("priority:none", Priority::None),
            ("priority:low", Priority::Low),
            ("priority:medium", Priority::Medium),
            ("priority:med", Priority::Medium),
            ("priority:high", Priority::High),
            ("priority:urgent", Priority::Urgent),
        ] {
            let expr = parse(input).unwrap();
            if let FilterExpr::Condition(cond) = expr {
                assert_eq!(cond.value, FilterValue::Priority(expected));
            } else {
                panic!("Expected Condition for {input}");
            }
        }
    }

    #[test]
    fn test_parse_all_statuses() {
        for (input, expected) in [
            ("status:todo", TaskStatus::Todo),
            ("status:in_progress", TaskStatus::InProgress),
            ("status:in-progress", TaskStatus::InProgress),
            ("status:inprogress", TaskStatus::InProgress),
            ("status:blocked", TaskStatus::Blocked),
            ("status:done", TaskStatus::Done),
            ("status:completed", TaskStatus::Done),
            ("status:cancelled", TaskStatus::Cancelled),
            ("status:canceled", TaskStatus::Cancelled),
        ] {
            let expr = parse(input).unwrap();
            if let FilterExpr::Condition(cond) = expr {
                assert_eq!(
                    cond.value,
                    FilterValue::Status(expected),
                    "Failed for {input}"
                );
            } else {
                panic!("Expected Condition for {input}");
            }
        }
    }

    #[test]
    fn test_parse_due_filters() {
        for (input, expected) in [
            ("due:today", DueFilter::Today),
            ("due:tomorrow", DueFilter::Tomorrow),
            ("due:thisweek", DueFilter::ThisWeek),
            ("due:this-week", DueFilter::ThisWeek),
            ("due:nextweek", DueFilter::NextWeek),
            ("due:overdue", DueFilter::Overdue),
            ("due:none", DueFilter::None),
        ] {
            let expr = parse(input).unwrap();
            if let FilterExpr::Condition(cond) = expr {
                assert_eq!(cond.value, FilterValue::Due(expected), "Failed for {input}");
            } else {
                panic!("Expected Condition for {input}");
            }
        }
    }

    #[test]
    fn test_parse_due_specific_date() {
        let expr = parse("due:2025-12-25").unwrap();
        if let FilterExpr::Condition(cond) = expr {
            let expected = NaiveDate::from_ymd_opt(2025, 12, 25).unwrap();
            assert_eq!(cond.value, FilterValue::Due(DueFilter::On(expected)));
        } else {
            panic!("Expected Condition");
        }
    }

    #[test]
    fn test_parse_due_before_date() {
        let expr = parse("due:<2025-12-25").unwrap();
        if let FilterExpr::Condition(cond) = expr {
            let expected = NaiveDate::from_ymd_opt(2025, 12, 25).unwrap();
            assert_eq!(cond.value, FilterValue::Due(DueFilter::Before(expected)));
        } else {
            panic!("Expected Condition");
        }
    }

    #[test]
    fn test_parse_due_after_date() {
        let expr = parse("due:>2025-12-25").unwrap();
        if let FilterExpr::Condition(cond) = expr {
            let expected = NaiveDate::from_ymd_opt(2025, 12, 25).unwrap();
            assert_eq!(cond.value, FilterValue::Due(DueFilter::After(expected)));
        } else {
            panic!("Expected Condition");
        }
    }

    #[test]
    fn test_parse_has_fields() {
        for (input, expected) in [
            ("has:due", HasField::Due),
            ("has:project", HasField::Project),
            ("has:tags", HasField::Tags),
            ("has:tag", HasField::Tags),
            ("has:estimate", HasField::Estimate),
            ("has:description", HasField::Description),
            ("has:desc", HasField::Description),
        ] {
            let expr = parse(input).unwrap();
            if let FilterExpr::Condition(cond) = expr {
                assert_eq!(cond.value, FilterValue::Has(expected), "Failed for {input}");
            } else {
                panic!("Expected Condition for {input}");
            }
        }
    }

    #[test]
    fn test_parse_tag_alias() {
        let expr1 = parse("tags:bug").unwrap();
        let expr2 = parse("tag:bug").unwrap();
        assert_eq!(expr1, expr2);
    }

    #[test]
    fn test_parse_project_partial_match() {
        let expr = parse("project:backend").unwrap();
        if let FilterExpr::Condition(cond) = expr {
            assert_eq!(cond.field, FilterField::Project);
            assert_eq!(cond.value, FilterValue::ProjectName("backend".to_string()));
        } else {
            panic!("Expected Condition");
        }
    }

    #[test]
    fn test_parse_title_search() {
        let expr = parse("title:refactor").unwrap();
        if let FilterExpr::Condition(cond) = expr {
            assert_eq!(cond.field, FilterField::Title);
            assert_eq!(cond.value, FilterValue::TitleText("refactor".to_string()));
        } else {
            panic!("Expected Condition");
        }
    }

    #[test]
    fn test_parse_complex_expression() {
        let expr =
            parse("(priority:high OR priority:urgent) AND !status:done AND tags:work").unwrap();
        // Should be: ((priority:high OR priority:urgent) AND !status:done) AND tags:work
        assert!(matches!(expr, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_parse_empty_string() {
        let result = parse("");
        assert!(matches!(result, Err(ParseError::EmptyExpression)));
    }

    #[test]
    fn test_parse_whitespace_only() {
        let result = parse("   ");
        assert!(matches!(result, Err(ParseError::EmptyExpression)));
    }

    #[test]
    fn test_parse_unknown_field() {
        let result = parse("unknown:value");
        assert!(matches!(result, Err(ParseError::UnknownField { .. })));
    }

    #[test]
    fn test_parse_invalid_priority() {
        let result = parse("priority:extreme");
        assert!(
            matches!(result, Err(ParseError::InvalidValue { field, .. }) if field == "priority")
        );
    }

    #[test]
    fn test_parse_invalid_status() {
        let result = parse("status:running");
        assert!(matches!(result, Err(ParseError::InvalidValue { field, .. }) if field == "status"));
    }

    #[test]
    fn test_parse_invalid_due() {
        let result = parse("due:sometime");
        assert!(matches!(result, Err(ParseError::InvalidValue { field, .. }) if field == "due"));
    }

    #[test]
    fn test_parse_case_insensitive_fields() {
        let expr1 = parse("PRIORITY:high").unwrap();
        let expr2 = parse("Priority:high").unwrap();
        let expr3 = parse("priority:high").unwrap();
        assert_eq!(expr1, expr2);
        assert_eq!(expr2, expr3);
    }

    #[test]
    fn test_parse_case_insensitive_values() {
        let expr1 = parse("priority:HIGH").unwrap();
        let expr2 = parse("priority:High").unwrap();
        let expr3 = parse("priority:high").unwrap();
        assert_eq!(expr1, expr2);
        assert_eq!(expr2, expr3);
    }

    #[test]
    fn test_double_negation() {
        let expr = parse("!!status:done").unwrap();
        if let FilterExpr::Not(inner) = expr {
            assert!(matches!(*inner, FilterExpr::Not(_)));
        } else {
            panic!("Expected Not expression");
        }
    }

    #[test]
    fn test_nested_parentheses() {
        let expr = parse("((priority:high))").unwrap();
        assert!(matches!(expr, FilterExpr::Condition(_)));
    }

    #[test]
    fn test_parse_created_filters() {
        for (input, expected) in [
            ("created:today", CreatedFilter::Today),
            ("created:yesterday", CreatedFilter::Yesterday),
            ("created:thisweek", CreatedFilter::ThisWeek),
            ("created:this-week", CreatedFilter::ThisWeek),
            ("created:lastweek", CreatedFilter::LastWeek),
            ("created:last-week", CreatedFilter::LastWeek),
        ] {
            let expr = parse(input).unwrap();
            if let FilterExpr::Condition(cond) = expr {
                assert_eq!(
                    cond.value,
                    FilterValue::Created(expected),
                    "Failed for {input}"
                );
            } else {
                panic!("Expected Condition for {input}");
            }
        }
    }

    #[test]
    fn test_parse_created_specific_date() {
        let expr = parse("created:2025-01-15").unwrap();
        if let FilterExpr::Condition(cond) = expr {
            let expected = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
            assert_eq!(
                cond.value,
                FilterValue::Created(CreatedFilter::On(expected))
            );
        } else {
            panic!("Expected Condition");
        }
    }

    #[test]
    fn test_parse_created_before_date() {
        let expr = parse("created:<2025-01-15").unwrap();
        if let FilterExpr::Condition(cond) = expr {
            let expected = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
            assert_eq!(
                cond.value,
                FilterValue::Created(CreatedFilter::Before(expected))
            );
        } else {
            panic!("Expected Condition");
        }
    }

    #[test]
    fn test_parse_created_after_date() {
        let expr = parse("created:>2025-01-15").unwrap();
        if let FilterExpr::Condition(cond) = expr {
            let expected = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
            assert_eq!(
                cond.value,
                FilterValue::Created(CreatedFilter::After(expected))
            );
        } else {
            panic!("Expected Condition");
        }
    }

    // ========================================================================
    // Range syntax tests
    // ========================================================================

    #[test]
    fn test_parse_due_date_range() {
        let expr = parse("due:2025-01-01..2025-01-31").unwrap();
        if let FilterExpr::Condition(cond) = expr {
            let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
            let end = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();
            assert_eq!(cond.value, FilterValue::Due(DueFilter::Between(start, end)));
        } else {
            panic!("Expected Condition");
        }
    }

    #[test]
    fn test_parse_due_open_start_range() {
        let expr = parse("due:2025-06-01..").unwrap();
        if let FilterExpr::Condition(cond) = expr {
            let date = NaiveDate::from_ymd_opt(2025, 6, 1).unwrap();
            assert_eq!(cond.value, FilterValue::Due(DueFilter::OnOrAfter(date)));
        } else {
            panic!("Expected Condition");
        }
    }

    #[test]
    fn test_parse_due_open_end_range() {
        let expr = parse("due:..2025-12-31").unwrap();
        if let FilterExpr::Condition(cond) = expr {
            let date = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();
            assert_eq!(cond.value, FilterValue::Due(DueFilter::OnOrBefore(date)));
        } else {
            panic!("Expected Condition");
        }
    }

    #[test]
    fn test_parse_due_invalid_range_end_before_start() {
        let result = parse("due:2025-12-31..2025-01-01");
        assert!(matches!(result, Err(ParseError::InvalidValue { field, .. }) if field == "due"));
    }

    #[test]
    fn test_parse_created_date_range() {
        let expr = parse("created:2025-01-01..2025-03-31").unwrap();
        if let FilterExpr::Condition(cond) = expr {
            let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
            let end = NaiveDate::from_ymd_opt(2025, 3, 31).unwrap();
            assert_eq!(
                cond.value,
                FilterValue::Created(CreatedFilter::Between(start, end))
            );
        } else {
            panic!("Expected Condition");
        }
    }

    #[test]
    fn test_parse_created_open_start_range() {
        let expr = parse("created:2025-01-01..").unwrap();
        if let FilterExpr::Condition(cond) = expr {
            let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
            assert_eq!(
                cond.value,
                FilterValue::Created(CreatedFilter::OnOrAfter(date))
            );
        } else {
            panic!("Expected Condition");
        }
    }

    #[test]
    fn test_parse_created_open_end_range() {
        let expr = parse("created:..2025-06-30").unwrap();
        if let FilterExpr::Condition(cond) = expr {
            let date = NaiveDate::from_ymd_opt(2025, 6, 30).unwrap();
            assert_eq!(
                cond.value,
                FilterValue::Created(CreatedFilter::OnOrBefore(date))
            );
        } else {
            panic!("Expected Condition");
        }
    }

    #[test]
    fn test_parse_scheduled_date_range() {
        let expr = parse("scheduled:2025-02-01..2025-02-28").unwrap();
        if let FilterExpr::Condition(cond) = expr {
            let start = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
            let end = NaiveDate::from_ymd_opt(2025, 2, 28).unwrap();
            assert_eq!(
                cond.value,
                FilterValue::Scheduled(ScheduledFilter::Between(start, end))
            );
        } else {
            panic!("Expected Condition");
        }
    }

    #[test]
    fn test_parse_scheduled_open_start_range() {
        let expr = parse("scheduled:2025-03-01..").unwrap();
        if let FilterExpr::Condition(cond) = expr {
            let date = NaiveDate::from_ymd_opt(2025, 3, 1).unwrap();
            assert_eq!(
                cond.value,
                FilterValue::Scheduled(ScheduledFilter::OnOrAfter(date))
            );
        } else {
            panic!("Expected Condition");
        }
    }

    #[test]
    fn test_parse_scheduled_open_end_range() {
        let expr = parse("scheduled:..2025-04-30").unwrap();
        if let FilterExpr::Condition(cond) = expr {
            let date = NaiveDate::from_ymd_opt(2025, 4, 30).unwrap();
            assert_eq!(
                cond.value,
                FilterValue::Scheduled(ScheduledFilter::OnOrBefore(date))
            );
        } else {
            panic!("Expected Condition");
        }
    }

    #[test]
    fn test_parse_estimate_numeric_range() {
        let expr = parse("estimate:30..120").unwrap();
        if let FilterExpr::Condition(cond) = expr {
            assert_eq!(
                cond.value,
                FilterValue::Estimate(NumericFilter::Between(30, 120))
            );
        } else {
            panic!("Expected Condition");
        }
    }

    #[test]
    fn test_parse_estimate_open_start_range() {
        let expr = parse("estimate:60..").unwrap();
        if let FilterExpr::Condition(cond) = expr {
            assert_eq!(
                cond.value,
                FilterValue::Estimate(NumericFilter::GreaterOrEqual(60))
            );
        } else {
            panic!("Expected Condition");
        }
    }

    #[test]
    fn test_parse_estimate_open_end_range() {
        let expr = parse("estimate:..30").unwrap();
        if let FilterExpr::Condition(cond) = expr {
            assert_eq!(
                cond.value,
                FilterValue::Estimate(NumericFilter::LessOrEqual(30))
            );
        } else {
            panic!("Expected Condition");
        }
    }

    #[test]
    fn test_parse_actual_numeric_range() {
        let expr = parse("actual:15..90").unwrap();
        if let FilterExpr::Condition(cond) = expr {
            assert_eq!(
                cond.value,
                FilterValue::Actual(NumericFilter::Between(15, 90))
            );
        } else {
            panic!("Expected Condition");
        }
    }

    #[test]
    fn test_parse_estimate_invalid_range_end_before_start() {
        let result = parse("estimate:120..30");
        assert!(
            matches!(result, Err(ParseError::InvalidValue { field, .. }) if field == "estimate")
        );
    }

    #[test]
    fn test_parse_invalid_date_range_format() {
        // Invalid date format in range
        let result = parse("due:2025-01-01..invalid");
        assert!(matches!(result, Err(ParseError::InvalidValue { .. })));
    }

    #[test]
    fn test_parse_empty_range() {
        // Just ".." should fail - lexer rejects it as unexpected character
        let result = parse("due:..");
        assert!(result.is_err());
        // Either UnexpectedChar (from lexer) or InvalidValue (from parser) is acceptable
        assert!(
            matches!(result, Err(ParseError::UnexpectedChar { .. }))
                || matches!(result, Err(ParseError::InvalidValue { .. }))
        );
    }

    #[test]
    fn test_parse_range_same_date() {
        // Same start and end should be allowed (single day range)
        let expr = parse("due:2025-06-15..2025-06-15").unwrap();
        if let FilterExpr::Condition(cond) = expr {
            let date = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
            assert_eq!(cond.value, FilterValue::Due(DueFilter::Between(date, date)));
        } else {
            panic!("Expected Condition");
        }
    }

    #[test]
    fn test_parse_range_same_number() {
        // Same start and end should be allowed
        let expr = parse("estimate:60..60").unwrap();
        if let FilterExpr::Condition(cond) = expr {
            assert_eq!(
                cond.value,
                FilterValue::Estimate(NumericFilter::Between(60, 60))
            );
        } else {
            panic!("Expected Condition");
        }
    }
}
