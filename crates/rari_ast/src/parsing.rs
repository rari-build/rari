// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use std::sync::{Arc, OnceLock};

use oxc::{
    allocator::Allocator,
    parser::{ParseOptions, Parser},
    span::SourceType,
};

use crate::{
    MediaType, ModuleSpecifier, ParseDiagnostic, ParseDiagnostics, ParsedSource, SourceTextInfo,
};

/// Parameters for parsing.
pub struct ParseParams {
    /// Specifier of the source text.
    pub specifier: ModuleSpecifier,
    /// Source text.
    pub text: Arc<str>,
    /// Media type of the source text.
    pub media_type: MediaType,
    /// Whether to capture tokens or not.
    pub capture_tokens: bool,
    pub scope_analysis: bool,
    pub maybe_source_type: Option<SourceType>,
}

/// Parses the provided information attempting to figure out if the provided
/// text is for a script or a module.
pub fn parse_program(
    allocator: &Allocator,
    params: ParseParams,
) -> Result<ParsedSource<'_>, ParseDiagnostic> {
    parse(allocator, params, None)
}

/// Parses the provided information to a module.
pub fn parse_module(
    allocator: &Allocator,
    params: ParseParams,
) -> Result<ParsedSource<'_>, ParseDiagnostic> {
    parse(allocator, params, Some(true))
}

/// Parses the provided information to a script.
pub fn parse_script(
    allocator: &Allocator,
    params: ParseParams,
) -> Result<ParsedSource<'_>, ParseDiagnostic> {
    parse(allocator, params, Some(false))
}

fn parse(
    allocator: &Allocator,
    params: ParseParams,
    force_module: Option<bool>,
) -> Result<ParsedSource<'_>, ParseDiagnostic> {
    let source = strip_bom_from_arc(params.text, true);
    let specifier = params.specifier;
    let media_type = params.media_type;

    let source_type = params.maybe_source_type.unwrap_or_else(|| get_source_type(media_type));

    let source_type = match force_module {
        Some(true) => source_type.with_module(true),
        Some(false) => source_type.with_script(true),
        None => {
            match media_type {
                MediaType::Cjs => source_type.with_script(true),
                MediaType::Cts => {
                    // cts files can contain module declarations like
                    source_type.with_module(true)
                }
                MediaType::Mjs | MediaType::Mts => source_type.with_module(true),
                _ => source_type,
            }
        }
    };

    let parse_options = ParseOptions { parse_regular_expression: false, ..Default::default() };

    let source_in_arena = allocator.alloc_str(source.as_ref());

    let parser = Parser::new(allocator, source_in_arena, source_type).with_options(parse_options);
    let ret = if params.capture_tokens {
        parser.with_config(oxc::parser::config::TokensParserConfig).parse()
    } else {
        parser.parse()
    };

    if ret.fatal_error {
        let source_text_info = SourceTextInfo::new(Arc::clone(&source));
        if let Some(error) = ret.diagnostics.into_iter().next() {
            return Err(ParseDiagnostic::from_oxc_diagnostic(error, &specifier, source_text_info));
        }
        return Err(ParseDiagnostic::from_message(
            "Parser panicked".to_string(),
            &specifier,
            SourceTextInfo::new(Arc::clone(&source)),
            oxc::span::Span::new(0, 0),
        ));
    }

    if !ret.diagnostics.is_empty()
        && let Some(unparsed_start) = unparsed_source_start(&ret.program, source_in_arena)
    {
        let source_text_info = SourceTextInfo::new(Arc::clone(&source));
        let mut diagnostics = ret.diagnostics.into_iter().collect::<Vec<_>>();
        let index = diagnostics
            .iter()
            .position(|d| d.labels.iter().any(|label| label.offset() >= unparsed_start))
            .unwrap_or(0);
        return Err(ParseDiagnostic::from_oxc_diagnostic(
            diagnostics.swap_remove(index),
            &specifier,
            source_text_info,
        ));
    }

    if source_type.is_script() && ret.module_record.has_module_syntax {
        let span =
            first_module_decl_span(&ret.program).unwrap_or_else(|| oxc::span::Span::new(0, 0));
        return Err(ParseDiagnostic::from_message(
            "'import', and 'export' cannot be used outside of module code".to_string(),
            &specifier,
            SourceTextInfo::new(Arc::clone(&source)),
            span,
        ));
    }

    let diagnostics = if ret.diagnostics.is_empty() {
        ParseDiagnostics::default()
    } else {
        let source_text_info = SourceTextInfo::new(Arc::clone(&source));
        let diags = ret
            .diagnostics
            .into_iter()
            .map(|err| {
                ParseDiagnostic::from_oxc_diagnostic(err, &specifier, source_text_info.clone())
            })
            .collect();
        ParseDiagnostics { diagnostics: diags, script_module_diagnostics: Vec::new() }
    };

    let scoping = if params.scope_analysis {
        let semantic_ret = oxc::semantic::SemanticBuilder::new().build(&ret.program);
        Some(semantic_ret.semantic.into_scoping())
    } else {
        None
    };

    Ok(ParsedSource {
        specifier,
        media_type,
        text: source,
        source_text_info: OnceLock::default(),
        program: ret.program,
        module_record: ret.module_record,
        tokens: ret.tokens,
        diagnostics,
        scoping,
    })
}

