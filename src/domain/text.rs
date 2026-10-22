//! Character-safe text helpers shared by the app and UI layers.
//!
//! Slicing a `String` by byte index (`&s[..n]`) panics when `n` falls inside a
//! multi-byte character, so every display truncation should go through these
//! helpers, which count characters instead of bytes.

/// Truncate `s` to at most `max_chars` characters, appending `suffix` if anything
/// was cut. The returned string, suffix included, never exceeds `max_chars`
/// characters.
///
/// If `max_chars` is smaller than the suffix, the suffix itself is truncated.
#[must_use]
pub fn truncate_chars(s: &str, max_chars: usize, suffix: &str) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let suffix_len = suffix.chars().count();
    if max_chars <= suffix_len {
        return suffix.chars().take(max_chars).collect();
    }
    let keep = max_chars - suffix_len;
    let mut out: String = s.chars().take(keep).collect();
    out.push_str(suffix);
    out
}

/// Like [`truncate_chars`] but keeps the *end* of the string, prefixing `prefix`
/// when anything was cut. Useful for file paths where the tail matters most.
#[must_use]
pub fn truncate_chars_start(s: &str, max_chars: usize, prefix: &str) -> String {
    let total = s.chars().count();
    if total <= max_chars {
        return s.to_string();
    }
    let prefix_len = prefix.chars().count();
    if max_chars <= prefix_len {
        return prefix.chars().take(max_chars).collect();
    }
    let keep = max_chars - prefix_len;
    let mut out = String::from(prefix);
    out.extend(s.chars().skip(total - keep));
    out
}

/// Byte offset of the character at index `char_idx` in `s`, clamped to `s.len()`.
///
/// Converts a character-based cursor column into a byte offset that is always a
/// valid slice boundary.
#[must_use]
pub fn byte_offset(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map_or(s.len(), |(byte_idx, _)| byte_idx)
}

/// Number of characters in `s` (what the UI treats as its length).
#[must_use]
pub fn char_len(s: &str) -> usize {
    s.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_chars_leaves_short_strings_alone() {
        assert_eq!(truncate_chars("hello", 10, "..."), "hello");
        assert_eq!(truncate_chars("hello", 5, "..."), "hello");
    }

    #[test]
    fn truncate_chars_respects_total_budget() {
        assert_eq!(truncate_chars("hello world", 8, "..."), "hello...");
        assert_eq!(truncate_chars("hello world", 6, "…"), "hello…");
    }

    #[test]
    fn truncate_chars_counts_characters_not_bytes() {
        // Each of these is multi-byte; a byte slice at 5 would panic.
        assert_eq!(truncate_chars("héllo wörld", 8, "..."), "héllo...");
        assert_eq!(truncate_chars("日本語のテキスト", 4, "…"), "日本語…");
        assert_eq!(truncate_chars("🎉🎉🎉🎉🎉", 3, ".."), "🎉..");
    }

    #[test]
    fn truncate_chars_tiny_budget_truncates_suffix() {
        assert_eq!(truncate_chars("hello world", 2, "..."), "..");
        assert_eq!(truncate_chars("hello world", 0, "..."), "");
    }

    #[test]
    fn truncate_chars_start_keeps_tail() {
        assert_eq!(truncate_chars_start("src/a/b.rs", 20, "..."), "src/a/b.rs");
        assert_eq!(
            truncate_chars_start("src/auth/middleware.rs", 10, "..."),
            "...ware.rs"
        );
        assert_eq!(
            truncate_chars_start("ünïcödé/päth.rs", 8, "..."),
            "...th.rs"
        );
    }

    #[test]
    fn byte_offset_maps_char_index_to_boundary() {
        let s = "héllo";
        assert_eq!(byte_offset(s, 0), 0);
        assert_eq!(byte_offset(s, 1), 1);
        assert_eq!(byte_offset(s, 2), 3); // 'é' is two bytes
        assert_eq!(byte_offset(s, 5), s.len());
        assert_eq!(byte_offset(s, 99), s.len());
        assert_eq!(byte_offset("", 3), 0);
    }

    #[test]
    fn char_len_counts_chars() {
        assert_eq!(char_len("héllo"), 5);
        assert_eq!(char_len("🎉🎉"), 2);
        assert_eq!(char_len(""), 0);
    }
}
