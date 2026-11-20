// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

#![expect(
    clippy::too_many_lines,
    reason = "Ported code not yet updated to the workspace lint rules"
)]

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt,
    fmt::{Display, Write as _},
    path::PathBuf,
};

use deno_terminal::colors;
use unicode_width::UnicodeWidthStr;

use crate::{ModuleSpecifier, SourceTextInfo};

pub enum DiagnosticLevel {
    Error,
    Warning,
}

#[derive(Clone, Copy, Debug)]
pub struct DiagnosticSourceRange {
    pub start: DiagnosticSourcePos,
    pub end: DiagnosticSourcePos,
}

#[derive(Clone, Copy, Debug)]
pub enum DiagnosticSourcePos {
    ByteIndex(usize),
    LineAndCol {
        // 0-indexed line number in bytes
        line: usize,
        // 0-indexed column number in bytes
        column: usize,
    },
}

impl DiagnosticSourcePos {
    fn byte_index(&self, source: &SourceTextInfo) -> usize {
        match self {
            Self::ByteIndex(index) => *index,
            Self::LineAndCol { line, column } => source.line_start(*line) + *column,
        }
    }
}

#[derive(Clone, Debug)]
pub enum DiagnosticLocation<'a> {
    /// The diagnostic is relevant to a specific path.
    Path { path: PathBuf },
    /// The diagnostic is relevant to an entire module.
    Module {
        /// The specifier of the module that contains the diagnostic.
        specifier: Cow<'a, ModuleSpecifier>,
    },
    /// The diagnostic is relevant to a specific position in a module.
    ModulePosition {
        /// The specifier of the module that contains the diagnostic.
        specifier: Cow<'a, ModuleSpecifier>,
        /// The source position of the diagnostic.
        source_pos: DiagnosticSourcePos,
        text_info: Cow<'a, SourceTextInfo>,
    },
}

impl DiagnosticLocation<'_> {
    /// Return the line and column number of the diagnostic.
    ///
    /// The line number is 1-indexed.
    ///
    /// The column number is 1-indexed. This is the number of UTF-16 code units
    /// from the start of the line to the diagnostic.
    fn position(&self) -> Option<(usize, usize)> {
        match self {
            DiagnosticLocation::Path { .. } | DiagnosticLocation::Module { .. } => None,
            DiagnosticLocation::ModulePosition { specifier: _, source_pos, text_info } => {
                let pos = source_pos.byte_index(text_info);
                let line_index = text_info.line_index(pos);
                let line_start = text_info.line_start(line_index);
                let content =
                    text_info.range_text(line_start, pos.min(text_info.line_end(line_index)));
                let line = line_index + 1;
                let column = content.encode_utf16().count() + 1;
                Some((line, column))
            }
        }
    }
}

pub struct DiagnosticSnippet<'a> {
    /// The source text for this snippet.
    pub source: Cow<'a, crate::SourceTextInfo>,
    /// The piece of the snippet that should be highlighted. For best results, the
    /// highlights should not overlap and be ordered by their start position.
    pub highlights: Vec<DiagnosticSnippetHighlight<'a>>,
}

#[derive(Clone)]
pub struct DiagnosticSnippetHighlight<'a> {
    /// The range of the snippet that should be highlighted.
    pub range: DiagnosticSourceRange,
    /// The style of the highlight.
    pub style: DiagnosticSnippetHighlightStyle,
    /// An optional inline description of the highlight.
    pub description: Option<Cow<'a, str>>,
}

#[derive(Clone, Copy)]
pub enum DiagnosticSnippetHighlightStyle {
    /// The highlight is an error. This will place red carets under the highlight.
    Error,
    /// The highlight is a warning. This will place yellow carets under the
    /// highlight.
    Warning,
    /// The highlight shows a hint. This will place blue dashes under the
    /// highlight.
    Hint,
}

impl DiagnosticSnippetHighlightStyle {
    fn style_underline(&self, s: impl std::fmt::Display) -> impl std::fmt::Display {
        match self {
            Self::Error => colors::red_bold(s),
            Self::Warning => colors::yellow_bold(s),
            Self::Hint => colors::intense_blue(s),
        }
    }

