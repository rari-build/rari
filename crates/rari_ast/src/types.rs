// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

#![expect(clippy::needless_pass_by_value)]

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

    /// Span of the diagnostic.
    pub fn span(&self) -> Span {
        self.0.span
    }

    /// The diagnostic message.
    pub fn message(&self) -> &str {
        &self.0.message
    }

    /// Structured diagnostic code, when one is available.
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

    /// Whether this diagnostic is considered fatal for transpilation.
    pub fn is_fatal(&self) -> bool {
        // All parse errors are considered fatal
        true
    }

    /// 1-indexed display position the diagnostic occurred at.
    pub fn display_position(&self) -> LineAndColumnDisplay {
        self.0.source.line_and_column_display(self.0.span.start as usize)
    }

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

/// The parser reports errors like an unexpected end of file with an empty span
/// after any trailing whitespace. Point those at the end of the preceding code
/// instead so the diagnostic shows that code rather than a blank line.
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
    // the span moved across a line, so point at the end of the code
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
    SourceLine { text: String, underline: String },
    /// An ellipsis indicating skipped lines.
    Ellipsis,
}

/// Formats highlight entries with line numbers and pipe margins.
///
/// Output format:
/// ```text
///   |
/// N | {source_line}
///   | {underline}
/// ```
fn format_highlight_entries(entries: &[HighlightEntry], start_line_number: usize) -> String {
    let num_str = start_line_number.to_string();
    let padding = " ".repeat(num_str.len());
    let mut result = format!("{padding} |\n");

    let mut current_line_num = start_line_number;
    let entry_count = entries.len();
    for (i, entry) in entries.iter().enumerate() {
        match entry {
            HighlightEntry::SourceLine { text, underline } => {
                let _ = writeln!(result, "{current_line_num} | {text}");
                let _ = write!(result, "{padding} | {underline}");
                current_line_num += 1;
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
            let entries = get_range_highlight_entries(self.source(), self.span());
            format_highlight_entries(&entries, display_position.line_number)
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
fn get_range_highlight_entries(source: &SourceTextInfo, byte_range: Span) -> Vec<HighlightEntry> {
    fn get_column_index_of_pos(text: &str, pos: usize) -> usize {
        let line_start_byte_pos = get_line_start_byte_pos(text, pos);
        text[line_start_byte_pos..pos].chars().count()
    }

    fn get_line_start_byte_pos(text: &str, pos: usize) -> usize {
        let text_bytes = text.as_bytes();
        for i in (0..pos).rev() {
            if text_bytes.get(i) == Some(&(b'\n')) {
                return i + 1;
            }
        }
        0
    }

    fn get_text_and_error_range(
        source: &SourceTextInfo,
        byte_range: Span,
    ) -> (&str, (usize, usize)) {
        let start = byte_range.start as usize;
        let end = byte_range.end as usize;
        let mut first_line_index = source.line_index(start);
        let mut first_line_start = source.line_start(first_line_index);
        let last_line_end = source.line_end(source.line_index(end));
        let mut sub_text = source.range_text(first_line_start, last_line_end);

        // while the text is empty, show the previous line
        while sub_text.trim().is_empty() && first_line_index > 0 {
            first_line_index -= 1;
            first_line_start = source.line_start(first_line_index);
            sub_text = source.range_text(first_line_start, last_line_end);
        }

        let error_start = start - first_line_start;
        let error_end = error_start + (end - start);
        (sub_text, (error_start, error_end))
    }

    let (sub_text, (error_start, error_end)) = get_text_and_error_range(source, byte_range);

    let mut entries = Vec::new();
    // don't use .lines() here because it will trim any empty
    // lines, which might for some reason be part of the range
    let lines = sub_text.split('\n').collect::<Vec<_>>();
    let line_count = lines.len();
    for (i, mut line) in lines.into_iter().enumerate() {
        if line.ends_with('\r') {
            line = &line[..line.len() - 1];
        }
        let is_last_line = i == line_count - 1;
        if i > 2 && !is_last_line {
            continue;
        }
        if i == 2 && !is_last_line {
            entries.push(HighlightEntry::Ellipsis);
            continue;
        }

        let mut error_start_char_index =
            if i == 0 { get_column_index_of_pos(sub_text, error_start) } else { 0 };
        let mut error_end_char_index = if is_last_line {
            get_column_index_of_pos(sub_text, error_end)
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

        let underline = format!(
            "{}{}",
            " ".repeat(error_start_char_index),
            "~".repeat(std::cmp::max(
                1, // this means it's the end of the line, so display a single ~
                error_end_char_index - error_start_char_index,
            ))
        );

        entries.push(HighlightEntry::SourceLine { text, underline });
    }
    entries
}

/// Formats a range highlight as a plain string (without line numbers).
/// Used for backwards compatibility.
#[cfg(test)]
fn get_range_text_highlight(source: &SourceTextInfo, byte_range: Span) -> String {
    let entries = get_range_highlight_entries(source, byte_range);
    let mut result = String::new();
    for (i, entry) in entries.iter().enumerate() {
        if i > 0 {
            result.push('\n');
        }
        match entry {
            HighlightEntry::SourceLine { text, underline } => {
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
#[expect(
    clippy::cast_possible_truncation,
    reason = "Ported code not yet updated to the workspace lint rules"
)]
mod test {
    use oxc::span::Span;

    use super::get_range_text_highlight;
    use crate::SourceTextInfo;

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
}
