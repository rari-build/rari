use rari_ast::oxc::{
    ast::ast::{Directive, Expression, Statement},
    span::{GetSpan, Span},
};

struct PrologueEntry<'s> {
    span: Span,
    value: &'s str,
}

fn prologue<'s>(
    directives: &'s [Directive<'_>],
    statements: &'s [Statement<'_>],
) -> Vec<PrologueEntry<'s>> {
    let mut entries: Vec<PrologueEntry<'s>> = directives
        .iter()
        .map(|d| PrologueEntry {
            span: d.span,
            value: d.expression.value.as_str().unwrap_or_default(),
        })
        .collect();
    for stmt in statements {
        match stmt {
            Statement::ExpressionStatement(expr_stmt) => match &expr_stmt.expression {
                Expression::StringLiteral(lit) => {
                    entries.push(PrologueEntry {
                        span: expr_stmt.span,
                        value: lit.value.as_str().unwrap_or_default(),
                    });
                }
                _ => break,
            },
            Statement::EmptyStatement(_) => {}
            _ => break,
        }
    }
    entries
}

enum UseCache {
    Plain,
    Kind(String),
}

fn parse_use_cache(value: &str) -> Option<UseCache> {
    if value == "use cache" {
        return Some(UseCache::Plain);
    }
    value.strip_prefix("use cache: ").map(|kind| UseCache::Kind(kind.to_string()))
}

pub fn has_use_cache_directive(directives: &[Directive<'_>], statements: &[Statement<'_>]) -> bool {
    prologue(directives, statements).iter().any(|entry| parse_use_cache(entry.value).is_some())
}

