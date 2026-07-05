//! Byte-offset ↔ LSP position conversion for both wire encodings.
//!
//! All internal work uses UTF-8 byte offsets; positions cross the wire in
//! the encoding negotiated at initialize time (UTF-8 when the client offers
//! it, UTF-16 otherwise).

use lsp_types::Position;

/// The position encoding negotiated with the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionEncoding {
    /// `character` counts UTF-8 bytes.
    Utf8,
    /// `character` counts UTF-16 code units (the LSP default).
    Utf16,
}

/// Byte offsets of every line start, for position conversion.
#[derive(Debug)]
pub struct LineIndex {
    /// Byte offset where each line begins; always starts with `0`.
    line_starts: Vec<usize>,
    /// Total text length in bytes.
    len: usize,
}

impl LineIndex {
    /// Index `text`'s line starts.
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![0];
        line_starts.extend(
            text.bytes()
                .enumerate()
                .filter(|&(_, b)| b == b'\n')
                .map(|(i, _)| i + 1),
        );
        Self {
            line_starts,
            len: text.len(),
        }
    }

    /// The byte range of line `line` (clamped), excluding its newline.
    fn line_span(&self, text: &str, line: usize) -> (usize, usize) {
        let line = line.min(self.line_starts.len() - 1);
        let start = self.line_starts[line];
        let end = self
            .line_starts
            .get(line + 1)
            .map_or(self.len, |&next| next);
        let content = text[start..end].trim_end_matches(['\n', '\r']);
        (start, start + content.len())
    }

    /// Convert an LSP position to a byte offset into `text` (the same text
    /// this index was built from). Out-of-range positions clamp.
    pub fn offset(&self, text: &str, position: Position, encoding: PositionEncoding) -> usize {
        let (start, end) = self.line_span(text, position.line as usize);
        let target = position.character as usize;
        match encoding {
            PositionEncoding::Utf8 => {
                // Snap into the line and back to a char boundary.
                let mut offset = (start + target).min(end);
                while offset > start && !text.is_char_boundary(offset) {
                    offset -= 1;
                }
                offset
            }
            PositionEncoding::Utf16 => {
                let mut units = 0usize;
                for (i, c) in text[start..end].char_indices() {
                    if units >= target {
                        return start + i;
                    }
                    units += c.len_utf16();
                }
                end
            }
        }
    }

    /// Convert a byte offset into `text` to an LSP position. Offsets snap
    /// down to the nearest char boundary; out-of-range offsets clamp.
    pub fn position(&self, text: &str, offset: usize, encoding: PositionEncoding) -> Position {
        let mut offset = offset.min(self.len);
        while offset > 0 && !text.is_char_boundary(offset) {
            offset -= 1;
        }
        let line = self.line_starts.partition_point(|&s| s <= offset) - 1;
        let start = self.line_starts[line];
        let character = match encoding {
            PositionEncoding::Utf8 => offset - start,
            PositionEncoding::Utf16 => text[start..offset].chars().map(char::len_utf16).sum(),
        };
        Position {
            line: clamp_u32(line),
            character: clamp_u32(character),
        }
    }
}

/// Saturating usize → u32 for LSP wire values.
fn clamp_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `"日"` is 3 UTF-8 bytes and 1 UTF-16 unit; `"𝕏"` is 4 and 2.
    const TEXT: &str = "ab\n日x𝕏y\n\nend";

    fn index() -> LineIndex {
        LineIndex::new(TEXT)
    }

    fn pos(line: u32, character: u32) -> Position {
        Position { line, character }
    }

    #[test]
    fn utf8_round_trip() {
        let idx = index();
        for (line, character, offset) in [(0, 0, 0), (0, 2, 2), (1, 3, 6), (1, 8, 11), (3, 3, 17)] {
            assert_eq!(
                idx.offset(TEXT, pos(line, character), PositionEncoding::Utf8),
                offset
            );
            assert_eq!(
                idx.position(TEXT, offset, PositionEncoding::Utf8),
                pos(line, character)
            );
        }
    }

    #[test]
    fn utf16_round_trip() {
        let idx = index();
        // Line 1 is `日x𝕏y`: units 0,1,2,4,5 ↔ bytes 3,6,7,11,12.
        for (character, offset) in [(0, 3), (1, 6), (2, 7), (4, 11), (5, 12)] {
            assert_eq!(
                idx.offset(TEXT, pos(1, character), PositionEncoding::Utf16),
                offset
            );
            assert_eq!(
                idx.position(TEXT, offset, PositionEncoding::Utf16),
                pos(1, character)
            );
        }
    }

    #[test]
    fn positions_clamp_to_line_and_document() {
        let idx = index();
        // Past the end of a line clamps to the newline, not into it.
        assert_eq!(idx.offset(TEXT, pos(0, 99), PositionEncoding::Utf8), 2);
        assert_eq!(idx.offset(TEXT, pos(1, 99), PositionEncoding::Utf16), 12);
        // Past the last line clamps to the document end.
        assert_eq!(idx.offset(TEXT, pos(9, 9), PositionEncoding::Utf8), 17);
        assert_eq!(idx.position(TEXT, 999, PositionEncoding::Utf8), pos(3, 3));
    }

    #[test]
    fn utf8_character_snaps_to_char_boundary() {
        let idx = index();
        // Byte 4 is inside `日`; snap back to its start.
        assert_eq!(idx.offset(TEXT, pos(1, 1), PositionEncoding::Utf8), 3);
        assert_eq!(idx.position(TEXT, 4, PositionEncoding::Utf8), pos(1, 0),);
    }

    #[test]
    fn crlf_line_ends_before_carriage_return() {
        let text = "ab\r\ncd";
        let idx = LineIndex::new(text);
        assert_eq!(idx.offset(text, pos(0, 99), PositionEncoding::Utf8), 2);
        assert_eq!(idx.offset(text, pos(1, 1), PositionEncoding::Utf8), 5);
    }
}