    fn underline_char(self) -> char {
        match self {
            Self::Error | Self::Warning => '^',
            Self::Hint => '-',
        }
    }
}

/// Returns the text of the line with the given number (1-indexed).
fn line_text(source: &SourceTextInfo, line_number: usize) -> &str {
    source.line_text(line_number - 1)
}

/// Returns the line number (1-indexed) of the line that contains the given
/// position.
fn line_number(source: &SourceTextInfo, pos: DiagnosticSourcePos) -> usize {
    source.line_index(pos.byte_index(source)) + 1
}

pub trait Diagnostic {
    /// The level of the diagnostic.
    fn level(&self) -> DiagnosticLevel;

    /// The diagnostic code, like `no-explicit-any` or `ban-untagged-ignore`.
    fn code(&self) -> Cow<'_, str>;

    /// The human-readable diagnostic message.
    fn message(&self) -> Cow<'_, str>;

    /// The location this diagnostic is associated with.
    fn location(&self) -> DiagnosticLocation<'_>;

    /// A snippet showing the source code associated with the diagnostic.
    fn snippet(&self) -> Option<DiagnosticSnippet<'_>>;

    /// A hint for fixing the diagnostic.
    fn hint(&self) -> Option<Cow<'_, str>>;

    /// A snippet showing how the diagnostic can be fixed.
    fn snippet_fixed(&self) -> Option<DiagnosticSnippet<'_>>;

    fn info(&self) -> Cow<'_, [Cow<'_, str>]>;

    /// An optional URL to the documentation for the diagnostic.
    fn docs_url(&self) -> Option<Cow<'_, str>>;

    fn display(&self) -> DiagnosticDisplay<'_, Self> {
        DiagnosticDisplay { diagnostic: self }
    }
}

struct RepeatingCharFmt(char, usize);
impl fmt::Display for RepeatingCharFmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for _ in 0..self.1 {
            f.write_char(self.0)?;
        }
        Ok(())
    }
}

/// How many spaces a tab should be displayed as. 2 is the default used for
/// `deno fmt`, so we'll use that here.
const TAB_WIDTH: usize = 2;

struct ReplaceTab<'a>(&'a str);
impl fmt::Display for ReplaceTab<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut written = 0;
        for (i, c) in self.0.char_indices() {
            if c == '\t' {
                self.0[written..i].fmt(f)?;
                RepeatingCharFmt(' ', TAB_WIDTH).fmt(f)?;
                written = i + 1;
            }
        }
        self.0[written..].fmt(f)?;
        Ok(())
    }
}

/// The width of the string as displayed, assuming tabs are 2 spaces wide.
fn display_width(str: &str) -> usize {
    let num_tabs = str.chars().filter(|c| *c == '\t').count();
    str.width_cjk() + num_tabs * TAB_WIDTH - num_tabs
}

pub struct DiagnosticDisplay<'a, T: Diagnostic + ?Sized> {
    diagnostic: &'a T,
}

impl<T: Diagnostic + ?Sized> Display for DiagnosticDisplay<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        print_diagnostic(f, self.diagnostic)
    }
}

