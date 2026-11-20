use std::fmt::Write as _;

use rari_ast::oxc::{ast::ast::Function, span::Span};

use crate::directive;

fn js_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn slice(source: &str, span: Span) -> &str {
    &source[span.start as usize..span.end as usize]
}

fn params_text<'s>(source: &'s str, function: &Function<'_>) -> &'s str {
    let text = slice(source, function.params.span);
    let text = text.strip_prefix('(').unwrap_or(text);
    text.strip_suffix(')').unwrap_or(text)
}

fn params_text_without_this<'s>(
    source: &'s str,
    function: &Function<'_>,
) -> (Option<&'s str>, &'s str) {
    let params = params_text(source, function);
    let Some(this_param) = &function.this_param else {
        return (None, params);
    };

    let params_start = function.params.span.start as usize + 1;
    let this_end = this_param.span.end as usize - params_start;
    let mut remaining_start = this_end;
    while params.as_bytes().get(remaining_start).is_some_and(u8::is_ascii_whitespace) {
        remaining_start += 1;
    }
    if params.as_bytes().get(remaining_start) == Some(&b',') {
        remaining_start += 1;
    }

    let this_text = &params[..this_end];
    (Some(this_text), params[remaining_start..].trim_start())
}

fn body_without_directives(source: &str, function: &Function<'_>) -> String {
    let Some(body) = function.body.as_ref() else {
        return "{}".to_string();
    };
    let mut removals =
        directive::leading_framework_directive_spans(&body.directives, &body.statements);
    removals.sort_by_key(|span| span.start);
    for span in &mut removals {
        let mut end = span.end as usize;
        if source.as_bytes().get(end) == Some(&b';') {
            end += 1;
        }
        if let Ok(end) = u32::try_from(end) {
            span.end = end;
        }
    }

    let mut out = String::with_capacity((body.span.end - body.span.start) as usize);
    let mut cursor = body.span.start as usize;
    for span in removals {
        out.push_str(&source[cursor..span.start as usize]);
        cursor = span.end as usize;
    }
    out.push_str(&source[cursor..body.span.end as usize]);
    out
}

fn param_count(function: &Function<'_>) -> usize {
    function.params.items.len() + usize::from(function.params.rest.is_some())
}

#[non_exhaustive]
pub struct CacheDeclarationInput<'a, 'f> {
    pub source: &'a str,
    pub function: &'a Function<'f>,
    pub function_name: &'a str,
    pub export_name: &'a str,
    pub closure_vars: &'a [String],
    pub ref_id: &'a str,
    pub cache_kind: &'a str,
    pub cache_name: &'a str,
    pub inner_name: &'a str,
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Struct is used as a parameter pack, ownership not needed but consistent API"
)]
pub fn create_cache_declarations(input: CacheDeclarationInput<'_, '_>) -> Vec<String> {
    let function = input.function;
    let source = input.source;

    let mut params = String::new();
    if input.closure_vars.is_empty() {
        params.push_str(params_text(source, function));
    } else if function.this_param.is_some() {
        let (this_param, original_params) = params_text_without_this(source, function);
        if let Some(this_param) = this_param {
            params.push_str(this_param);
        }
        if !params.is_empty() {
            params.push_str(", ");
        }
        params.push_str("$$ACTION_BOUND_ARGS");
        if !original_params.trim().is_empty() {
            params.push_str(", ");
            params.push_str(original_params);
        }
    } else {
        params.push_str("$$ACTION_BOUND_ARGS");
        let original_params = params_text(source, function);
        if !original_params.trim().is_empty() {
            params.push_str(", ");
            params.push_str(original_params);
        }
    }

    let mut inner = format!("const {} = ", input.inner_name);
    if function.r#async {
        inner.push_str("async ");
    }
    inner.push_str("function");
    if function.generator {
        inner.push('*');
    }
    let _ = write!(inner, " {}", input.function_name);
    if let Some(type_params) = &function.type_parameters {
        inner.push_str(slice(source, type_params.span));
    }
    let _ = write!(inner, "({params})");
    if let Some(return_type) = &function.return_type {
        inner.push_str(slice(source, return_type.span));
    }
    let _ = write!(inner, " {};", body_without_directives(source, function));

    let name_define = format!(
        "Object.defineProperty({}, \"name\", {{\n    value: {}\n}});",
        input.inner_name,
        js_string(input.export_name),
    );

    let wrapper = format!(
        "var {} = function() {{\n    return $$cache__({}, {}, {}, {}, Array.prototype.slice.call(arguments));\n}};",
        input.cache_name,
        js_string(input.cache_kind),
        js_string(input.ref_id),
        param_count(function),
        input.inner_name,
    );

    let register = format!(
        "registerServerReference({}, {}, null);",
        input.cache_name,
        js_string(input.ref_id),
    );

    vec![inner, name_define, wrapper, register]
}

