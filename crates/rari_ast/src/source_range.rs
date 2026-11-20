// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use std::sync::Arc;

use oxc::span::Span;

/// Wrapper around source text that provides line/column info.
#[derive(Clone, Debug)]
pub struct SourceTextInfo {
    text: Arc<str>,
    /// Byte offsets of line starts (lazily computed would be better,
    /// but for simplicity we compute eagerly).
    line_starts: Vec<usize>,
}

impl SourceTextInfo {
    pub fn new(text: Arc<str>) -> Self {
        let line_starts =
            std::iter::once(0).chain(text.match_indices('\n').map(|(i, _)| i + 1)).collect();
        Self { text, line_starts }
    }

    pub fn from_string(text: String) -> Self {
        Self::new(text.into())
    }

    pub fn text(&self) -> &Arc<str> {
        &self.text
    }

    pub fn text_str(&self) -> &str {
        &self.text
    }

    /// Number of lines in the source.
    pub fn lines_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Get the 0-indexed line index for a byte position.
    pub fn line_index(&self, byte_pos: usize) -> usize {
        match self.line_starts.binary_search(&byte_pos) {
            Ok(line) => line,
            Err(line) => line - 1,
        }
    }

    /// Get the byte offset of the start of a line (0-indexed).
    pub fn line_start(&self, line_index: usize) -> usize {
        self.line_starts[line_index]
    }

    /// Get the byte offset of the end of a line (0-indexed).
    pub fn line_end(&self, line_index: usize) -> usize {
        if line_index + 1 < self.line_starts.len() {
            let mut end = self.line_starts[line_index + 1];
            // exclude \n and \r\n line endings
            if end > 0 && self.text.as_bytes().get(end - 1) == Some(&b'\n') {
                end -= 1;
            }
            if end > 0 && self.text.as_bytes().get(end - 1) == Some(&b'\r') {
                end -= 1;
            }
            end
        } else {
            self.text.len()
        }
    }

    /// Get text for a span.
    pub fn span_text(&self, span: Span) -> &str {
        &self.text[span.start as usize..span.end as usize]
    }

    /// Get text for a byte range.
    pub fn range_text(&self, start: usize, end: usize) -> &str {
        &self.text[start..end]
    }

    /// Get the text of a line (0-indexed).
    pub fn line_text(&self, line_index: usize) -> &str {
        let start = self.line_start(line_index);
        let end = self.line_end(line_index);
        &self.text[start..end]
    }

    /// Get 1-indexed line and column display for a byte position, counting a
    /// tab as 4 columns.
    pub fn line_and_column_display(&self, byte_pos: usize) -> LineAndColumnDisplay {
        self.line_and_column_display_with_indent_width(byte_pos, 4)
    }

    /// Get 1-indexed line and column display for a byte position, counting a
    /// tab as `indent_width` columns and every other character as one.
    pub fn line_and_column_display_with_indent_width(
        &self,
        byte_pos: usize,
        indent_width: usize,
    ) -> LineAndColumnDisplay {
        let line_index = self.line_index(byte_pos);
        let line_start = self.line_start(line_index);
        let column = self.text[line_start..byte_pos]
            .chars()
            .map(|c| if c == '\t' { indent_width } else { 1 })
            .sum::<usize>();
        LineAndColumnDisplay { line_number: line_index + 1, column_number: column + 1 }
    }
}

/// 1-indexed line and column for display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineAndColumnDisplay {
    pub line_number: usize,
    pub column_number: usize,
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "Ported code not yet updated to the workspace lint rules")]
mod test {
    use super::*;

    #[test]
    fn line_and_column_display_tabs() {
        let info = SourceTextInfo::from_string("a\n\t\tb".to_string());
        let pos = info.text_str().find('b').unwrap();
        assert_eq!(
            info.line_and_column_display(pos),
            LineAndColumnDisplay { line_number: 2, column_number: 9 }
        );
        assert_eq!(
            info.line_and_column_display_with_indent_width(pos, 2),
            LineAndColumnDisplay { line_number: 2, column_number: 5 }
        );
    }
}