pub fn extract_cache_kind(
    directives: &[Directive<'_>],
    statements: &[Statement<'_>],
) -> Option<String> {
    match prologue(directives, statements).iter().find_map(|entry| parse_use_cache(entry.value))? {
        UseCache::Plain => None,
        UseCache::Kind(kind) => Some(kind),
    }
}

fn cache_kind_from_directive_value(value: &str) -> Option<String> {
    parse_use_cache(value).map(|directive| match directive {
        UseCache::Plain => "default".to_string(),
        UseCache::Kind(kind) => kind,
    })
}

pub fn extract_file_level_cache_kind(
    directives: &[Directive<'_>],
    statements: &[Statement<'_>],
) -> Option<String> {
    prologue(directives, statements)
        .iter()
        .find_map(|entry| cache_kind_from_directive_value(entry.value))
}

pub fn file_level_cache_directive_spans(
    directives: &[Directive<'_>],
    statements: &[Statement<'_>],
) -> Vec<Span> {
    let mut spans: Vec<Span> = directives
        .iter()
        .filter(|d| {
            cache_kind_from_directive_value(d.expression.value.as_str().unwrap_or_default())
                .is_some()
        })
        .map(|d| d.span)
        .collect();
    for stmt in statements {
        if let Statement::ExpressionStatement(expr_stmt) = stmt
            && let Expression::StringLiteral(lit) = &expr_stmt.expression
            && cache_kind_from_directive_value(lit.value.as_str().unwrap_or_default()).is_some()
        {
            spans.push(stmt.span());
        }
    }
    spans
}

pub fn leading_framework_directive_spans(
    directives: &[Directive<'_>],
    statements: &[Statement<'_>],
) -> Vec<Span> {
    let mut spans = Vec::new();
    for entry in prologue(directives, statements) {
        let value = entry.value;
        let is_framework_directive = value == "use cache"
            || value.starts_with("use cache: ")
            || value == "use server"
            || value == "use client";
        if !is_framework_directive {
            break;
        }
        spans.push(entry.span);
    }
    spans
}

pub fn detect_use_cache(source: &str) -> bool {
    let bytes = source.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        let q = bytes[i] as char;
        if q != '"' && q != '\'' && q != '`' {
            i += 1;
            continue;
        }
        let rest = &source[i + 1..];
        let trimmed = rest.trim_start();

        if let Some(after_directive) = trimmed.strip_prefix("use cache") {
            let after_directive = after_directive.trim_start();
            if after_directive.starts_with(q) {
                return true;
            }
            if let Some(kind_rest) = after_directive.strip_prefix(':') {
                let kind_rest = kind_rest.trim_start();
                let kind_end = kind_rest
                    .find(|c: char| !c.is_alphanumeric() && c != '-')
                    .unwrap_or(kind_rest.len());
                if kind_end > 0 && kind_rest[kind_end..].starts_with(q) {
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}

#[cfg(test)]
#[expect(clippy::panic, reason = "Test helpers fail fast when a snippet parses unexpectedly")]
mod tests {
    use rari_ast::oxc::{
        allocator::Allocator,
        ast::ast::{Function, Statement},
        parser::Parser,
        span::SourceType,
    };

    use super::*;

    fn with_function<R>(source: &str, check: impl FnOnce(&Function<'_>) -> R) -> R {
        let allocator = Allocator::default();
        let ret = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let Some(Statement::FunctionDeclaration(function)) = ret.program.body.first() else {
            panic!("expected a function declaration");
        };
        check(function)
    }

    fn body_has_directive(source: &str) -> bool {
        with_function(source, |f| {
            f.body.as_ref().is_some_and(|b| has_use_cache_directive(&b.directives, &b.statements))
        })
    }

    #[test]
    fn test_detect_use_cache_directive() {
        assert!(body_has_directive("async function f() { 'use cache' }"));
    }

    #[test]
    fn test_detect_no_directive() {
        assert!(!body_has_directive("async function f() {}"));
    }

    #[test]
    fn test_detect_use_cache_kind() {
        let source = "async function f() { \"use cache: stale-while-revalidate\" }";
        assert!(body_has_directive(source));
        let kind = with_function(source, |f| {
            f.body.as_ref().and_then(|b| extract_cache_kind(&b.directives, &b.statements))
        });
        assert_eq!(kind, Some("stale-while-revalidate".to_string()));
    }

    #[test]
    fn test_directive_after_other_statement_is_ignored() {
        assert!(!body_has_directive("async function f() { const x = 1; 'use cache' }"));
    }

    #[test]
    fn test_directive_after_empty_statement_is_detected() {
        assert!(body_has_directive("async function f() { 'use strict';; 'use cache' }"));
    }

    #[test]
    fn test_detect_use_cache_via_string_scan() {
        assert!(detect_use_cache("\"use cache\""));
        assert!(detect_use_cache("'use cache'"));
        assert!(detect_use_cache("\"use cache: stale-while-revalidate\""));
        assert!(detect_use_cache("'use cache: fresh'"));
        assert!(!detect_use_cache("const x = 1;"));
        assert!(!detect_use_cache("usecache is not a thing"));
    }

    #[test]
    fn test_detect_use_cache_remote_kind() {
        assert!(detect_use_cache("\"use cache: remote\""));
        assert!(detect_use_cache("'use cache: remote'"));
        assert!(detect_use_cache("`use cache: remote`"));
    }

    #[test]
    fn test_extract_remote_cache_kind_from_ast() {
        let source = "async function f() { \"use cache: remote\" }";
        assert!(body_has_directive(source));
        let kind = with_function(source, |f| {
            f.body.as_ref().and_then(|b| extract_cache_kind(&b.directives, &b.statements))
        });
        assert_eq!(kind, Some("remote".to_string()));
    }

    #[test]
    fn test_file_level_skips_non_cache_string_literals() {
        let allocator = Allocator::default();
        let ret = Parser::new(&allocator, "'use strict'\n'use cache'\n", SourceType::tsx()).parse();
        assert_eq!(
            extract_file_level_cache_kind(&ret.program.directives, &ret.program.body),
            Some("default".to_string())
        );
    }

    #[test]
    fn test_non_directive_statement_stops_search() {
        assert!(!body_has_directive("async function f() { \"use server\" }"));
    }
}
