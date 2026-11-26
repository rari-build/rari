// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use std::{borrow::Cow, fmt, fmt::Write as _};

use deno_error::JsError;
use oxc::{
    diagnostics::{OxcCode, OxcDiagnostic},
    span::Span,
};

use crate::{
    LineAndColumnDisplay, ModuleSpecifier, SourceTextInfo,
    diagnostics::{
        Diagnostic, DiagnosticLevel, DiagnosticLocation, DiagnosticSnippet,
        DiagnosticSnippetHighlight, DiagnosticSnippetHighlightStyle, DiagnosticSourcePos,
        DiagnosticSourceRange,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseDiagnosticCode<'a> {
    scope: Option<&'a str>,
    number: Option<&'a str>,
}

impl<'a> ParseDiagnosticCode<'a> {
    pub fn scope(&self) -> Option<&'a str> {
        self.scope
    }

    pub fn number(&self) -> Option<&'a str> {
        self.number
    }
}

/// Parsing diagnostic.
#[derive(Debug, Clone, JsError)]
#[class(syntax)]
pub struct ParseDiagnostic(pub(crate) Box<ParseDiagnosticInner>);

#[derive(Debug, Clone)]
pub(crate) struct ParseDiagnosticInner {
    pub specifier: ModuleSpecifier,
    pub span: Span,
    pub message: String,
    pub code: OxcCode,
    pub source: SourceTextInfo,
}

impl Eq for ParseDiagnostic {}

impl PartialEq for ParseDiagnostic {
    fn eq(&self, other: &Self) -> bool {
        self.0.specifier == other.0.specifier
            && self.0.span == other.0.span
            && self.0.message == other.0.message
    }
}

impl ParseDiagnostic {
    /// Specifier of the source the diagnostic occurred in.
    pub fn specifier(&self) -> &ModuleSpecifier {
        &self.0.specifier
    }

    pub fn span(&self) -> Span {
        self.0.span
    }

    pub fn message(&self) -> &str {
        &self.0.message
    }

    pub fn diagnostic_code(&self) -> Option<ParseDiagnosticCode<'_>> {
        if self.0.code.is_some() {
            Some(ParseDiagnosticCode {
                scope: self.0.code.scope.as_deref(),
                number: self.0.code.number.as_deref(),
            })
        } else {
            None
        }
    }

    pub(crate) fn source(&self) -> &SourceTextInfo {
        &self.0.source
    }

    pub fn is_fatal(&self) -> bool {
        true
    }

    /// 1-indexed display position the diagnostic occurred at.
    pub fn display_position(&self) -> LineAndColumnDisplay {
        self.0.source.line_and_column_display(self.0.span.start as usize)
    }

    #[expect(clippy::needless_pass_by_value)]
    pub fn from_oxc_diagnostic(
        err: OxcDiagnostic,
        specifier: &ModuleSpecifier,
        source: SourceTextInfo,
    ) -> Self {
        let span = err
            .labels
            .first()
            .map(|label| Span::new(label.offset(), label.offset() + label.len()))
            .unwrap_or(Span::new(0, 0));
        let span = adjust_empty_span_at_trailing_whitespace(span, &source);

        Self(Box::new(ParseDiagnosticInner {
            specifier: specifier.clone(),
            span,
            message: err.message.to_string(),
            code: err.code.clone(),
            source,
        }))
    }

    pub fn from_message(
        message: String,
        specifier: &ModuleSpecifier,
        source: SourceTextInfo,
        span: Span,
    ) -> Self {
        Self(Box::new(ParseDiagnosticInner {
            specifier: specifier.clone(),
            span,
            message,
            code: OxcCode::default(),
            source,
        }))
    }
}

fn adjust_empty_span_at_trailing_whitespace(span: Span, source: &SourceTextInfo) -> Span {
    if span.start != span.end {
        return span;
    }
    let text = source.text_str();
    let Some(preceding) = text.get(..span.start as usize) else {
        return span;
    };
    let trimmed_len = preceding.trim_end().len();
    if trimmed_len == preceding.len() || trimmed_len == 0 {
        return span;
    }
    if source.line_index(trimmed_len) == source.line_index(span.start as usize) {
        return span;
    }
    Span::new(trimmed_len as u32, trimmed_len as u32)
}

