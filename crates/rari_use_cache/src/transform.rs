use std::{error, fmt, panic};

use rari_ast::oxc::{
    allocator::Allocator,
    ast::ast::{Declaration, ExportDefaultDeclarationKind, Function, Program, Statement},
    parser::Parser,
    span::{GetSpan, SourceType, Span},
};
use rustc_hash::FxHashSet;

use crate::{closure, directive, hoist, id};

const DEFAULT_EXPORT_BINDING: &str = "$$RSC_SERVER_CACHE_DEFAULT_EXPORT";

#[derive(Debug)]
#[non_exhaustive]
pub enum TransformError {
    Parse(String),
    Codegen(String),
    Utf8(String),
    Panic(String),
}

impl fmt::Display for TransformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "Parse error: {msg}"),
            Self::Codegen(msg) => write!(f, "Codegen error: {msg}"),
            Self::Utf8(msg) => write!(f, "UTF-8 error: {msg}"),
            Self::Panic(msg) => write!(f, "Panic: {msg}"),
        }
    }
}

impl error::Error for TransformError {}

#[non_exhaustive]
pub struct TransformOutput {
    pub code: String,
    pub needs_react_cache: bool,
    pub needs_cache_wrapper: bool,
    pub needs_register_ref: bool,
}

struct Edit {
    span: Span,
    text: String,
}

enum ExportShape {
    None,
    Named,
    Default,
}

struct CacheCandidate<'p, 'a> {
    function: &'p Function<'a>,
    shape: ExportShape,
    reference_export_name: String,
    local_binding_name: String,
}

fn candidate<'p, 'a>(stmt: &'p Statement<'a>) -> Option<CacheCandidate<'p, 'a>> {
    match stmt {
        Statement::FunctionDeclaration(function) => {
            let name = function.id.as_ref()?.name.to_string();
            Some(CacheCandidate {
                function,
                shape: ExportShape::None,
                reference_export_name: name.clone(),
                local_binding_name: name,
            })
        }
        Statement::ExportDeclaration(export) => match &export.declaration {
            Declaration::FunctionDeclaration(function) => {
                let name = function.id.as_ref()?.name.to_string();
                Some(CacheCandidate {
                    function,
                    shape: ExportShape::Named,
                    reference_export_name: name.clone(),
                    local_binding_name: name,
                })
            }
            _ => None,
        },
        Statement::ExportDefaultDeclaration(export) => match &export.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                let name = function.id.as_ref().map(|id| id.name.to_string());
                Some(CacheCandidate {
                    function,
                    shape: ExportShape::Default,
                    reference_export_name: name.clone().unwrap_or_else(|| "default".to_string()),
                    local_binding_name: name.unwrap_or_else(|| DEFAULT_EXPORT_BINDING.to_string()),
                })
            }
            _ => None,
        },
        _ => None,
    }
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "Boolean flags track independent transform states, grouping into enum would be less clear"
)]
struct TransformVisitor<'s> {
    source: &'s str,
    filename: &'s str,
    hash_salt: &'s str,
    index: usize,
    has_cache_fns: bool,
    file_cache_kind: Option<String>,
    module_idents: FxHashSet<String>,
    needs_react_cache: bool,
    needs_cache_wrapper: bool,
    needs_register_ref: bool,
    edits: Vec<Edit>,
}

impl<'s> TransformVisitor<'s> {
    fn new(source: &'s str, filename: &'s str, hash_salt: &'s str) -> Self {
        Self {
            source,
            filename,
            hash_salt,
            index: 0,
            has_cache_fns: false,
            file_cache_kind: None,
            module_idents: FxHashSet::default(),
            needs_react_cache: false,
            needs_cache_wrapper: false,
            needs_register_ref: false,
            edits: Vec::new(),
        }
    }

    fn visit_program(&mut self, program: &Program<'_>) {
        self.file_cache_kind =
            directive::extract_file_level_cache_kind(&program.directives, &program.body);

        for stmt in &program.body {
            self.module_idents.extend(closure::collect_module_level_idents(stmt));
        }

        for span in directive::file_level_cache_directive_spans(&program.directives, &program.body)
        {
            self.edits.push(Edit { span, text: String::new() });
        }

        for stmt in &program.body {
            if let Some(text) = self.maybe_transform_item(stmt) {
                self.edits.push(Edit { span: stmt.span(), text });
            }
        }
    }

    fn maybe_transform_item(&mut self, stmt: &Statement<'_>) -> Option<String> {
        let candidate = candidate(stmt)?;
        let function = candidate.function;
        let body = function.body.as_ref()?;

        if !function.r#async {
            return None;
        }

        let body_has_directive =
            directive::has_use_cache_directive(&body.directives, &body.statements);
        if !body_has_directive && self.file_cache_kind.is_none() {
            return None;
        }

        self.has_cache_fns = true;
        let cache_kind = if body_has_directive {
            directive::extract_cache_kind(&body.directives, &body.statements)
                .filter(|kind| !kind.is_empty())
                .unwrap_or_else(|| "default".to_string())
        } else {
            self.file_cache_kind.clone().unwrap_or_else(|| "default".to_string())
        };

        let fn_params = closure::collect_fn_params(function);
        let closure_vars = closure::collect_closure_idents(
            body,
            &self.module_idents,
            &fn_params,
            Some(&candidate.local_binding_name),
        );