fn print_diagnostic(
    io: &mut dyn std::fmt::Write,
    diagnostic: &(impl Diagnostic + ?Sized),
) -> Result<(), std::fmt::Error> {
    match diagnostic.level() {
        DiagnosticLevel::Error => {
            write!(io, "{}", colors::red_bold(format_args!("error[{}]", diagnostic.code())))?;
        }
        DiagnosticLevel::Warning => {
            write!(io, "{}", colors::yellow_bold(format_args!("warning[{}]", diagnostic.code())))?;
        }
    }

    writeln!(io, ": {}", colors::bold(diagnostic.message()))?;

    let mut max_line_number_digits = 1;
    // Whether any highlight spans multiple lines. When it does, a `>` marker is
    // drawn in the gutter next to each line number that is part of a multi-line
    // span, so the extent of the span is visible even though interior lines are
    // not underlined.
    let mut has_multiline_highlight = false;
    let mut note_highlight = |source: &SourceTextInfo, highlight: &DiagnosticSnippetHighlight| {
        let first_line = line_number(source, highlight.range.start);
        let last_line = line_number(source, highlight.range.end);
        max_line_number_digits = max_line_number_digits.max(last_line.ilog10() + 1);
        if first_line != last_line {
            has_multiline_highlight = true;
        }
    };
    if let Some(snippet) = diagnostic.snippet() {
        for highlight in &snippet.highlights {
            note_highlight(&snippet.source, highlight);
        }
    }
    if let Some(snippet) = diagnostic.snippet_fixed() {
        for highlight in &snippet.highlights {
            note_highlight(&snippet.source, highlight);
        }
    }

    // Width of the gutter marker column (`> ` / blank) reserved to the left of
    // line numbers. Zero when there are no multi-line highlights, keeping
    // single-line diagnostics unchanged.
    let marker_width: usize = if has_multiline_highlight { 2 } else { 0 };

    let location = diagnostic.location();
    write!(
        io,
        "{}{}",
        RepeatingCharFmt(' ', max_line_number_digits as usize + marker_width),
        colors::intense_blue("-->"),
    )?;
    match &location {
        DiagnosticLocation::Path { path } => {
            write!(io, " {}", colors::cyan(path.display()))?;
        }
        DiagnosticLocation::Module { specifier }
        | DiagnosticLocation::ModulePosition { specifier, .. } => {
            if let Some(path) = specifier_to_file_path(specifier) {
                write!(io, " {}", colors::cyan(path.display()))?;
            } else {
                write!(io, " {}", colors::cyan(specifier.as_str()))?;
            }
        }
    }
    if let Some((line, column)) = location.position() {
        write!(io, "{}", colors::yellow(format_args!(":{line}:{column}")))?;
    }

    if diagnostic.snippet().is_some()
        || diagnostic.hint().is_some()
        || diagnostic.snippet_fixed().is_some()
        || !diagnostic.info().is_empty()
        || diagnostic.docs_url().is_some()
    {
        writeln!(io)?;
    }

    if let Some(snippet) = diagnostic.snippet() {
        print_snippet(io, &snippet, max_line_number_digits, marker_width)?;
    }

    if let Some(hint) = diagnostic.hint() {
        write!(
            io,
            "{} {} ",
            RepeatingCharFmt(' ', max_line_number_digits as usize + marker_width),
            colors::intense_blue("=")
        )?;
        writeln!(io, "{}: {}", colors::bold("hint"), hint)?;
    }

    if let Some(snippet) = diagnostic.snippet_fixed() {
        print_snippet(io, &snippet, max_line_number_digits, marker_width)?;
    }

    if !diagnostic.info().is_empty() || diagnostic.docs_url().is_some() {
        writeln!(io)?;
    }

    for info in diagnostic.info().iter() {
        writeln!(io, "  {}: {}", colors::intense_blue("info"), info)?;
    }
    if let Some(docs_url) = diagnostic.docs_url() {
        writeln!(io, "  {}: {}", colors::intense_blue("docs"), docs_url)?;
    }

    Ok(())
}

/// The number of leading lines shown for a collapsed multi-line highlight.
const HIGHLIGHT_HEAD_LINES: usize = 2;

/// The number of trailing lines shown for a collapsed multi-line highlight.
const HIGHLIGHT_TAIL_LINES: usize = 2;

/// The maximum number of source lines shown for a single highlight before its
/// middle is collapsed. A highlight spanning more lines than this shows its
/// first `HIGHLIGHT_HEAD_LINES` lines, a `...` elision marker, and its last
/// `HIGHLIGHT_TAIL_LINES` lines. This is `HEAD + TAIL + 1` so that collapsing
/// only happens when it actually elides more than a single line.
const MAX_HIGHLIGHT_LINES: usize = HIGHLIGHT_HEAD_LINES + HIGHLIGHT_TAIL_LINES + 1;