impl Diagnostic for ParseDiagnostic {
    fn level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Error
    }

    fn code(&self) -> Cow<'_, str> {
        if self.0.code.is_some() {
            Cow::Owned(self.0.code.to_string())
        } else {
            Cow::Borrowed("parse-error")
        }
    }

    fn message(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.0.message)
    }

    fn location(&self) -> DiagnosticLocation<'_> {
        DiagnosticLocation::ModulePosition {
            specifier: Cow::Borrowed(self.specifier()),
            source_pos: DiagnosticSourcePos::ByteIndex(self.0.span.start as usize),
            text_info: Cow::Borrowed(self.source()),
        }
    }

    fn snippet(&self) -> Option<DiagnosticSnippet<'_>> {
        let span = self.span();
        Some(DiagnosticSnippet {
            source: Cow::Borrowed(self.source()),
            highlights: vec![DiagnosticSnippetHighlight {
                style: DiagnosticSnippetHighlightStyle::Error,
                range: DiagnosticSourceRange {
                    start: DiagnosticSourcePos::ByteIndex(span.start as usize),
                    end: DiagnosticSourcePos::ByteIndex(span.end as usize),
                },
                description: None,
            }],
        })
    }

    fn hint(&self) -> Option<Cow<'_, str>> {
        None
    }

    fn snippet_fixed(&self) -> Option<DiagnosticSnippet<'_>> {
        None
    }

    fn info(&self) -> Cow<'_, [Cow<'_, str>]> {
        Cow::Borrowed(&[])
    }

    fn docs_url(&self) -> Option<Cow<'_, str>> {
        None
    }
}

impl std::error::Error for ParseDiagnostic {}

/// A single entry in a source code highlight.
enum HighlightEntry {
    /// A source line with its underline indicator.
    SourceLine { line_number: usize, text: String, underline: String },
    /// An ellipsis indicating skipped lines.
    Ellipsis,
}

/// Formats highlight entries with line numbers and pipe margins.
/// Output format:
/// ```text
///   |
/// N | {source_line}
///   | {underline}
/// ```
fn format_highlight_entries(entries: &[HighlightEntry]) -> String {
    let line_number_width = entries
        .iter()
        .filter_map(|entry| match entry {
            HighlightEntry::SourceLine { line_number, .. } => Some(line_number.to_string().len()),
            HighlightEntry::Ellipsis => None,
        })
        .max()
        .unwrap_or(1);
    let padding = " ".repeat(line_number_width);
    let mut result = format!("{padding} |\n");

    let entry_count = entries.len();
    for (i, entry) in entries.iter().enumerate() {
        match entry {
            HighlightEntry::SourceLine { line_number, text, underline } => {
                let _ = writeln!(result, "{line_number:>line_number_width$} | {text}");
                let _ = write!(result, "{padding} | {underline}");
            }
            HighlightEntry::Ellipsis => {
                let _ = write!(result, "{padding} | ...");
            }
        }
        if i + 1 < entry_count {
            result.push('\n');
        }
    }

    result
}

impl fmt::Display for ParseDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let display_position = self.display_position();
        let message = self.message();
        let location = format!(
            "{}:{}:{}",
            self.specifier(),
            display_position.line_number,
            display_position.column_number,
        );

        write!(f, "SyntaxError: {message}")?;

        // todo(dsherret): remove this catch unwind once we've
        // tested this out a lot
        let snippet_result = std::panic::catch_unwind(|| {
            let (entries, _) = get_range_highlight_entries(self.source(), self.span());
            format_highlight_entries(&entries)
        });

        match snippet_result {
            Ok(snippet) => {
                write!(f, "\n{snippet}")?;
            }
            Err(err) => {
                write!(f, "\nBug in rari_ast. Please report this issue: {err:?}")?;
            }
        }

        write!(f, "\n    at {location}")
    }
}

