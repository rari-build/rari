#![expect(clippy::exhaustive_structs)]

use std::{
    borrow::Cow,
    io::{Error, ErrorKind::InvalidInput},
    path::Path,
};

use cow_utils::CowUtils;
use deno_ast::{MediaType, ParseParams, SourceMapOption};
use deno_core::{ModuleCodeString, ModuleName, SourceMapData, url::Url, v8::VERSION_STRING};
use deno_error::JsErrorBox;

deno_error::js_error_wrapper!(deno_ast::ParseDiagnostic, JsParseDiagnostic, "Error");
deno_error::js_error_wrapper!(deno_ast::TranspileError, JsTranspileError, "Error");

const NODE_VERSION_TOKEN: &str = "__NODE_VERSION__";
const V8_VERSION_TOKEN: &str = "__V8_VERSION__";

pub fn substitute_version_placeholders_in_source(source: &str) -> String {
    if !source.contains(NODE_VERSION_TOKEN) && !source.contains(V8_VERSION_TOKEN) {
        return source.to_string();
    }

    let result: Cow<str> = source.cow_replace(NODE_VERSION_TOKEN, deno_node::NODE_VERSION);

    if result.contains(V8_VERSION_TOKEN) {
        return result.cow_replace(V8_VERSION_TOKEN, VERSION_STRING).into_owned();
    }

    result.into_owned()
}

fn is_init_node_module(name: &str) -> bool {
    name.ends_with("/init_node.ts") || name == "init_node.ts"
}

fn maybe_substitute_version_placeholders(name: &str, source: ModuleCodeString) -> ModuleCodeString {
    if !is_init_node_module(name) {
        return source;
    }

    substitute_version_placeholders_in_source(source.as_str()).into()
}

#[expect(clippy::missing_errors_doc)]
pub fn maybe_transpile_source(
    name: &ModuleName,
    source: ModuleCodeString,
) -> Result<(ModuleCodeString, Option<SourceMapData>), JsErrorBox> {
    let name_string = name.to_string();
    let source = maybe_substitute_version_placeholders(&name_string, source);
    let media_type = if name.starts_with("node:") {
        MediaType::TypeScript
    } else if name.starts_with("ext:") {
        MediaType::from_path(Path::new(&name_string))
    } else {
        MediaType::from_path(Path::new(&name))
    };

    match media_type {
        MediaType::TypeScript | MediaType::Tsx | MediaType::Jsx => {}
        MediaType::JavaScript | MediaType::Mjs => return Ok((source, None)),
        _ => {
            return Err(JsErrorBox::from_err(Error::new(
                InvalidInput,
                format!("Unsupported media type for transpilation: {media_type:?} for file {name}"),
            )));
        }
    }

    let parsed = deno_ast::parse_module(ParseParams {
        specifier: Url::parse(name).unwrap_or_else(|_| {
            #[expect(clippy::expect_used, reason = "Fallback URL construction is infallible")]
            Url::parse(&format!("file:///__rari_script__/{name}"))
                .expect("failed to create fallback URL")
        }),
        text: source.into(),
        media_type,
        capture_tokens: false,
        scope_analysis: false,
        maybe_syntax: None,
    })
    .map_err(|e| JsErrorBox::from_err(JsParseDiagnostic(e)))?;
    let transpiled_source = parsed
        .transpile(
            &deno_ast::TranspileOptions {
                imports_not_used_as_values: deno_ast::ImportsNotUsedAsValues::Remove,
                ..Default::default()
            },
            &deno_ast::TranspileModuleOptions::default(),
            &deno_ast::EmitOptions {
                source_map: if cfg!(debug_assertions) {
                    SourceMapOption::Separate
                } else {
                    SourceMapOption::None
                },
                ..Default::default()
            },
        )
        .map_err(|e| JsErrorBox::from_err(JsTranspileError(e)))?
        .into_source();

    let maybe_source_map: Option<SourceMapData> =
        transpiled_source.source_map.map(|sm| sm.into_bytes().into());
    let source_text = transpiled_source.text;
    Ok((source_text.into(), maybe_source_map))
}