/// Prints a snippet to the given writer and returns the line number indent.
fn print_snippet(
    io: &mut dyn std::fmt::Write,
    snippet: &DiagnosticSnippet<'_>,
    max_line_number_digits: u32,
    marker_width: usize,
) -> Result<(), std::fmt::Error> {
    fn print_padded(
        io: &mut dyn std::fmt::Write,
        text: impl std::fmt::Display,
        padding: u32,
    ) -> Result<(), std::fmt::Error> {
        for _ in 0..padding {
            write!(io, " ")?;
        }
        write!(io, "{text}")?;
        Ok(())
    }

    let DiagnosticSnippet { source, highlights } = snippet;
    // The gutter indent for non-source rows (the `... `, ` | ` and underline
    // rows): the line-number field plus the marker column.
    let bar_indent = max_line_number_digits + marker_width as u32;

    // Determine which source lines to display, and where to insert elision
    // markers ("...") for long multi-line highlights. Rather than rendering an
    // underline under every line of a large span (which can be dozens of lines),
    // we show the first few lines of the span, elide the middle with a "...",
    // and show the final line. Carets are only drawn on the first and last line
    // of a multi-line highlight.
    let mut lines_to_show = HashMap::<usize, Vec<usize>>::new();
    // The set of line numbers after which the middle of a span was elided; used
    // to print a "..." marker before the following (final) line of the span.
    let mut elided_after = HashSet::<usize>::new();
    let mut highlights_info = Vec::new();
    for (i, highlight) in highlights.iter().enumerate() {
        let start_line_number = line_number(source, highlight.range.start);
        let end_line_number = line_number(source, highlight.range.end);
        highlights_info.push((start_line_number, end_line_number));

        let line_count = end_line_number - start_line_number + 1;
        if line_count <= MAX_HIGHLIGHT_LINES {
            for line_number in start_line_number..=end_line_number {
                lines_to_show.entry(line_number).or_default().push(i);
            }
        } else {
            // Show the first `HIGHLIGHT_HEAD_LINES` and last `HIGHLIGHT_TAIL_LINES`
            // lines of the span, eliding the middle with a `...` marker.
            let head_end = start_line_number + HIGHLIGHT_HEAD_LINES - 1;
            let tail_start = end_line_number - (HIGHLIGHT_TAIL_LINES - 1);
            for line_number in start_line_number..=head_end {
                lines_to_show.entry(line_number).or_default().push(i);
            }
            elided_after.insert(head_end);
            for line_number in tail_start..=end_line_number {
                lines_to_show.entry(line_number).or_default().push(i);
            }
        }
    }

    let mut lines_to_show = lines_to_show.into_iter().collect::<Vec<_>>();
    lines_to_show.sort();

    print_padded(io, colors::intense_blue(" | "), bar_indent)?;
    writeln!(io)?;
    let mut previous_line_number: Option<usize> = None;
    for (line_number, highlight_indexes) in lines_to_show {
        if previous_line_number.is_some_and(|previous| elided_after.contains(&previous)) {
            print_padded(io, colors::intense_blue("..."), bar_indent)?;
            writeln!(io)?;
        }

        // Draw the gutter marker column: `> ` next to line numbers that are part
        // of a multi-line span, blank otherwise.
        if marker_width > 0 {
            let is_span_line = highlight_indexes.iter().any(|&i| {
                let (start_line_number, end_line_number) = highlights_info[i];
                start_line_number != end_line_number
            });
            if is_span_line {
                write!(io, "{}", colors::intense_blue(">"))?;
                print_padded(io, "", (marker_width - 1) as u32)?;
            } else {
                print_padded(io, "", marker_width as u32)?;
            }
        }

        print_padded(
            io,
            colors::intense_blue(format_args!("{line_number} | ")),
            max_line_number_digits - line_number.ilog10() - 1,
        )?;

        let line_start_pos = source.line_start(line_number - 1);
        let line_end_pos = source.line_end(line_number - 1);
        let line_text = line_text(source, line_number);
        writeln!(io, "{}", ReplaceTab(line_text))?;

        let mut wrote_description = false;
        for highlight_index in highlight_indexes {
            let highlight = &highlights[highlight_index];
            let (start_line_number, end_line_number) = highlights_info[highlight_index];

            // Only the first and last line of a highlight are underlined. Interior
            // lines are shown for context but left un-underlined to avoid a wall of
            // carets spanning many lines.
            let is_first_line = start_line_number == line_number;
            let is_last_line = end_line_number == line_number;
            if !is_first_line && !is_last_line {
                continue;
            }

            let (padding_width, highlight_width) = if start_line_number == end_line_number {
                let highlight_start =
                    highlight.range.start.byte_index(source).clamp(line_start_pos, line_end_pos);
                let highlight_end =
                    highlight.range.end.byte_index(source).clamp(highlight_start, line_end_pos);
                (
                    display_width(source.range_text(line_start_pos, highlight_start)),
                    display_width(source.range_text(highlight_start, highlight_end)).max(1),
                )
            } else if is_first_line {
                let highlight_start =
                    highlight.range.start.byte_index(source).clamp(line_start_pos, line_end_pos);
                (
                    display_width(source.range_text(line_start_pos, highlight_start)),
                    display_width(source.range_text(highlight_start, line_end_pos)),
                )
            } else {
                // Last line of a multi-line highlight: underline only the offending
                // text, skipping the line's leading indentation so the carets sit
                // under the code rather than the whitespace.
                let highlight_end =
                    highlight.range.end.byte_index(source).clamp(line_start_pos, line_end_pos);
                let line_prefix = source.range_text(line_start_pos, highlight_end);
                let trimmed = line_prefix.trim_start();
                let indent = &line_prefix[..line_prefix.len() - trimmed.len()];
                (display_width(indent), display_width(trimmed))
            };

            let underline = RepeatingCharFmt(highlight.style.underline_char(), highlight_width);
            print_padded(io, colors::intense_blue(" | "), bar_indent)?;
            write!(io, "{}", RepeatingCharFmt(' ', padding_width))?;
            write!(io, "{}", highlight.style.style_underline(underline))?;

            if is_last_line && let Some(description) = &highlight.description {
                write!(io, " {}", highlight.style.style_underline(description))?;
                wrote_description = true;
            }

            writeln!(io)?;
        }

        if wrote_description {
            print_padded(io, colors::intense_blue(" | "), bar_indent)?;
            writeln!(io)?;
        }

        previous_line_number = Some(line_number);
    }

    Ok(())
}

