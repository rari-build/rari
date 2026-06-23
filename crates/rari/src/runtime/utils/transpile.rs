// Copyright 2018-2025 the Deno authors. MIT license.

#![allow(clippy::exhaustive_structs)]

use std::borrow::Cow;
use std::path::Path;

use cow_utils::CowUtils;
use deno_ast::MediaType;
use deno_ast::ParseParams;
use deno_ast::SourceMapOption;
use deno_core::ModuleCodeString;
use deno_core::ModuleName;
use deno_core::SourceMapData;
use deno_error::JsErrorBox;

deno_error::js_error_wrapper!(deno_ast::ParseDiagnostic, JsParseDiagnostic, "Error");
deno_error::js_error_wrapper!(deno_ast::TranspileError, JsTranspileError, "Error");

/// NOTE: In deno_node >= 0.190.0, this is exported as `deno_node::NODE_VERSION`.
/// When we upgrade, we should use that constant directly to stay aligned.
const NODE_VERSION: &str = "26.3.0";

fn maybe_substitute_version_placeholders(name: &str, source: ModuleCodeString) -> ModuleCodeString {
    const NODE_VERSION_TOKEN: &str = "__NODE_VERSION__";
    const V8_VERSION_TOKEN: &str = "__V8_VERSION__";

    if name != "ext:init_node/init_node.js" {
        return source;
    }

    let source_str = source.as_str();

    if !source_str.contains(NODE_VERSION_TOKEN) && !source_str.contains(V8_VERSION_TOKEN) {
        return source;
    }

    let result: Cow<str> = source_str.cow_replace(NODE_VERSION_TOKEN, NODE_VERSION);

    if result.contains(V8_VERSION_TOKEN) {
        let final_result: Cow<str> =
            result.cow_replace(V8_VERSION_TOKEN, deno_core::v8::VERSION_STRING);
        return final_result.into_owned().into();
    }

    result.into_owned().into()
}

pub fn maybe_transpile_source(
    name: ModuleName,
    source: ModuleCodeString,
) -> Result<(ModuleCodeString, Option<SourceMapData>), JsErrorBox> {
    let name_string = name.to_string();
    let source = maybe_substitute_version_placeholders(&name_string, source);
    let media_type = if name.starts_with("node:") {
        MediaType::TypeScript
    } else {
        MediaType::from_path(Path::new(&name))
    };

    match media_type {
        MediaType::TypeScript | MediaType::Tsx | MediaType::Jsx => {}
        MediaType::JavaScript | MediaType::Mjs => return Ok((source, None)),
        _ => {
            return Err(JsErrorBox::from_err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Unsupported media type for transpilation: {media_type:?} for file {name}"),
            )));
        }
    }

    let parsed = deno_ast::parse_module(ParseParams {
        #[expect(clippy::expect_used, reason = "Infallible operation with valid inputs")]
        specifier: deno_core::url::Url::parse(&name).expect("invalid module name url"),
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

    let maybe_source_map: Option<SourceMapData> = transpiled_source
        .source_map
        .map(|sm| sm.into_bytes().into());
    let source_text = transpiled_source.text;
    Ok((source_text.into(), maybe_source_map))
}