fn unparsed_source_start(program: &oxc::ast::ast::Program, source: &str) -> Option<u32> {
    use oxc::span::GetSpan;
    let parsed_end = program
        .body
        .last()
        .map(|stmt| stmt.span().end)
        .into_iter()
        .chain(program.directives.last().map(|d| d.span.end))
        .chain(program.hashbang.as_ref().map(|h| h.span.end))
        .chain(program.comments.last().map(|c| c.span.end))
        .max()
        .unwrap_or(0);
    let rest = source.get(parsed_end as usize..)?;
    rest.char_indices()
        .find(|(_, c)| !c.is_whitespace() && *c != '\u{FEFF}')
        .map(|(i, _)| parsed_end + i as u32)
}

fn first_module_decl_span(program: &oxc::ast::ast::Program) -> Option<oxc::span::Span> {
    use oxc::span::GetSpan;
    program.body.iter().find_map(|stmt| stmt.as_module_declaration().map(GetSpan::span))
}

pub fn get_source_type(media_type: MediaType) -> SourceType {
    match media_type {
        MediaType::TypeScript | MediaType::Mts | MediaType::Cts => SourceType::ts(),
        MediaType::Tsx => SourceType::tsx(),
        MediaType::Dts | MediaType::Dmts | MediaType::Dcts => {
            SourceType::ts().with_typescript_definition(true)
        }
        MediaType::Jsx => SourceType::jsx(),
        MediaType::JavaScript => SourceType::unambiguous(),
        MediaType::Cjs => SourceType::cjs(),
        MediaType::Mjs
        | MediaType::Json
        | MediaType::Jsonc
        | MediaType::Json5
        | MediaType::Markdown
        | MediaType::Wasm
        | MediaType::SourceMap
        | MediaType::Css
        | MediaType::Sql
        | MediaType::Html
        | MediaType::Unknown => SourceType::mjs(),
    }
}

pub fn strip_bom(mut s: String) -> String {
    if s.starts_with('\u{FEFF}') {
        s.drain(..3);
    }
    s
}

fn strip_bom_from_arc(s: Arc<str>, should_panic_in_debug: bool) -> Arc<str> {
    if let Some(stripped_text) = s.strip_prefix('\u{FEFF}') {
        // this is only a perf concern, so don't crash in release
        assert!(
            !(cfg!(debug_assertions) && should_panic_in_debug),
            "BOM should be stripped from text before providing it to rari_ast to avoid a file text allocation"
        );
        stripped_text.into()
    } else {
        s
    }
}

#[cfg(test)]
#[expect(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
mod test {
    use super::*;