/// Attempts to convert a specifier to a file path. By default, uses the Url
/// crate's `to_file_path()` method, but falls back to try and resolve unix-style
/// paths on Windows.
fn specifier_to_file_path(specifier: &ModuleSpecifier) -> Option<PathBuf> {
    fn to_file_path_if_not_wasm(specifier: &ModuleSpecifier) -> Option<PathBuf> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = specifier;
            None
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            // not available in Wasm
            specifier.to_file_path().ok()
        }
    }

    if specifier.scheme() != "file" {
        None
    } else if cfg!(windows) {
        match to_file_path_if_not_wasm(specifier) {
            Some(path) => Some(path),
            None => {
                // This might be a unix-style path which is used in the tests even on Windows.
                if specifier.scheme() == "file"
                    && specifier.host().is_none()
                    && specifier.port().is_none()
                    && specifier.path_segments().is_some()
                {
                    let path_str = specifier.path();
                    match String::from_utf8(
                        percent_encoding::percent_decode(path_str.as_bytes()).collect(),
                    ) {
                        Ok(path_str) => Some(PathBuf::from(path_str)),
                        Err(_) => None,
                    }
                } else {
                    None
                }
            }
        }
    } else {
        to_file_path_if_not_wasm(specifier)
    }
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "Ported code not yet updated to the workspace lint rules")]
mod tests {
    use std::borrow::Cow;

    use super::*;
    use crate::{ModuleSpecifier, SourceTextInfo};

    struct TestDiagnostic {
        text_info: SourceTextInfo,
        range: DiagnosticSourceRange,
        description: Option<&'static str>,
    }