/// Code in this function was adapted from:
/// https://github.com/dprint/dprint/blob/a026a1350d27a61ea18207cb31897b18eaab51a1/crates/core/src/formatting/utils/string_utils.rs#L62
fn get_range_highlight_entries(
    source: &SourceTextInfo,
    byte_range: Span,
) -> (Vec<HighlightEntry>, usize) {
    fn get_column_index_of_pos(source: &SourceTextInfo, line_index: usize, pos: usize) -> usize {
        let line_start = source.line_start(line_index);
        let line_end = source.line_end(line_index);
        source.range_text(line_start, pos.clamp(line_start, line_end)).chars().count()
    }

    let start = byte_range.start as usize;
    let end = byte_range.end as usize;
    let start_line_index = source.line_index(start);
    let end_line_index = source.line_index(end);
    let mut first_line_index = start_line_index;
    while first_line_index > 0 && source.line_text(first_line_index).trim().is_empty() {
        first_line_index -= 1;
    }

    let mut entries = Vec::new();
    for (i, line_index) in (first_line_index..=end_line_index).enumerate() {
        let line = source.line_text(line_index);
        let is_last_line = line_index == end_line_index;
        if i > 2 && !is_last_line {
            continue;
        }
        if i == 2 && !is_last_line {
            entries.push(HighlightEntry::Ellipsis);
            continue;
        }

        let has_highlight = (start_line_index..=end_line_index).contains(&line_index);
        let mut error_start_char_index = if has_highlight && line_index == start_line_index {
            get_column_index_of_pos(source, line_index, start)
        } else {
            0
        };
        let mut error_end_char_index = if !has_highlight {
            0
        } else if is_last_line {
            get_column_index_of_pos(source, line_index, end)
        } else {
            line.chars().count()
        };
        let line_char_count = line.chars().count();
        let text = if line_char_count > 90 {
            let start_char_index = if error_start_char_index > 60 {
                std::cmp::min(error_start_char_index - 20, line_char_count - 80)
            } else {
                0
            };
            error_start_char_index -= start_char_index;
            error_end_char_index -= start_char_index;
            let code_text = line.chars().skip(start_char_index).take(80).collect::<String>();
            let mut line_text = String::new();
            if start_char_index > 0 {
                line_text.push_str("...");
                error_start_char_index += 3;
                error_end_char_index += 3;
            }
            line_text.push_str(&code_text);
            if line_char_count > start_char_index + code_text.chars().count() {
                error_end_char_index =
                    std::cmp::min(error_end_char_index, line_text.chars().count());
                line_text.push_str("...");
            }
            line_text
        } else {
            line.to_string()
        };

        let underline = if has_highlight {
            format!(
                "{}{}",
                " ".repeat(error_start_char_index),
                "~".repeat(std::cmp::max(1, error_end_char_index - error_start_char_index,))
            )
        } else {
            String::new()
        };

        entries.push(HighlightEntry::SourceLine { line_number: line_index + 1, text, underline });
    }
    (entries, first_line_index)
}

/// Formats a range highlight as a plain string (without line numbers).
/// Used for backwards compatibility.
#[cfg(test)]
fn get_range_text_highlight(source: &SourceTextInfo, byte_range: Span) -> String {
    let (entries, _) = get_range_highlight_entries(source, byte_range);
    let mut result = String::new();
    for (i, entry) in entries.iter().enumerate() {
        if i > 0 {
            result.push('\n');
        }
        match entry {
            HighlightEntry::SourceLine { text, underline, .. } => {
                result.push_str(text);
                result.push('\n');
                result.push_str(underline);
            }
            HighlightEntry::Ellipsis => {
                result.push_str("...");
            }
        }
    }
    result
}

#[derive(Debug, JsError)]
#[class(syntax)]
pub struct ParseDiagnosticsError(pub Vec<ParseDiagnostic>);

impl std::error::Error for ParseDiagnosticsError {}