    #[test]
    fn should_parse_program() {
        let allocator = Allocator::default();
        let program = parse_program(
            &allocator,
            ParseParams {
                specifier: ModuleSpecifier::parse("file:///my_file.js").unwrap(),
                text: "// 1\n1 + 1\n// 2".into(),
                media_type: MediaType::JavaScript,
                capture_tokens: true,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        assert_eq!(program.specifier().as_str(), "file:///my_file.js");
        assert_eq!(program.text().as_ref(), "// 1\n1 + 1\n// 2");
        assert_eq!(program.media_type(), MediaType::JavaScript);
        assert_eq!(program.body().len(), 1);
        assert_eq!(program.get_leading_comments().count(), 1);
        assert_eq!(program.comments().len(), 2);
    }

    #[test]
    fn should_error_when_lexer_stops_early() {
        for (text, expected_line) in [
            ("# Some Markdown\n\n**bold**", 1),
            ("const a = 1;\n# not js\nconst b = 2;", 2),
            ("const a = 1;\n@", 2),
        ] {
            let allocator = Allocator::default();
            let err = parse_module(
                &allocator,
                ParseParams {
                    specifier: ModuleSpecifier::parse("file:///my_file.js").unwrap(),
                    text: text.into(),
                    media_type: MediaType::JavaScript,
                    capture_tokens: false,
                    maybe_source_type: None,
                    scope_analysis: false,
                },
            )
            .err()
            .unwrap_or_else(|| panic!("expected a parse error for {text:?}"));
            assert_eq!(err.display_position().line_number, expected_line, "{text:?}");
        }

        let allocator = Allocator::default();
        let program = parse_module(
            &allocator,
            ParseParams {
                specifier: ModuleSpecifier::parse("file:///my_file.js").unwrap(),
                text: "const a;\n// trailing\n\n".into(),
                media_type: MediaType::JavaScript,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        assert_eq!(program.diagnostics().len(), 1);
    }

    #[test]
    fn should_parse_module() {
        let allocator = Allocator::default();
        let program = parse_module(
            &allocator,
            ParseParams {
                specifier: ModuleSpecifier::parse("file:///my_file.js").unwrap(),
                text: "// 1\n1 + 1\n// 2".into(),
                media_type: MediaType::JavaScript,
                capture_tokens: true,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        assert!(program.is_module());
    }

    #[test]
    fn should_handle_parse_error() {
        let allocator = Allocator::default();
        let diagnostic = parse_module(
            &allocator,
            ParseParams {
                specifier: ModuleSpecifier::parse("file:///my_file.js").unwrap(),
                text: "t u".into(),
                media_type: MediaType::JavaScript,
                capture_tokens: true,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .err()
        .unwrap();
        assert_eq!(diagnostic.specifier().as_str(), "file:///my_file.js");
    }

    #[test]
    fn should_error_on_module_syntax_in_script() {
        let allocator = Allocator::default();
        let diagnostic = parse_program(
            &allocator,
            ParseParams {
                specifier: ModuleSpecifier::parse("file:///my_file.cjs").unwrap(),
                text: "export class Test {}".into(),
                media_type: MediaType::Cjs,
                capture_tokens: true,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .err()
        .unwrap();
        assert_eq!(
            diagnostic.message(),
            "'import', and 'export' cannot be used outside of module code"
        );
    }

    #[test]
    fn should_get_leading_comments_after_hashbang() {
        let allocator = Allocator::default();
        let program = parse_program(
            &allocator,
            ParseParams {
                specifier: ModuleSpecifier::parse("file:///my_file.js").unwrap(),
                text: "#!/bin/sh deno\n// 1\n".into(),
                media_type: MediaType::JavaScript,
                capture_tokens: true,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        assert_eq!(program.get_leading_comments().count(), 1);
    }

    #[test]
    fn test_parse_export_equals() {
        let allocator = Allocator::default();
        assert!(
            parse_program(
                &allocator,
                ParseParams {
                    specifier: ModuleSpecifier::parse("file:///my_file.ts").unwrap(),
                    text: "export = 5;".into(),
                    media_type: MediaType::Cts,
                    capture_tokens: false,
                    maybe_source_type: None,
                    scope_analysis: false,
                },
            )
            .is_ok()
        );
    }

    #[test]
    fn should_error_on_syntax_diagnostic() {
        let diagnostic = get_any_diagnostic("test;\nas#;");
        assert_ne!(diagnostic.message(), "");
    }

    #[test]
    fn should_error_without_issue_when_there_exists_multi_byte_char_on_line_with_syntax_error() {
        let diagnostic = get_any_diagnostic(concat!(
            "test;\n",
            r#"console.log("x", `duration ${d} not in range - ${min} ≥ ${d} && ${max} ≥ ${d}`),;"#,
        ));
        assert_ne!(diagnostic.message(), "");
    }

    #[test]
    fn should_diagnostic_for_no_equals_sign_in_var_decl() {
        let diagnostic = get_any_diagnostic("const Methods {\nf: (x, y) => x + y,\n};");
        assert_ne!(diagnostic.message(), "");
    }

    #[test]
    fn should_diganotic_when_var_stmts_sep_by_comma() {
        let diagnostic = get_any_diagnostic("let a = 0, let b = 1;");
        assert_ne!(diagnostic.message(), "");
    }

    #[test]
    fn should_diagnostic_for_exected_expr_type_alias() {
        let diagnostic = get_any_diagnostic("type T =\n  | unknown\n  { } & unknown;");
        assert_ne!(diagnostic.message(), "");
    }

    #[test]
    fn should_diganotic_missing_init_in_using() {
        let diagnostic = get_any_diagnostic("using test");
        assert_ne!(diagnostic.message(), "");
    }

    #[test]
    fn should_handle_parse_error_display_position() {
        let allocator = Allocator::default();
        let diagnostic = parse_module(
            &allocator,
            ParseParams {
                specifier: ModuleSpecifier::parse("file:///my_file.js").unwrap(),
                text: "t u".into(),
                media_type: MediaType::JavaScript,
                capture_tokens: true,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .err()
        .unwrap();
        assert_eq!(
            diagnostic.display_position(),
            crate::LineAndColumnDisplay { line_number: 1, column_number: 2 }
        );
    }

    fn get_any_diagnostic(text: &str) -> ParseDiagnostic {
        let allocator = Allocator::default();
        match parse_module(
            &allocator,
            ParseParams {
                specifier: ModuleSpecifier::parse("file:///my_file.ts").unwrap(),
                text: text.to_string().into(),
                media_type: MediaType::TypeScript,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        ) {
            Err(diagnostic) => diagnostic,
            Ok(parsed) => {
                parsed.diagnostics().first().expect("Expected at least one diagnostic").to_owned()
            }
        }
    }

    fn parse_ts_module(text: &str) -> Result<ParseDiagnostic, ParseDiagnostic> {
        let allocator = Allocator::default();
        parse_module(
            &allocator,
            ParseParams {
                specifier: ModuleSpecifier::parse("file:///my_file.ts").unwrap(),
                text: text.to_string().into(),
                media_type: MediaType::TypeScript,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .map(|parsed| parsed.diagnostics().first().cloned().unwrap())
    }

    #[test]
    fn should_parse_brand_checks_in_js() {
        use oxc::{
            ast::ast::{ClassElement, Statement},
            span::GetSpan,
        };

        let allocator = Allocator::default();
        let program = parse_module(
            &allocator,
            ParseParams {
                specifier: ModuleSpecifier::parse("file:///my_file.js").unwrap(),
                text: "class T { method() { #test in this; } }".into(),
                media_type: MediaType::JavaScript,
                capture_tokens: true,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();

        let Statement::ClassDeclaration(class_decl) = &program.body()[0] else {
            panic!("expected a class declaration");
        };
        let ClassElement::MethodDefinition(method) = &class_decl.body.body[0] else {
            panic!("expected a method");
        };
        let method_stmt = &method.value.body.as_ref().unwrap().statements[0];
        assert_eq!(method_stmt.span().source_text(program.text()), "#test in this;");
    }

    #[test]
    fn test_err_trailing_blank_line() {
        let diagnostic = parse_ts_module("setTimeout(() => {}),\n").err().unwrap();
        assert_eq!(
            diagnostic.to_string(),
            // should contain some context by including the previous line
            // instead of just a blank line
            [
                "SyntaxError: Unexpected token",
                "  |",
                "1 | setTimeout(() => {}),",
                "  |                      ~",
                "    at file:///my_file.ts:1:22",
            ]
            .join("\n")
        );
    }

    #[test]
    fn test_err_many_trailing_blank_lines() {
        let diagnostic = parse_ts_module("setTimeout(() => {}),\n\n\n\n\n\n\n\n").err().unwrap();
        assert_eq!(
            diagnostic.to_string(),
            // should contain some context by including the expression
            [
                "SyntaxError: Unexpected token",
                "  |",
                "1 | setTimeout(() => {}),",
                "  |                      ~",
                "    at file:///my_file.ts:1:22",
            ]
            .join("\n")
        );
    }

    #[test]
    fn test_strip_bom() {
        let text = "\u{FEFF}test";
        assert_eq!(strip_bom(text.to_string()), "test");
        let text = "test";
        assert_eq!(strip_bom(text.to_string()), "test");
        let text = "";
        assert_eq!(strip_bom(text.to_string()), "");
    }

    #[test]
    fn test_strip_bom_arc() {
        let text = "\u{FEFF}test";
        assert_eq!(strip_bom_from_arc(text.into(), false), "test".into());
        let text = "test";
        assert_eq!(strip_bom_from_arc(text.into(), false), "test".into());
        let text = "";
        assert_eq!(strip_bom_from_arc(text.into(), false), "".into());
    }
}