    impl Diagnostic for TestDiagnostic {
        fn level(&self) -> DiagnosticLevel {
            DiagnosticLevel::Error
        }
        fn code(&self) -> Cow<'_, str> {
            Cow::Borrowed("test-rule")
        }
        fn message(&self) -> Cow<'_, str> {
            Cow::Borrowed("a test diagnostic")
        }
        fn location(&self) -> DiagnosticLocation<'_> {
            DiagnosticLocation::ModulePosition {
                specifier: Cow::Owned("file:///test.ts".parse().unwrap()),
                source_pos: DiagnosticSourcePos::ByteIndex(
                    self.range.start.byte_index(&self.text_info),
                ),
                text_info: Cow::Borrowed(&self.text_info),
            }
        }
        fn snippet(&self) -> Option<DiagnosticSnippet<'_>> {
            Some(DiagnosticSnippet {
                source: Cow::Borrowed(&self.text_info),
                highlights: vec![DiagnosticSnippetHighlight {
                    range: self.range,
                    style: DiagnosticSnippetHighlightStyle::Error,
                    description: self.description.map(Cow::Borrowed),
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

    /// Removes ANSI escape sequences so rendered snippets can be compared as
    /// plain text regardless of whether colors are enabled.
    fn strip_ansi(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '\u{1b}' {
                for c in chars.by_ref() {
                    if c == 'm' {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    fn render(text: &str, range: DiagnosticSourceRange) -> String {
        render_with_description(text, range, None)
    }

    fn render_with_description(
        text: &str,
        range: DiagnosticSourceRange,
        description: Option<&'static str>,
    ) -> String {
        let diagnostic =
            TestDiagnostic { text_info: SourceTextInfo::new(text.into()), range, description };
        strip_ansi(&diagnostic.display().to_string())
    }

    fn line_col(line: usize, column: usize) -> DiagnosticSourcePos {
        DiagnosticSourcePos::LineAndCol { line, column }
    }

    #[test]
    fn test_snippet_single_line() {
        // A single-line highlight is rendered without a gutter marker column and
        // with carets directly under the highlighted range.
        let output = render(
            "const value = 1;\n",
            DiagnosticSourceRange { start: line_col(0, 6), end: line_col(0, 11) },
        );
        assert_eq!(
            output,
            "error[test-rule]: a test diagnostic\n \
       --> /test.ts:1:7\n  \
       | \n\
       1 | const value = 1;\n  \
       |       ^^^^^\n"
        );
    }

    #[test]
    fn test_snippet_crlf_range_excludes_carriage_return() {
        let output =
            render("foo\r\n", DiagnosticSourceRange { start: line_col(0, 0), end: line_col(0, 4) });
        assert_eq!(
            output,
            "error[test-rule]: a test diagnostic\n \
       --> /test.ts:1:1\n  \
       | \n\
       1 | foo\n  \
       | ^^^\n"
        );
    }

    #[test]
    fn test_snippet_range_starting_in_crlf_is_clamped_to_line_bounds() {
        let output = render(
            "a\r\nb",
            DiagnosticSourceRange {
                start: DiagnosticSourcePos::ByteIndex(2),
                end: DiagnosticSourcePos::ByteIndex(3),
            },
        );
        assert!(output.contains("1 | a\n"));
        assert!(output.contains("2 | b\n"));
    }

    #[test]
    fn test_snippet_zero_width_ranges_render_one_caret() {
        for (text, range, source_line) in [
            (
                "a\r\nb",
                DiagnosticSourceRange {
                    start: DiagnosticSourcePos::ByteIndex(1),
                    end: DiagnosticSourcePos::ByteIndex(2),
                },
                "| a",
            ),
            (
                "abc",
                DiagnosticSourceRange {
                    start: DiagnosticSourcePos::ByteIndex(3),
                    end: DiagnosticSourcePos::ByteIndex(3),
                },
                "| abc",
            ),
        ] {
            let output = render(text, range);
            let source_line_index =
                output.lines().position(|line| line.contains(source_line)).unwrap();
            let underline = output.lines().nth(source_line_index + 1).unwrap();
            assert_eq!(underline.matches('^').count(), 1, "{output}");
        }
    }

    #[test]
    fn test_snippet_multi_line_elided() {
        // A highlight spanning more than `MAX_HIGHLIGHT_LINES` lines shows a `>`
        // gutter marker on each span line, underlines only the first and last line
        // (trimming leading indentation on the last), and elides the middle with a
        // `...` marker, keeping the first two and last two lines.
        let text = "const p = new Promise(async (resolve) => {\n  \
                const a = 1;\n  \
                const b = 2;\n  \
                const c = 3;\n  \
                const d = 4;\n  \
                const e = 5;\n  \
                resolve(a);\n\
                });\n";
        let output =
            render(text, DiagnosticSourceRange { start: line_col(0, 22), end: line_col(7, 2) });
        assert_eq!(
            output,
            "error[test-rule]: a test diagnostic\n   --> /test.ts:1:23\n    | \n> 1 | const p = new Promise(async (resolve) => {\n    |                       ^^^^^^^^^^^^^^^^^^^^\n> 2 |   const a = 1;\n   ...\n> 7 |   resolve(a);\n> 8 | });\n    | ^^\n"
        );
    }

    #[test]
    fn test_snippet_multi_line_short_not_elided() {
        // A highlight spanning at most `MAX_HIGHLIGHT_LINES` lines shows every line
        // (with the gutter marker) but is not elided.
        let text = "foo(\n  1,\n  2,\n)\n";
        let output =
            render(text, DiagnosticSourceRange { start: line_col(0, 0), end: line_col(3, 1) });
        assert_eq!(
            output,
            "error[test-rule]: a test diagnostic\n   --> /test.ts:1:1\n    | \n> 1 | foo(\n    | ^^^^\n> 2 |   1,\n> 3 |   2,\n> 4 | )\n    | ^\n"
        );
    }

    #[test]
    fn test_snippet_multi_line_description_on_last_line() {
        // The highlight description is attached to the last line of the span.
        let text = "foo(\n  1,\n)\n";
        let output = render_with_description(
            text,
            DiagnosticSourceRange { start: line_col(0, 0), end: line_col(2, 1) },
            Some("this call"),
        );
        assert_eq!(
            output,
            "error[test-rule]: a test diagnostic\n   --> /test.ts:1:1\n    | \n> 1 | foo(\n    | ^^^^\n> 2 |   1,\n> 3 | )\n    | ^ this call\n    | \n"
        );
    }

    #[test]
    fn test_display_width() {
        assert_eq!(display_width("abc"), 3);
        assert_eq!(display_width("\t"), 2);
        assert_eq!(display_width("\t\t123"), 7);
        assert_eq!(display_width("🎄"), 2);
        assert_eq!(display_width("🎄🎄"), 4);
        assert_eq!(display_width("🧑‍🦰"), 2);
    }

    #[test]
    fn test_position_in_file_from_text_info_simple() {
        let specifier: ModuleSpecifier = "file:///dev/test.ts".parse().unwrap();
        let text_info = SourceTextInfo::new("foo\nbar\nbaz".into());
        let pos = text_info.line_start(1);
        let location = DiagnosticLocation::ModulePosition {
            specifier: Cow::Borrowed(&specifier),
            source_pos: DiagnosticSourcePos::ByteIndex(pos),
            text_info: Cow::Owned(text_info),
        };
        let position = location.position().unwrap();
        assert_eq!(position, (2, 1));
    }

    #[test]
    fn test_position_in_file_from_text_info_emoji() {
        let specifier: ModuleSpecifier = "file:///dev/test.ts".parse().unwrap();
        let text_info = SourceTextInfo::new("🧑‍🦰text".into());
        let pos = text_info.line_start(0) + 11; // the end of the emoji
        let location = DiagnosticLocation::ModulePosition {
            specifier: Cow::Borrowed(&specifier),
            source_pos: DiagnosticSourcePos::ByteIndex(pos),
            text_info: Cow::Owned(text_info),
        };
        let position = location.position().unwrap();
        assert_eq!(position, (1, 6));
    }

    #[test]
    fn test_specifier_to_file_path() {
        fn run_success_test(specifier: &str, expected_path: &str) {
            let result =
                specifier_to_file_path(&ModuleSpecifier::parse(specifier).unwrap()).unwrap();
            assert_eq!(result, PathBuf::from(expected_path));
        }

        run_success_test("file:///", "/");
        run_success_test("file:///test", "/test");
        run_success_test("file:///dir/test/test.txt", "/dir/test/test.txt");
        run_success_test("file:///dir/test%20test/test.txt", "/dir/test test/test.txt");
    }
}