        let export_name = &candidate.reference_export_name;
        let ref_id = id::generate_reference_id(self.hash_salt, self.filename, export_name, true);
        let cache_name = id::generate_cache_export_name(self.index, export_name);
        let inner_name = id::generate_cache_inner_name(self.index, export_name);

        let mut statements = hoist::create_cache_declarations(hoist::CacheDeclarationInput {
            source: self.source,
            function,
            function_name: &candidate.local_binding_name,
            export_name,
            closure_vars: &closure_vars,
            ref_id: &ref_id,
            cache_kind: &cache_kind,
            cache_name: &cache_name,
            inner_name: &inner_name,
        });

        self.needs_cache_wrapper = true;
        self.needs_register_ref = true;
        self.index += 1;

        let init = hoist::create_bound_replacement_init(&cache_name, &ref_id, &closure_vars);
        let local = &candidate.local_binding_name;
        match candidate.shape {
            ExportShape::None => statements.push(format!("var {local} = {init};")),
            ExportShape::Named => statements.push(format!("export var {local} = {init};")),
            ExportShape::Default => {
                statements.push(format!("var {local} = {init};"));
                statements.push(format!("export default {local};"));
            }
        }

        Some(statements.join("\n"))
    }
}

fn apply_edits(source: &str, mut edits: Vec<Edit>) -> String {
    edits.sort_by_key(|edit| edit.span.start);
    let mut out =
        String::with_capacity(source.len() + edits.iter().map(|e| e.text.len()).sum::<usize>());
    let mut cursor = 0usize;
    for edit in edits {
        let start = edit.span.start as usize;
        if start < cursor {
            continue;
        }
        out.push_str(&source[cursor..start]);
        out.push_str(&edit.text);
        cursor = edit.span.end as usize;
    }
    out.push_str(&source[cursor..]);
    out
}

fn unparsed_source_start(program: &Program<'_>, source: &str) -> Option<u32> {
    let parsed_end = program
        .body
        .last()
        .map(GetSpan::span)
        .map(|span| span.end)
        .into_iter()
        .chain(program.directives.last().map(|directive| directive.span.end))
        .chain(program.hashbang.as_ref().map(|hashbang| hashbang.span.end))
        .max()
        .unwrap_or(0);
    let mut cursor = parsed_end as usize;
    loop {
        let rest = source.get(cursor..)?;
        let (offset, _) = rest
            .char_indices()
            .find(|(_, character)| !character.is_whitespace() && *character != '\u{FEFF}')?;
        cursor += offset;
        if let Some(comment) =
            program.comments.iter().find(|comment| comment.span.start as usize == cursor)
        {
            cursor = comment.span.end as usize;
        } else {
            return u32::try_from(cursor).ok();
        }
    }
}

/// Transforms source code to add cache functionality.
///
/// # Errors
///
/// Returns [`TransformError`] if the source cannot be parsed or the transform
/// panics.
pub fn transform_source(
    source: &str,
    filename: &str,
    hash_salt: &str,
) -> Result<TransformOutput, TransformError> {
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
        if parsed.fatal_error {
            let message =
                parsed.diagnostics.iter().map(ToString::to_string).collect::<Vec<_>>().join("; ");
            return Err(TransformError::Parse(message));
        }
        if !parsed.diagnostics.is_empty()
            && let Some(unparsed_start) = unparsed_source_start(&parsed.program, source)
        {
            let message =
                parsed.diagnostics.iter().map(ToString::to_string).collect::<Vec<_>>().join("; ");
            return Err(TransformError::Parse(format!(
                "{message} (unparsed input at byte {unparsed_start})"
            )));
        }

        let mut visitor = TransformVisitor::new(source, filename, hash_salt);
        visitor.visit_program(&parsed.program);

        if !visitor.has_cache_fns {
            return Ok(TransformOutput {
                code: source.to_string(),
                needs_react_cache: false,
                needs_cache_wrapper: false,
                needs_register_ref: false,
            });
        }

        Ok(TransformOutput {
            code: apply_edits(source, visitor.edits),
            needs_react_cache: visitor.needs_react_cache,
            needs_cache_wrapper: visitor.needs_cache_wrapper,
            needs_register_ref: visitor.needs_register_ref,
        })
    }));

    match result {
        Ok(inner) => inner,
        Err(panic) => {
            let msg = if let Some(s) = panic.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = panic.downcast_ref::<&str>() {
                (*s).to_string()
            } else {
                "Unknown panic during transformation".to_string()
            };
            Err(TransformError::Panic(msg))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_fatal_partial_parse() {
        let result = transform_source(
            "async function f() { \"use cache\"; return 1 }\n@",
            "partial.ts",
            "test",
        );
        assert!(matches!(result, Err(TransformError::Parse(_))));
    }

    #[test]
    fn rejects_partial_parse_before_trailing_comment() {
        let result = transform_source(
            "async function f() { \"use cache\"; return 1 }\n@ // trailing",
            "partial.ts",
            "test",
        );
        assert!(matches!(result, Err(TransformError::Parse(_))));
    }

    #[test]
    fn accepts_trailing_comments_and_whitespace() {
        let source = "async function f() { \"use cache\"; return 1 }\n// trailing comment\n  ";
        let result = transform_source(source, "trailing.ts", "test");
        assert!(result.is_ok());
    }
}
