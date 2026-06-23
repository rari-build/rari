use deno_ast::swc::ast::{BlockStmt, Expr, ExprStmt, Lit, Stmt, Str};
use deno_ast::swc::ecma_visit::Visit;

#[non_exhaustive]
pub struct UseCacheDirective {
    pub found: bool,
    pub cache_kind: Option<String>,
}

impl Visit for UseCacheDirective {
    fn visit_stmt(&mut self, stmt: &Stmt) {
        if self.found {
            return;
        }
        if let Stmt::Expr(ExprStmt { expr, .. }) = stmt
            && let Expr::Lit(Lit::Str(Str { value, .. })) = expr.as_ref()
        {
            if value == "use cache" {
                self.found = true;
                self.cache_kind = None;
            } else if let Some(kind) = value.to_string_lossy().strip_prefix("use cache: ") {
                self.found = true;
                self.cache_kind = Some(kind.to_string());
            }
        }
    }
}

pub fn has_use_cache_directive(body: &BlockStmt) -> bool {
    let mut visitor = UseCacheDirective {
        found: false,
        cache_kind: None,
    };
    for stmt in &body.stmts {
        visitor.visit_stmt(stmt);
        if visitor.found {
            return true;
        }
        match stmt {
            Stmt::Expr(expr_stmt) => {
                if !matches!(*expr_stmt.expr, Expr::Lit(Lit::Str(_))) {
                    return false;
                }
            }
            Stmt::Empty(..) => {}
            _ => return false,
        }
    }
    false
}

pub fn extract_cache_kind(body: &BlockStmt) -> Option<String> {
    let mut visitor = UseCacheDirective {
        found: false,
        cache_kind: None,
    };
    for stmt in &body.stmts {
        visitor.visit_stmt(stmt);
        if visitor.found {
            return visitor.cache_kind;
        }
        match stmt {
            Stmt::Expr(expr_stmt) => {
                if !matches!(*expr_stmt.expr, Expr::Lit(Lit::Str(_))) {
                    return None;
                }
            }
            Stmt::Empty(..) => {}
            _ => return None,
        }
    }
    None
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
mod tests {
    #![allow(clippy::default_trait_access)]
    use super::*;

    #[test]
    fn test_detect_use_cache_directive() {
        let body = BlockStmt {
            span: Default::default(),
            ctxt: Default::default(),
            stmts: vec![Stmt::Expr(ExprStmt {
                span: Default::default(),
                expr: Box::new(Expr::Lit(Lit::Str(Str {
                    span: Default::default(),
                    value: "use cache".into(),
                    raw: None,
                }))),
            })],
        };
        assert!(has_use_cache_directive(&body));
    }

    #[test]
    fn test_detect_no_directive() {
        let body = BlockStmt {
            span: Default::default(),
            ctxt: Default::default(),
            stmts: vec![],
        };
        assert!(!has_use_cache_directive(&body));
    }

    #[test]
    fn test_detect_use_cache_kind() {
        let body = BlockStmt {
            span: Default::default(),
            ctxt: Default::default(),
            stmts: vec![Stmt::Expr(ExprStmt {
                span: Default::default(),
                expr: Box::new(Expr::Lit(Lit::Str(Str {
                    span: Default::default(),
                    value: "use cache: stale-while-revalidate".into(),
                    raw: None,
                }))),
            })],
        };
        assert!(has_use_cache_directive(&body));
        assert_eq!(
            extract_cache_kind(&body),
            Some("stale-while-revalidate".to_string())
        );
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
    fn test_non_directive_statement_stops_search() {
        let body = BlockStmt {
            span: Default::default(),
            ctxt: Default::default(),
            stmts: vec![Stmt::Expr(ExprStmt {
                span: Default::default(),
                expr: Box::new(Expr::Lit(Lit::Str(Str {
                    span: Default::default(),
                    value: "use server".into(),
                    raw: None,
                }))),
            })],
        };
        assert!(!has_use_cache_directive(&body));
    }
}
