// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use base64::Engine;
use oxc::{
    ast::ast::Program,
    codegen::{Codegen, CodegenOptions, CodegenReturn, CommentOptions, IndentChar, LegalComment},
};
use thiserror::Error;

use crate::ModuleSpecifier;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceMapOption {
    /// Source map should be inlined into the source (default)
    #[default]
    Inline,
    /// Source map should be generated as a separate file.
    Separate,
    /// Source map should not be generated at all.
    None,
}

#[derive(Debug, Clone, Hash)]
pub struct EmitOptions {
    /// How and if source maps should be generated.
    pub source_map: SourceMapOption,
    /// Base url to use for source maps.
    pub source_map_base: Option<ModuleSpecifier>,
    /// The `"file"` field of the generated source map.
    pub source_map_file: Option<String>,
    /// Whether to inline the source contents in the source map. Defaults to `true`.
    pub inline_sources: bool,
    /// Whether to remove comments in the output. Defaults to `false`.
    pub remove_comments: bool,
    /// Whether to escape non-ASCII characters in the output as `\uXXXX`.
    /// Defaults to `false`.
    pub ascii_only: bool,
}

impl Default for EmitOptions {
    fn default() -> Self {
        Self {
            source_map: SourceMapOption::default(),
            source_map_base: None,
            source_map_file: None,
            inline_sources: true,
            remove_comments: false,
            ascii_only: false,
        }
    }
}

/// Source emitted based on the emit options.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
pub struct EmittedSourceText {
    /// Emitted text as utf8 bytes.
    pub text: String,
    /// Source map back to the original file.
    pub source_map: Option<String>,
}

#[derive(Debug, Error, deno_error::JsError)]
pub enum EmitError {
    #[class(type)]
    #[error("Source map error: {0}")]
    SourceMap(String),
    #[class(type)]
    #[error(transparent)]
    SourceMapEncode(#[from] base64::EncodeSliceError),
}

/// Emits the program as a string of JavaScript code with optional source map.
pub fn emit(
    program: &Program<'_>,
    source_text: &str,
    file_name: &str,
    emit_options: &EmitOptions,
) -> Result<EmittedSourceText, EmitError> {
    let need_source_map = emit_options.source_map != SourceMapOption::None;

    let codegen_options = CodegenOptions {
        source_map_path: if need_source_map {
            Some(std::path::PathBuf::from(file_name))
        } else {
            None
        },
        indent_char: IndentChar::Space,
        indent_width: 2,
        ascii_only: emit_options.ascii_only,
        comments: if emit_options.remove_comments {
            CommentOptions {
                normal: false,
                jsdoc: false,
                annotation: false,
                legal: LegalComment::None,
            }
        } else {
            CommentOptions::default()
        },
        ..Default::default()
    };

    let codegen_ret: CodegenReturn =
        Codegen::new().with_options(codegen_options).with_source_text(source_text).build(program);

    let mut src_buf = codegen_ret.code.into_bytes();
    let mut map: Option<Vec<u8>> = None;

    if need_source_map && let Some(mut source_map) = codegen_ret.map {
        // Apply source_map_base: make source urls relative to the base url
        if let Some(base) = &emit_options.source_map_base {
            debug_assert!(base.as_str().ends_with('/'), "source map base should end with a slash");
            let new_sources: Vec<Option<String>> = source_map
                .get_sources()
                .map(|source| {
                    source.map(|source| {
                        ModuleSpecifier::parse(source)
                            .ok()
                            .and_then(|specifier| base.make_relative(&specifier))
                            .filter(|relative| !relative.is_empty())
                            .unwrap_or_else(|| source.to_string())
                    })
                })
                .collect();
            source_map.set_sources(new_sources);
        }

        // Apply source_map_file
        if let Some(ref file) = emit_options.source_map_file {
            source_map.set_file(file.as_str());
        }

        // Inline source contents if requested
        if emit_options.inline_sources {
            source_map.set_source_contents(vec![Some(source_text)]);
        }

        let map_buf = source_map.to_json_string().into_bytes();

        if emit_options.source_map == SourceMapOption::Inline {
            let mut inline_buf = vec![0; map_buf.len() * 4 / 3 + 4];
            let size = base64::prelude::BASE64_STANDARD
                .encode_slice(&map_buf, &mut inline_buf)
                .map_err(EmitError::SourceMapEncode)?;
            let inline_buf = &inline_buf[..size];
            let prelude_text = "//# sourceMappingURL=data:application/json;base64,";
            let src_has_trailing_newline = src_buf.ends_with(b"\n");
            let additional_capacity =
                usize::from(!src_has_trailing_newline) + prelude_text.len() + inline_buf.len();
            src_buf.reserve(additional_capacity);
            if !src_has_trailing_newline {
                src_buf.push(b'\n');
            }
            src_buf.extend(prelude_text.as_bytes());
            src_buf.extend(inline_buf);
        } else {
            map = Some(map_buf);
        }
    }

    Ok(EmittedSourceText {
        text: unsafe { String::from_utf8_unchecked(src_buf) },
        source_map: map.map(|b| unsafe { String::from_utf8_unchecked(b) }),
    })
}