impl fmt::Display for ParseDiagnosticsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, diagnostic) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, "\n\n")?;
            }
            write!(f, "{diagnostic}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[expect(clippy::cast_possible_truncation, clippy::unwrap_used)]
mod test {
    use oxc::span::Span;

    use super::get_range_text_highlight;
    use crate::{ModuleSpecifier, ParseDiagnostic, SourceTextInfo};

    #[test]
    fn range_highlight_all_text() {
        let text = SourceTextInfo::from_string(
      concat!(
        "Line 0 - Testing this out with a long line testing0 testing1 testing2 testing3 testing4 testing5 testing6\n",
        "Line 1\n",
        "Line 2\n",
        "Line 3\n",
        "Line 4"
      ).to_string(),
    );
        assert_eq!(
            get_range_text_highlight(
                &text,
                Span::new(text.line_start(0) as u32, text.line_end(4) as u32)
            ),
            concat!(
                "Line 0 - Testing this out with a long line testing0 testing1 testing2 testing3 t...\n",
                "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~\n",
                "Line 1\n",
                "~~~~~~\n",
                "...\n",
                "Line 4\n",
                "~~~~~~",
            ),
        );
    }

    #[test]
    fn range_highlight_all_text_last_line_long() {
        let text = SourceTextInfo::from_string(
      concat!(
        "Line 0\n",
        "Line 1\n",
        "Line 2\n",
        "Line 3\n",
        "Line 4 - Testing this out with a long line testing0 testing1 testing2 testing3 testing4 testing5 testing6\n",
      ).to_string(),
    );
        assert_eq!(
            get_range_text_highlight(
                &text,
                Span::new(text.line_start(0) as u32, text.line_end(4) as u32)
            ),
            concat!(
                "Line 0\n",
                "~~~~~~\n",
                "Line 1\n",
                "~~~~~~\n",
                "...\n",
                "Line 4 - Testing this out with a long line testing0 testing1 testing2 testing3 t...\n",
                "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~",
            ),
        );
    }

    #[test]
    fn range_highlight_range_start_long_line() {
        let text = SourceTextInfo::from_string(
      "Testing this out with a long line testing0 testing1 testing2 testing3 testing4 testing5 testing6 testing7".to_string(),
    );
        assert_eq!(
            get_range_text_highlight(
                &text,
                Span::new(text.line_start(0) as u32, (text.line_start(0) + 1) as u32)
            ),
            concat!(
                "Testing this out with a long line testing0 testing1 testing2 testing3 testing4 t...\n",
                "~",
            ),
        );
    }

    #[test]
    fn range_highlight_range_end_long_line() {
        let text = SourceTextInfo::from_string(
      "Testing this out with a long line testing0 testing1 testing2 testing3 testing4 testing5 testing6 testing7".to_string(),
    );
        assert_eq!(
            get_range_text_highlight(
                &text,
                Span::new((text.line_end(0) - 1) as u32, text.line_end(0) as u32)
            ),
            concat!(
                "...ong line testing0 testing1 testing2 testing3 testing4 testing5 testing6 testing7\n",
                "                                                                                  ~",
            ),
        );
    }

    #[test]
    fn range_highlight_whitespace_start_line() {
        let text = SourceTextInfo::from_string("  testing\r\ntest".to_string());
        assert_eq!(
            get_range_text_highlight(
                &text,
                Span::new((text.line_end(0) - 1) as u32, text.line_end(1) as u32)
            ),
            concat!("  testing\n", "        ~\n", "test\n", "~~~~",),
        );
    }

    #[test]
    fn range_highlight_uses_ecmascript_line_boundaries() {
        let text = SourceTextInfo::from_string("a\r\nb\rc\u{2028}d\u{2029}e".to_string());
        for (line_index, expected) in ["a", "b", "c", "d", "e"].into_iter().enumerate() {
            assert_eq!(
                get_range_text_highlight(
                    &text,
                    Span::new(text.line_start(line_index) as u32, text.line_end(line_index) as u32,)
                ),
                format!("{expected}\n~")
            );
        }
    }

    #[test]
    fn range_highlight_retains_preceding_code_line_for_whitespace() {
        let text = SourceTextInfo::from_string("code\n  ".to_string());
        let highlight = get_range_text_highlight(&text, Span::new(5, 7));
        assert!(highlight.starts_with("code\n"));
        assert!(highlight.ends_with("  \n~~"));
    }

    #[test]
    fn range_end_of_line() {
        let text = SourceTextInfo::from_string("  testingtestingtestingtesting".to_string());
        assert_eq!(
            get_range_text_highlight(
                &text,
                Span::new(text.line_end(0) as u32, text.line_end(0) as u32)
            ),
            concat!("  testingtestingtestingtesting\n", "                              ~",),
        );
    }

    #[test]
    fn diagnostic_highlight_uses_original_line_numbers() {
        let source = SourceTextInfo::from_string("code\n  ".to_string());
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.ts").unwrap();
        let diagnostic = ParseDiagnostic::from_message(
            "unexpected end".to_string(),
            &specifier,
            source,
            Span::new(5, 7),
        );
        let formatted = diagnostic.to_string();
        assert!(formatted.contains("\n1 | code\n"));
        assert!(formatted.contains("\n2 |   \n"));
    }

    #[test]
    fn diagnostic_highlight_keeps_line_numbers_after_elision() {
        let text = "a\nb\nc\nd\ne\nf\ng\nh";
        let source = SourceTextInfo::from_string(text.to_string());
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.ts").unwrap();
        let diagnostic = ParseDiagnostic::from_message(
            "range".to_string(),
            &specifier,
            source,
            Span::new(0, text.len() as u32),
        );
        let formatted = diagnostic.to_string();
        assert!(formatted.contains("\n1 | a\n"));
        assert!(formatted.contains("\n2 | b\n"));
        assert!(formatted.contains("\n8 | h\n"));
    }
}
