use std::sync::Arc;

use oxc::span::Span;

#[derive(Clone, Debug)]
pub struct SourceTextInfo {
    text: Arc<str>,
    line_starts: Vec<usize>,
}

impl SourceTextInfo {
    pub fn new(text: Arc<str>) -> Self {
        let mut line_starts = vec![0];
        let mut chars = text.char_indices().peekable();
        while let Some((index, ch)) = chars.next() {
            match ch {
                '\r' => {
                    if chars.peek().is_some_and(|(_, next)| *next == '\n') {
                        if let Some((newline_index, _)) = chars.next() {
                            line_starts.push(newline_index + 1);
                        }
                    } else {
                        line_starts.push(index + ch.len_utf8());
                    }
                }
                '\n' | '\u{2028}' | '\u{2029}' => {
                    line_starts.push(index + ch.len_utf8());
                }
                _ => {}
            }
        }
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

    pub fn lines_count(&self) -> usize {
        self.line_starts.len()
    }

    pub fn line_index(&self, byte_pos: usize) -> usize {
        match self.line_starts.binary_search(&byte_pos) {
            Ok(line) => line,
            Err(line) => line - 1,
        }
    }

    pub fn line_start(&self, line_index: usize) -> usize {
        self.line_starts[line_index]
    }

    pub fn line_end(&self, line_index: usize) -> usize {
        if line_index + 1 < self.line_starts.len() {
            let end = self.line_starts[line_index + 1];
            let Some(ch) = self.text[..end].chars().next_back() else {
                return end;
            };
            let break_len = if ch == '\n' && self.text[..end - ch.len_utf8()].ends_with('\r') {
                2
            } else {
                ch.len_utf8()
            };
            end - break_len
        } else {
            self.text.len()
        }
    }

    pub fn span_text(&self, span: Span) -> &str {
        &self.text[span.start as usize..span.end as usize]
    }

    pub fn range_text(&self, start: usize, end: usize) -> &str {
        &self.text[start..end]
    }

    pub fn line_text(&self, line_index: usize) -> &str {
        let start = self.line_start(line_index);
        let end = self.line_end(line_index);
        &self.text[start..end]
    }

    pub fn line_and_column_display(&self, byte_pos: usize) -> LineAndColumnDisplay {
        self.line_and_column_display_with_indent_width(byte_pos, 4)
    }

    pub fn line_and_column_display_with_indent_width(
        &self,
        byte_pos: usize,
        indent_width: usize,
    ) -> LineAndColumnDisplay {
        let line_index = self.line_index(byte_pos);
        let line_start = self.line_start(line_index);
        let line_end = self.line_end(line_index);
        let byte_pos = byte_pos.min(line_end);
        let column = self.text[line_start..byte_pos]
            .chars()
            .map(|c| if c == '\t' { indent_width } else { 1 })
            .sum::<usize>();
        LineAndColumnDisplay { line_number: line_index + 1, column_number: column + 1 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineAndColumnDisplay {
    pub line_number: usize,
    pub column_number: usize,
}

#[cfg(test)]
#[expect(clippy::unwrap_used)]
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

    #[test]
    fn line_endings_follow_ecmascript() {
        let info = SourceTextInfo::from_string("a\r\nb\rc\u{2028}d\u{2029}e".to_string());
        assert_eq!(info.lines_count(), 5);
        assert_eq!(
            (0..info.lines_count()).map(|i| info.line_text(i)).collect::<Vec<_>>(),
            ["a", "b", "c", "d", "e"]
        );
        assert_eq!(info.line_index(2), 0);
        assert_eq!(info.line_index(3), 1);
        assert_eq!(info.line_index(5), 2);
    }

    #[test]
    fn line_column_ignores_crlf_bytes() {
        let info = SourceTextInfo::from_string("foo\r\nbar".to_string());
        assert_eq!(
            info.line_and_column_display(4),
            LineAndColumnDisplay { line_number: 1, column_number: 4 }
        );
    }
}