fn try_body(cache_name: &str, has_bound_args: bool, indent: usize) -> String {
    let pad = " ".repeat(indent);
    let apply_args = if has_bound_args {
        format!("{pad}            $$ACTION_BOUND_ARGS,\n{pad}            ...args")
    } else {
        format!("{pad}            ...args")
    };
    format!(
        "{{\n{pad}    try {{\n{pad}        return {cache_name}.apply(null, [\n{apply_args}\n{pad}        ]);\n{pad}    }} catch (e) {{\n{pad}        if (e !== null && typeof e.then === \"function\") return await e;\n{pad}        throw e;\n{pad}    }}\n{pad}}}"
    )
}

pub fn create_bound_replacement_init(
    cache_name: &str,
    ref_id: &str,
    closure_vars: &[String],
) -> String {
    if closure_vars.is_empty() {
        return format!("async function(...args) {}", try_body(cache_name, false, 0));
    }

    let mut bound_args = format!("[\n    {}", js_string(ref_id));
    for var_name in closure_vars {
        let _ = write!(bound_args, ",\n    {var_name}");
    }
    bound_args.push_str("\n]");

    format!(
        "(($$ACTION_BOUND_ARGS)=>async (...args)=>{})({bound_args})",
        try_body(cache_name, true, 4),
    )
}

#[cfg(test)]
mod tests {
    use rari_ast::oxc::{
        allocator::Allocator, ast::ast::Statement, parser::Parser, span::SourceType,
    };

    use super::*;

    #[test]
    fn adjacent_cache_directives_are_removed_without_joining_return() {
        let source = r#"async function f(){ "use cache";"use server";return 1 }"#;
        let allocator = Allocator::default();
        let ret = Parser::new(&allocator, source, SourceType::tsx()).parse();
        assert!(matches!(ret.program.body.first(), Some(Statement::FunctionDeclaration(_))));
        let Some(Statement::FunctionDeclaration(function)) = ret.program.body.first() else {
            return;
        };
        let body = body_without_directives(source, function);
        assert!(body.contains("return 1"));
        assert!(!body.contains(r#""use cache""#));
        assert!(!body.contains(r#""use server""#));
    }

    #[test]
    fn explicit_this_parameter_is_not_a_runtime_argument() {
        let source =
            "async function f(this: { value: number }, a: number) { \"use cache\"; return a }";
        let allocator = Allocator::default();
        let ret = Parser::new(&allocator, source, SourceType::tsx()).parse();
        assert!(matches!(ret.program.body.first(), Some(Statement::FunctionDeclaration(_))));
        let Some(Statement::FunctionDeclaration(function)) = ret.program.body.first() else {
            return;
        };
        assert_eq!(param_count(function), 1);
        let declarations = create_cache_declarations(CacheDeclarationInput {
            source,
            function,
            function_name: "f",
            export_name: "f",
            closure_vars: &["captured".to_string()],
            ref_id: "id",
            cache_kind: "default",
            cache_name: "cache",
            inner_name: "inner",
        });
        assert!(
            declarations[0]
                .contains("function f(this: { value: number }, $$ACTION_BOUND_ARGS, a: number)")
        );
    }
}
