// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use std::{borrow::Cow, path::Path};

use deno_error::JsError;
use deno_media_type::MediaType;
use oxc::{
    allocator::{Allocator, CloneIn},
    ast::ast::{Program, Statement},
    ast_visit::VisitMut,
    semantic::{Scoping, SemanticBuilder},
    transformer::{DecoratorOptions, JsxOptions, TransformOptions, Transformer, TypeScriptOptions},
};
use thiserror::Error;

use crate::{
    EmitError, EmitOptions, EmittedSourceText, ModuleKind, ModuleSpecifier, ParseDiagnostic,
    ParseDiagnostics, ParseDiagnosticsError, ParsedSource, emit,
};

mod transforms;

/// Holds whether the `ParsedSource` was cloned or consumed (owned) during
/// transpilation. This is useful for logging in the CLI when transpilation
/// occurs when a `ParsedSource` is cloned, as it's a performance issue.
#[derive(Debug, Clone)]
pub enum TranspileResult {
    /// The `ParsedSource` needed to be cloned in order to transpile.
    ///
    /// This is a performance issue and you should strive to get an `Owned` result.
    Cloned(EmittedSourceText),
    /// The emit occurred consuming the `ParsedSource` without cloning.
    Owned(EmittedSourceText),
}

impl TranspileResult {
    pub fn into_source(self) -> EmittedSourceText {
        match self {
            Self::Owned(source) | Self::Cloned(source) => source,
        }
    }
}

#[derive(Debug, Error, JsError)]
pub enum TranspileError {
    /// Parse errors that prevent transpiling.
    #[class(inherit)]
    #[error(transparent)]
    ParseErrors(#[from] ParseDiagnosticsError),
    #[class(inherit)]
    #[error(transparent)]
    TransformProgram(#[from] TransformProgramError),
    #[class(type)]
    #[error("{0}")]
    EmitDiagnostic(String),
    #[class(inherit)]
    #[error(transparent)]
    Emit(#[from] EmitError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImportsNotUsedAsValues {
    Remove,
    Preserve,
    Error,
}

#[derive(Debug, Clone, Hash)]
pub struct JsxClassicOptions {
    /// When transforming JSX, what value should be used for the JSX factory.
    /// Defaults to `React.createElement`.
    pub factory: String,
    /// When transforming JSX, what value should be used for the JSX fragment
    /// factory.  Defaults to `React.Fragment`.
    pub fragment_factory: String,
}

impl Default for JsxClassicOptions {
    fn default() -> Self {
        Self { factory: "React.createElement".into(), fragment_factory: "React.Fragment".into() }
    }
}

#[derive(Debug, Clone, Hash)]
pub struct JsxAutomaticOptions {
    /// If it is in development mode, meaning that it should import `jsx-dev-runtime`
    /// and transform JSX using `jsxDEV` import from the JSX import source as well as
    /// provide additional debug information to the JSX factory.
    pub development: bool,
    /// The string module specifier to implicitly import JSX factories from when
    /// transpiling JSX.
    pub import_source: Option<String>,
}

#[derive(Debug, Clone, Hash)]
pub enum JsxRuntime {
    Classic(JsxClassicOptions),
    Automatic(JsxAutomaticOptions),
}

impl Default for JsxRuntime {
    fn default() -> Self {
        Self::Classic(JsxClassicOptions::default())
    }
}

impl JsxRuntime {
    pub fn automatic(&self) -> Option<&JsxAutomaticOptions> {
        match self {
            Self::Classic(_) => None,
            Self::Automatic(o) => Some(o),
        }
    }

    pub fn classic(&self) -> Option<&JsxClassicOptions> {
        match self {
            Self::Classic(o) => Some(o),
            Self::Automatic(_) => None,
        }
    }
}

#[derive(Debug, Default, Clone, Hash)]
pub enum DecoratorsTranspileOption {
    /// Leave decorators as-is.
    #[default]
    None,
    /// TC39 Decorators Proposal - https://github.com/tc39/proposal-decorators
    Ecma,
    /// TypeScript experimental decorators.
    LegacyTypeScript {
        /// Also emit experimental decorator meta data.
        emit_metadata: bool,
    },
}

/// Options which can be adjusted when transpiling a module.
///
/// This implements `Hash` so the CLI can use it to bust the emit cache.
#[derive(Debug, Clone, Hash)]
pub struct TranspileOptions {
    /// Kind of decorators to use.
    pub decorators: DecoratorsTranspileOption,
    /// `true` changes type stripping behaviour so that _only_ `type` imports
    /// are stripped.
    pub verbatim_module_syntax: bool,
    /// What to do with import statements that only import types i.e. whether to
    /// remove them (`Remove`), keep them as side-effect imports (`Preserve`)
    /// or error (`Error`). Defaults to `Remove`.
    pub imports_not_used_as_values: ImportsNotUsedAsValues,
    /// Options for transforming JSX. Will not transform when `None`.
    pub jsx: Option<JsxRuntime>,
    /// Should import declarations be transformed to variable declarations using
    /// a dynamic import. This is useful for import & export declaration support
    /// in script contexts such as the Deno REPL.  Defaults to `false`.
    pub var_decl_imports: bool,
}

impl Default for TranspileOptions {
    fn default() -> Self {
        Self {
            decorators: DecoratorsTranspileOption::default(),
            verbatim_module_syntax: false,
            imports_not_used_as_values: ImportsNotUsedAsValues::Remove,
            jsx: Some(JsxRuntime::default()),
            var_decl_imports: false,
        }
    }
}

/// Transpile options specific to the module being transpiled.
///
/// This is separate from `TranspileOptions` in order for that to
/// be shared across modules, but this to be changed per module.
#[derive(Debug, Default, Clone, Hash)]
pub struct TranspileModuleOptions {
    /// The kind of module being transpiled.
    ///
    /// Defaults to being derived from the media type of the parsed source.
    pub module_kind: Option<ModuleKind>,
}

impl<'a> ParsedSource<'a> {
    /// Transform a TypeScript file into a JavaScript file.
    pub fn transpile(
        &mut self,
        allocator: &'a Allocator,
        transpile_options: &TranspileOptions,
        transpile_module_options: &TranspileModuleOptions,
        emit_options: &EmitOptions,
    ) -> Result<EmittedSourceText, TranspileError> {
        let mut program = self.program.clone_in(allocator);
        let module_kind = transpile_module_options.module_kind.unwrap_or({
            if matches!(self.media_type, MediaType::Cjs | MediaType::Cts) {
                ModuleKind::Cjs
            } else {
                ModuleKind::Esm
            }
        });
        ensure_no_fatal_diagnostics(self.diagnostics.for_module_kind(module_kind))?;
        ensure_supported_decorators(transpile_options)?;

        // A CommonJS module without ES module syntax is transpiled as a script so
        // that `import x = require()` / `export =` lower to `require` /
        // `module.exports` and the JSX import source is required instead of
        // imported.
        if module_kind.is_cjs()
            && program.source_type.is_module()
            && crate::parsed_source::compute_is_script_from_body(&program.body)
        {
            transforms::CjsScriptExports::new(allocator).transform(&mut program);
        }

        let transpile_options = resolve_transpile_options(self.media_type, transpile_options);

        // If @jsxImportSource pragma is found in comments, switch to automatic
        // runtime to match SWC/Babel behavior where @jsxImportSource implies
        // automatic runtime.
        let transpile_options = maybe_upgrade_jsx_for_pragma(
            &transpile_options,
            &self.get_leading_comments().cloned().collect::<Vec<_>>(),
            self.text.as_ref(),
        );

        // Apply custom pre-transforms
        if transpile_options.var_decl_imports {
            let mut strip_exports = transforms::StripExports::new(allocator);
            strip_exports.visit_program(&mut program);

            // Transform import declarations to variable declarations before
            // the TS pass so that the transformer doesn't optimize them away
            let mut import_to_var = transforms::ImportDeclsToVarDecls::new(allocator);
            import_to_var.visit_program(&mut program);
        }

        // Build OXC TransformOptions
        let transform_options = build_transform_options(&transpile_options, &self.specifier);

        // Run semantic analysis to get scoping info. This also checks for early
        // errors the parser doesn't report (ex. legacy octal literals in strict
        // mode).
        let semantic_ret = SemanticBuilder::new()
            .with_check_syntax_error(true)
            .with_enum_eval(true)
            .build(&program);
        if semantic_ret.diagnostics.has_errors() {
            let text_info = self.text_info_lazy().clone();
            return Err(TranspileError::ParseErrors(ParseDiagnosticsError(
                semantic_ret
                    .diagnostics
                    .into_iter()
                    .filter(|err| err.severity == oxc::diagnostics::Severity::Error)
                    .map(|err| {
                        ParseDiagnostic::from_oxc_diagnostic(
                            err,
                            &self.specifier,
                            text_info.clone(),
                        )
                    })
                    .collect(),
            )));
        }
        let scoping = semantic_ret.semantic.into_scoping();
        if transpile_options.imports_not_used_as_values == ImportsNotUsedAsValues::Error {
            ensure_no_type_only_imports(&program, &scoping)?;
        } else if transpile_options.imports_not_used_as_values == ImportsNotUsedAsValues::Preserve {
            transforms::preserve_type_only_imports(&mut program, &scoping);
        }

        // Run OXC transformer (TS strip, JSX, decorators, etc.)
        let transformer =
            Transformer::new(allocator, Path::new(self.specifier.as_str()), &transform_options);
        let transform_ret = transformer.build_with_scoping(scoping, &mut program);
        ensure_no_transform_errors(&transform_ret.diagnostics)?;

        transforms::SortClassElementsBySourceOrder.visit_program(&mut program);

        // Apply custom post-transforms: transform any import declarations
        // inserted by the JSX transform to variable declarations
        let has_jsx_transform = transpile_options.jsx.is_some();
        if transpile_options.var_decl_imports && has_jsx_transform {
            let mut import_to_var = transforms::ImportDeclsToVarDecls::new(allocator);
            import_to_var.visit_program(&mut program);
        }

        // Emit
        let file_name = self.specifier.as_str();
        Ok(emit(&program, self.text.as_ref(), file_name, emit_options)?)
    }
}

fn resolve_transpile_options(
    media_type: MediaType,
    options: &TranspileOptions,
) -> Cow<'_, TranspileOptions> {
    if options.jsx.is_some() {
        let allows_jsx = matches!(media_type, MediaType::Jsx | MediaType::Tsx);
        if !allows_jsx {
            return Cow::Owned(TranspileOptions { jsx: None, ..(options.clone()) });
        }
    }

    Cow::Borrowed(options)
}

/// Scan leading comments for `@jsxImportSource` pragma. If found and the
/// current JSX runtime is Classic, upgrade to Automatic to match the behavior
/// of SWC/Babel where `@jsxImportSource` implies automatic runtime.
fn maybe_upgrade_jsx_for_pragma<'a>(
    options: &Cow<'a, TranspileOptions>,
    comments: &[oxc::ast::ast::Comment],
    source_text: &str,
) -> Cow<'a, TranspileOptions> {
    let Some(import_source) = jsx_import_source_pragma(comments, source_text) else {
        return options.clone();
    };

    match &options.jsx {
        Some(JsxRuntime::Classic(_)) => Cow::Owned(TranspileOptions {
            jsx: Some(JsxRuntime::Automatic(JsxAutomaticOptions {
                development: false,
                import_source: Some(import_source),
            })),
            ..(options.as_ref().clone())
        }),
        Some(JsxRuntime::Automatic(automatic)) => Cow::Owned(TranspileOptions {
            jsx: Some(JsxRuntime::Automatic(JsxAutomaticOptions {
                import_source: Some(import_source),
                ..automatic.clone()
            })),
            ..(options.as_ref().clone())
        }),
        None => options.clone(),
    }
}

fn jsx_import_source_pragma(
    comments: &[oxc::ast::ast::Comment],
    source_text: &str,
) -> Option<String> {
    comments.iter().find_map(|comment| {
        let text = comment.content_span().source_text(source_text);
        parse_jsx_import_source_pragma(text)
    })
}

fn parse_jsx_import_source_pragma(text: &str) -> Option<String> {
    const PRAGMA: &str = "@jsxImportSource";
    text.lines().find_map(|line| {
        let line = line.trim();
        let line = line.strip_prefix('*').unwrap_or(line).trim();
        let after_pragma = line.strip_prefix(PRAGMA)?;
        if !after_pragma.chars().next().is_some_and(char::is_whitespace) {
            return None;
        }
        let mut values = after_pragma.split_whitespace();
        let import_source = values.next()?;
        values.next().is_none().then(|| import_source.to_string())
    })
}

fn build_transform_options(
    options: &TranspileOptions,
    _specifier: &ModuleSpecifier,
) -> TransformOptions {
    let typescript = TypeScriptOptions {
        only_remove_type_imports: options.verbatim_module_syntax,
        // inline `const enum` values like TypeScript does
        optimize_const_enums: true,
        ..Default::default()
    };

    let jsx = if let Some(jsx_runtime) = &options.jsx {
        match jsx_runtime {
            JsxRuntime::Classic(classic) => JsxOptions {
                jsx_plugin: true,
                display_name_plugin: false,
                jsx_self_plugin: false,
                jsx_source_plugin: false,
                runtime: oxc::transformer::JsxRuntime::Classic,
                development: false,
                throw_if_namespace: false,
                pure: true,
                import_source: None,
                pragma: Some(classic.factory.clone()),
                pragma_frag: Some(classic.fragment_factory.clone()),
                ..Default::default()
            },
            JsxRuntime::Automatic(automatic) => JsxOptions {
                jsx_plugin: true,
                display_name_plugin: false,
                jsx_self_plugin: automatic.development,
                jsx_source_plugin: automatic.development,
                runtime: oxc::transformer::JsxRuntime::Automatic,
                development: automatic.development,
                throw_if_namespace: false,
                pure: true,
                import_source: automatic.import_source.clone(),
                ..Default::default()
            },
        }
    } else {
        JsxOptions { jsx_plugin: false, ..Default::default() }
    };

    let decorator = match &options.decorators {
        DecoratorsTranspileOption::None => DecoratorOptions::default(),
        DecoratorsTranspileOption::Ecma => DecoratorOptions {
            legacy: false,
            emit_decorator_metadata: false,
            strict_null_checks: true,
        },
        DecoratorsTranspileOption::LegacyTypeScript { emit_metadata } => DecoratorOptions {
            legacy: true,
            emit_decorator_metadata: *emit_metadata,
            strict_null_checks: true,
        },
    };

    TransformOptions { typescript, jsx, decorator, ..Default::default() }
}

#[derive(Debug, Error, JsError)]
pub enum TransformProgramError {
    #[class(inherit)]
    #[error(transparent)]
    ParseDiagnostics(#[from] ParseDiagnosticsError),
    #[class(syntax)]
    #[error("{0}")]
    Transform(String),
}

/// Low level function for transforming a program using OXC's transformer.
pub fn transform_program<'a>(
    allocator: &'a Allocator,
    program: &mut Program<'a>,
    options: &TranspileOptions,
    specifier: &ModuleSpecifier,
    diagnostics: &ParseDiagnostics,
    module_kind: ModuleKind,
) -> Result<(), TransformProgramError> {
    ensure_no_fatal_diagnostics(diagnostics.for_module_kind(module_kind))?;
    ensure_supported_decorators(options)?;

    let transform_options = build_transform_options(options, specifier);

    // Run semantic analysis
    let semantic_ret = SemanticBuilder::new().with_enum_eval(true).build(program);
    let scoping = semantic_ret.semantic.into_scoping();
    if options.imports_not_used_as_values == ImportsNotUsedAsValues::Error {
        ensure_no_type_only_imports(program, &scoping)?;
    } else if options.imports_not_used_as_values == ImportsNotUsedAsValues::Preserve {
        transforms::preserve_type_only_imports(program, &scoping);
    }

    // Run transformer
    let transformer =
        Transformer::new(allocator, Path::new(specifier.as_str()), &transform_options);
    let transform_ret = transformer.build_with_scoping(scoping, program);

    ensure_no_transform_errors(&transform_ret.diagnostics)?;
    transforms::SortClassElementsBySourceOrder.visit_program(program);

    Ok(())
}

fn ensure_supported_decorators(options: &TranspileOptions) -> Result<(), TransformProgramError> {
    if matches!(options.decorators, DecoratorsTranspileOption::Ecma) {
        return Err(TransformProgramError::Transform(
            "TC39 decorators proposal transpilation is not supported".to_string(),
        ));
    }
    Ok(())
}

fn ensure_no_type_only_imports(
    program: &Program<'_>,
    scoping: &Scoping,
) -> Result<(), TransformProgramError> {
    for stmt in &program.body {
        let Statement::ImportDeclaration(import_decl) = stmt else {
            continue;
        };
        if import_decl.import_kind.is_type() {
            continue;
        }
        let Some(specifiers) = import_decl.specifiers.as_ref() else {
            continue;
        };

        for specifier in specifiers {
            if matches!(
                specifier,
                oxc::ast::ast::ImportDeclarationSpecifier::ImportSpecifier(import_specifier)
                    if import_specifier.import_kind.is_type()
            ) {
                continue;
            }
            let mut has_type_reference = false;
            let mut has_value_reference = false;
            for reference in scoping.get_resolved_references(specifier.local().symbol_id()) {
                if reference.is_type() {
                    has_type_reference = true;
                } else {
                    has_value_reference = true;
                }
            }
            if has_type_reference && !has_value_reference {
                return Err(TransformProgramError::Transform(format!(
                    "Import '{}' is only used as a type; use an explicit type-only import",
                    specifier.local().name.as_str()
                )));
            }
        }
    }
    Ok(())
}

fn ensure_no_transform_errors(
    diagnostics: &oxc::diagnostics::Diagnostics,
) -> Result<(), TransformProgramError> {
    if diagnostics.has_errors() {
        let messages: Vec<String> =
            diagnostics.errors().map(std::string::ToString::to_string).collect();
        return Err(TransformProgramError::Transform(messages.join("\n")));
    }
    Ok(())
}

fn ensure_no_fatal_diagnostics<'a>(
    diagnostics: Box<dyn Iterator<Item = &'a ParseDiagnostic> + 'a>,
) -> Result<(), ParseDiagnosticsError> {
    let fatal_diagnostics = diagnostics.filter(|d| d.is_fatal()).cloned().collect::<Vec<_>>();
    if fatal_diagnostics.is_empty() {
        Ok(())
    } else {
        Err(ParseDiagnosticsError(fatal_diagnostics))
    }
}

#[cfg(test)]
#[expect(
    clippy::absolute_paths,
    clippy::unwrap_used,
    reason = "Ported code not yet updated to the workspace lint rules"
)]
mod tests {
    use base64::Engine;
    use oxc::allocator::Allocator;

    use super::*;
    use crate::{
        MediaType, ModuleSpecifier, ParseParams, SourceMapOption, parse_module, parse_program,
    };

    /// Compares a source map with the expected one, ignoring the `mappings`
    /// (which depend on the source map generator) beyond checking that there
    /// are some.
    #[track_caller]
    fn assert_source_map_eq(actual: &str, expected: &str) {
        let mut actual: serde_json::Value = serde_json::from_str(actual).unwrap();
        let mappings = actual.as_object_mut().unwrap().remove("mappings").unwrap();
        assert_ne!(mappings.as_str().unwrap(), "");
        let expected: serde_json::Value = serde_json::from_str(expected).unwrap();
        assert_eq!(actual, expected);
    }

    /// Splits emitted text into the code and the decoded inline source map.
    #[track_caller]
    fn split_inline_source_map(text: &str) -> (&str, serde_json::Value) {
        let prefix = "//# sourceMappingURL=data:application/json;base64,";
        let index = text.rfind(prefix).unwrap();
        let encoded = text[index + prefix.len()..].trim_end();
        let decoded = base64::prelude::BASE64_STANDARD.decode(encoded).unwrap();
        (&text[..index], serde_json::from_slice(&decoded).unwrap())
    }

    #[test]
    fn jsx_import_source_pragma_requires_a_complete_directive() {
        assert_eq!(
            parse_jsx_import_source_pragma("* @jsxImportSource jsx_lib"),
            Some("jsx_lib".to_string())
        );
        assert_eq!(parse_jsx_import_source_pragma("@jsxImportSource"), None);
        assert_eq!(parse_jsx_import_source_pragma("@jsxImportSourceExtra jsx_lib"), None);
        assert_eq!(parse_jsx_import_source_pragma("@jsxImportSource jsx_lib trailing"), None);
    }

    #[test]
    fn test_transpile() {
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.ts").unwrap();
        let source = r#"
enum D {
  A,
  B,
}

const enum E {
  A,
  B,
}

console.log(E.A);

namespace N {
  export enum D {
    A = "value"
  }
  export const Value = 5;
}

export class A {
  private b: string;
  protected c: number = 1;
  e: "foo";
  constructor (public d = D.A) {
    const e = "foo" as const;
    this.e = e;
    console.log(N.Value);
  }
}
    "#;
        let allocator = Allocator::default();
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::TypeScript,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        let transpiled_source = program
            .transpile(
                &allocator,
                &TranspileOptions::default(),
                &TranspileModuleOptions::default(),
                &EmitOptions::default(),
            )
            .unwrap();
        let expected_text = r#"var D = /* @__PURE__ */ function(D) {
  D[D["A"] = 0] = "A";
  D[D["B"] = 1] = "B";
  return D;
}(D || {});
console.log(0);
let N;
(function(_N) {
  let D = /* @__PURE__ */ function(D) {
    D["A"] = "value";
    return D;
  }(_N.D || {});
  _N.D = D;
  const Value = _N.Value = 5;
})(N || (N = {}));
export class A {
  b;
  c = 1;
  e;
  constructor(d = D.A) {
    this.d = d;
    const e = "foo";
    this.e = e;
    console.log(N.Value);
  }
  d;
}
"#;
        let (transpiled_code, _) = split_inline_source_map(&transpiled_source.text);
        assert_eq!(transpiled_code, expected_text);
        assert!(
            transpiled_source.text.contains("\n//# sourceMappingURL=data:application/json;base64,")
        );
        assert!(transpiled_source.source_map.is_none());
    }

    #[test]
    fn test_parameter_property_field_order_matches_runtime_initialization() {
        let allocator = Allocator::default();
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.ts").unwrap();
        let source = "class Example {\n  first = 1;\n  constructor(public second = 2) {}\n}";
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::TypeScript,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        let code = program
            .transpile(
                &allocator,
                &TranspileOptions::default(),
                &TranspileModuleOptions::default(),
                &EmitOptions { source_map: SourceMapOption::None, ..Default::default() },
            )
            .unwrap()
            .text;
        assert!(code.find("first = 1;").unwrap() < code.find("second;").unwrap());
    }

    #[test]
    fn test_explicit_resource_management() {
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.ts").unwrap();
        let source = r"Deno.test({
  permissions: {
    net: true
  }
}, async function listenerExplicitResourceManagement() {
  let done;
  {
    using listener = Deno.listen({
      port: listenPort
    });
    done = assertRejects(()=>listener.accept(), Deno.errors.BadResource);
  }
  await done;
});
";
        let allocator = Allocator::default();
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::TypeScript,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        let transpiled_source = program
            .transpile(
                &allocator,
                &TranspileOptions::default(),
                &TranspileModuleOptions::default(),
                &EmitOptions::default(),
            )
            .unwrap();
        // the `using` declaration is left as-is (only reformatted by the emitter)
        let expected_text = r"Deno.test({ permissions: { net: true } }, async function listenerExplicitResourceManagement() {
  let done;
  {
    using listener = Deno.listen({ port: listenPort });
    done = assertRejects(() => listener.accept(), Deno.errors.BadResource);
  }
  await done;
});
";
        assert_eq!(&transpiled_source.text[..expected_text.len()], expected_text);
    }

    #[test]
    fn test_transpile_tsx() {
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.ts").unwrap();
        let source = r"
    export class A {
      render() {
        return <div><span></span></div>
      }
    }
    ";
        let allocator = Allocator::default();
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::Tsx,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: true, // ensure scope analysis doesn't conflict with a second resolver pass
            },
        )
        .unwrap();
        let transpiled_source = program
            .transpile(
                &allocator,
                &TranspileOptions::default(),
                &TranspileModuleOptions::default(),
                &EmitOptions::default(),
            )
            .unwrap();
        assert!(transpiled_source.text.contains("React.createElement(\"div\", null"));
    }

    #[test]
    fn test_transpile_jsx_disabled() {
        let allocator = Allocator::default();
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.tsx").unwrap();
        let source = "const element = <div />;";
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::Tsx,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        let code = program
            .transpile(
                &allocator,
                &TranspileOptions { jsx: None, ..Default::default() },
                &TranspileModuleOptions::default(),
                &EmitOptions { source_map: SourceMapOption::None, ..Default::default() },
            )
            .unwrap()
            .text;
        assert_eq!(code, "const element = <div />;\n");
    }

    #[test]
    fn test_retranspile_parsed_source_with_different_jsx_options() {
        let allocator = Allocator::default();
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.tsx").unwrap();
        let source = "const element = <div />;";
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::Tsx,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        let transformed = program
            .transpile(
                &allocator,
                &TranspileOptions::default(),
                &TranspileModuleOptions::default(),
                &EmitOptions { source_map: SourceMapOption::None, ..Default::default() },
            )
            .unwrap()
            .text;
        let preserved = program
            .transpile(
                &allocator,
                &TranspileOptions { jsx: None, ..Default::default() },
                &TranspileModuleOptions::default(),
                &EmitOptions { source_map: SourceMapOption::None, ..Default::default() },
            )
            .unwrap()
            .text;
        assert!(transformed.contains("React.createElement(\"div\", null)"));
        assert_eq!(preserved, "const element = <div />;\n");
    }

    #[test]
    fn test_transpile_tsx_with_namespace() {
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.ts").unwrap();
        let source = r#"
    export class A {
      render() {
        return <my:tag><span my:attr="this"></span></my:tag>
      }
    }
    "#;
        let allocator = Allocator::default();
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::Tsx,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: true, // ensure scope analysis doesn't conflict with a second resolver pass
            },
        )
        .unwrap();
        let transpiled_source = program
            .transpile(
                &allocator,
                &TranspileOptions::default(),
                &TranspileModuleOptions::default(),
                &EmitOptions::default(),
            )
            .unwrap();
        assert!(transpiled_source.text.contains("React.createElement(\"my:tag\", null"));
        assert!(transpiled_source.text.contains("\"my:attr\": \"this\""));
    }

    #[test]
    fn test_transpile_jsx_pragma() {
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.ts").unwrap();
        let source = r#"
/** @jsx h */
/** @jsxFrag Fragment */
import { h, Fragment } from "https://deno.land/x/mod.ts";

function App() {
  return (
    <div><></></div>
  );
}"#;
        let allocator = Allocator::default();
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::Jsx,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: true,
            },
        )
        .unwrap();
        let code = program
            .transpile(
                &allocator,
                &TranspileOptions::default(),
                &TranspileModuleOptions::default(),
                &EmitOptions { remove_comments: true, ..Default::default() },
            )
            .unwrap()
            .text;
        let expected = r#"import { h, Fragment } from "https://deno.land/x/mod.ts";
function App() {
  return h("div", null, h(Fragment, null));
}"#;
        assert_eq!(&code[..expected.len()], expected);
    }

    #[test]
    fn test_transpile_jsx_import_source_pragma() {
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.tsx").unwrap();
        let source = r"
/** @jsxImportSource jsx_lib */

function App() {
  return (
    <div><></></div>
  );
}";
        let allocator = Allocator::default();
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::Jsx,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: true,
            },
        )
        .unwrap();
        let code = program
            .transpile(
                &allocator,
                &TranspileOptions::default(),
                &TranspileModuleOptions::default(),
                &EmitOptions { remove_comments: false, ..Default::default() },
            )
            .unwrap()
            .text;
        let expected = r#"import { Fragment as _Fragment, jsx as _jsx } from "jsx_lib/jsx-runtime";
/** @jsxImportSource jsx_lib */
function App() {
  return /* @__PURE__ */ _jsx("div", { children: /* @__PURE__ */ _jsx(_Fragment, {}) });
}
"#;
        assert_eq!(&code[..expected.len()], expected);
    }

    #[test]
    fn test_jsx_import_source_pragma_must_be_leading() {
        let allocator = Allocator::default();
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.tsx").unwrap();
        let source = r"const value = 1;
/** @jsxImportSource jsx_lib */
function App() { return <div />; }";
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::Tsx,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        let code = program
            .transpile(
                &allocator,
                &TranspileOptions::default(),
                &TranspileModuleOptions::default(),
                &EmitOptions { remove_comments: true, ..Default::default() },
            )
            .unwrap()
            .text;
        assert!(code.contains("React.createElement(\"div\", null)"));
        assert!(!code.contains("jsx_lib"));
    }

    #[test]
    fn test_transpile_jsx_import_source_no_pragma() {
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.tsx").unwrap();
        let source = r"
function App() {
  return (
    <div><></></div>
  );
}";
        let allocator = Allocator::default();
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::Jsx,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: true,
            },
        )
        .unwrap();
        let transpile_options = TranspileOptions {
            jsx: Some(JsxRuntime::Automatic(JsxAutomaticOptions {
                import_source: Some("jsx_lib".to_string()),
                development: false,
            })),
            ..Default::default()
        };
        let code = program
            .transpile(
                &allocator,
                &transpile_options,
                &TranspileModuleOptions::default(),
                &EmitOptions { remove_comments: true, ..Default::default() },
            )
            .unwrap()
            .text;
        let expected = r#"import { Fragment as _Fragment, jsx as _jsx } from "jsx_lib/jsx-runtime";
function App() {
  return _jsx("div", { children: _jsx(_Fragment, {}) });
}
"#;
        assert_eq!(&code[..expected.len()], expected);
    }

    #[test]
    fn test_transpile_jsx_import_source_no_pragma_dev() {
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.tsx").unwrap();
        let source = r"function App() {
  return (
    <div><></></div>
  );
}";
        let allocator = Allocator::default();
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::Jsx,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: true,
            },
        )
        .unwrap();
        let transpile_options = TranspileOptions {
            jsx: Some(JsxRuntime::Automatic(JsxAutomaticOptions {
                import_source: Some("jsx_lib".to_string()),
                development: true,
            })),
            ..Default::default()
        };
        let code = program
            .transpile(
                &allocator,
                &transpile_options,
                &TranspileModuleOptions::default(),
                &EmitOptions { remove_comments: true, ..Default::default() },
            )
            .unwrap()
            .text;
        let expected = r#"var _jsxFileName = "https://deno.land/x/mod.tsx";
import { Fragment as _Fragment, jsxDEV as _jsxDEV } from "jsx_lib/jsx-dev-runtime";
function App() {
  return _jsxDEV("div", { children: _jsxDEV(_Fragment, {}, void 0, false, {
    fileName: _jsxFileName,
    lineNumber: 3,
    columnNumber: 10
  }, this) }, void 0, false, {
    fileName: _jsxFileName,
    lineNumber: 3,
    columnNumber: 5
  }, this);
}
"#;
        assert_eq!(&code[..expected.len()], expected);
    }

    #[test]
    fn test_transpile_jsx_import_source_pragma_var_decl_imports() {
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.tsx").unwrap();
        let source = r#"
/** @jsxImportSource jsx_lib */
import * as example from "example";

function App() {
  return (
    <div><></></div>
  );
}"#;
        let allocator = Allocator::default();
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::Jsx,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: true,
            },
        )
        .unwrap();
        let transpile_options = TranspileOptions { var_decl_imports: true, ..Default::default() };
        let code = program
            .transpile(
                &allocator,
                &transpile_options,
                &TranspileModuleOptions::default(),
                &EmitOptions::default(),
            )
            .unwrap()
            .text;
        let expected = r#"const { Fragment: _Fragment, jsx: _jsx } = await import("jsx_lib/jsx-runtime");
/** @jsxImportSource jsx_lib */
const example = await import("example");
function App() {
  return /* @__PURE__ */ _jsx("div", { children: /* @__PURE__ */ _jsx(_Fragment, {}) });
}
"#;
        assert_eq!(&code[..expected.len()], expected);
    }

    #[test]
    fn test_transpile_jsx_import_source_cjs() {
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.tsx").unwrap();
        let source = r"
function App() {
  return (
    <div><></></div>
  );
}";
        let allocator = Allocator::default();
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::Jsx,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: true,
            },
        )
        .unwrap();
        let transpile_options = TranspileOptions {
            jsx: Some(JsxRuntime::Automatic(JsxAutomaticOptions {
                import_source: Some("jsx_lib".to_string()),
                development: false,
            })),
            ..Default::default()
        };
        let transpile_module_options = TranspileModuleOptions {
            module_kind: Some(ModuleKind::Cjs), // notice this
        };
        let code = program
            .transpile(
                &allocator,
                &transpile_options,
                &transpile_module_options,
                &EmitOptions { remove_comments: true, ..Default::default() },
            )
            .unwrap()
            .text;
        let expected = r#"var _reactJsxRuntime = require("jsx_lib/jsx-runtime");
function App() {
  return _reactJsxRuntime.jsx("div", { children: _reactJsxRuntime.jsx(_reactJsxRuntime.Fragment, {}) });
}
"#;
        assert_eq!(&code[..expected.len()], expected);
    }

    #[test]
    fn test_transpile_jsx_import_source_cjs_with_import_export() {
        let specifier = ModuleSpecifier::parse("file:///mod.tsx").unwrap();
        let source = r"
import test = require('./test');
export import other = require('./test');
console.log(test);
console.log(other);
export import asdf = other.add;
function App() {
  return (
    <div><></></div>
  );
}";
        let allocator = Allocator::default();
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::Tsx,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: true,
            },
        )
        .unwrap();
        let transpile_options = TranspileOptions {
            jsx: Some(JsxRuntime::Automatic(JsxAutomaticOptions {
                import_source: Some("jsx_lib".to_string()),
                development: false,
            })),
            ..Default::default()
        };
        let transpile_module_options = TranspileModuleOptions {
            module_kind: Some(ModuleKind::Cjs), // notice this
        };
        let code = program
            .transpile(
                &allocator,
                &transpile_options,
                &transpile_module_options,
                &EmitOptions { remove_comments: true, ..Default::default() },
            )
            .unwrap()
            .text;
        let expected = r#"var _reactJsxRuntime = require("jsx_lib/jsx-runtime");
const test = require("./test");
const other = require("./test");
exports.other = other;
console.log(test);
console.log(other);
var asdf = other.add;
exports.asdf = asdf;
function App() {
  return _reactJsxRuntime.jsx("div", { children: _reactJsxRuntime.jsx(_reactJsxRuntime.Fragment, {}) });
}
"#;
        assert_eq!(&code[..expected.len()], expected);
    }

    #[test]
    fn test_transpile_jsx_import_source_cjs_with_ts_export_equals() {
        let specifier = ModuleSpecifier::parse("file:///mod.tsx").unwrap();
        let source = r"
export = function App() {
  return (
    <div><></></div>
  );
}";
        let allocator = Allocator::default();
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::Tsx,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: true,
            },
        )
        .unwrap();
        let transpile_options = TranspileOptions {
            jsx: Some(JsxRuntime::Automatic(JsxAutomaticOptions {
                import_source: Some("jsx_lib".to_string()),
                development: false,
            })),
            ..Default::default()
        };
        let transpile_module_options = TranspileModuleOptions {
            module_kind: Some(ModuleKind::Cjs), // notice this
        };
        let code = program
            .transpile(
                &allocator,
                &transpile_options,
                &transpile_module_options,
                &EmitOptions { remove_comments: true, ..Default::default() },
            )
            .unwrap()
            .text;
        let expected = r#"var _reactJsxRuntime = require("jsx_lib/jsx-runtime");
module.exports = function App() {
  return _reactJsxRuntime.jsx("div", { children: _reactJsxRuntime.jsx(_reactJsxRuntime.Fragment, {}) });
};
"#;
        assert_eq!(&code[..expected.len()], expected);
    }

    #[test]
    #[ignore = "oxc only emits legacy decorators via a helper import from `@oxc-project/runtime` (its inline helper mode is not implemented), so the `_ts_decorate` helper is not inlined"]
    fn test_transpile_decorators() {
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.ts").unwrap();
        let source = r"
    function enumerable(value: boolean) {
      return function (
        _target: any,
        _propertyKey: string,
        descriptor: PropertyDescriptor,
      ) {
        descriptor.enumerable = value;
      };
    }

    export class A {
      @enumerable(false)
      a() {
        Test.value;
      }
    }
    ";
        let allocator = Allocator::default();
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::TypeScript,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        let code = program
            .transpile(
                &allocator,
                &TranspileOptions {
                    decorators: DecoratorsTranspileOption::LegacyTypeScript {
                        emit_metadata: false,
                    },
                    ..Default::default()
                },
                &TranspileModuleOptions::default(),
                &EmitOptions::default(),
            )
            .unwrap()
            .text;
        let expected = r#"function _ts_decorate(decorators, target, key, desc) {
  var c = arguments.length, r = c < 3 ? target : desc === null ? desc = Object.getOwnPropertyDescriptor(target, key) : desc, d;
  if (typeof Reflect === "object" && typeof Reflect.decorate === "function") r = Reflect.decorate(decorators, target, key, desc);
  else for(var i = decorators.length - 1; i >= 0; i--)if (d = decorators[i]) r = (c < 3 ? d(r) : c > 3 ? d(target, key, r) : d(target, key)) || r;
  return c > 3 && r && Object.defineProperty(target, key, r), r;
}
function enumerable(value) {
  return function(_target, _propertyKey, descriptor) {
    descriptor.enumerable = value;
  };
}
export class A {
  a() {
    Test.value;
  }
}
_ts_decorate([
  enumerable(false)
], A.prototype, "a", null);"#;
        assert_eq!(&code[0..expected.len()], expected);
    }

    #[test]
    fn test_transpile_decorators_proposal_is_unsupported() {
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.ts").unwrap();
        let source = r"
    function enumerable(value: boolean) {
      return function (
        _target: any,
        _propertyKey: string,
        descriptor: PropertyDescriptor,
      ) {
        descriptor.enumerable = value;
        return descriptor;
      };
    }

    export class A {
      @enumerable(false)
      a() {
        Test.value;
      }
    }
    ";
        let allocator = Allocator::default();
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::TypeScript,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        let error = program
            .transpile(
                &allocator,
                &TranspileOptions {
                    decorators: DecoratorsTranspileOption::Ecma,
                    ..Default::default()
                },
                &TranspileModuleOptions::default(),
                &EmitOptions::default(),
            )
            .unwrap_err();
        assert!(
            error.to_string().contains("TC39 decorators proposal transpilation is not supported")
        );
    }

    #[test]
    fn test_transpile_no_decorators() {
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.ts").unwrap();
        let source = r"
    function enumerable(value: boolean) {
      return function (
        _target: any,
        _propertyKey: string,
        descriptor: PropertyDescriptor,
      ) {
        descriptor.enumerable = value;
        return descriptor;
      };
    }

    export class A {
      @enumerable(false)
      a() {
        Test.value;
      }
    }
    ";
        let allocator = Allocator::default();
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::TypeScript,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        let code = program
            .transpile(
                &allocator,
                &TranspileOptions::default(),
                &TranspileModuleOptions::default(),
                &EmitOptions::default(),
            )
            .unwrap()
            .text;
        let expected = r"function enumerable(value) {
  return function(_target, _propertyKey, descriptor) {
    descriptor.enumerable = value;
    return descriptor;
  };
}
export class A {
  @enumerable(false) a() {
    Test.value;
  }
}
";
        assert_eq!(&code[0..expected.len()], expected);
    }

    #[test]
    fn transpile_handle_code_nested_in_ts_nodes_with_jsx_pass() {
        // from issue 12409
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.ts").unwrap();
        let source = r"
export function g() {
  let algorithm: any
  algorithm = {}

  return <Promise>(
    test(algorithm, false, keyUsages)
  )
}
  ";
        let allocator = Allocator::default();
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::TypeScript,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        let transpile_options =
            TranspileOptions { jsx: Some(JsxRuntime::default()), ..Default::default() };
        let code = program
            .transpile(
                &allocator,
                &transpile_options,
                &TranspileModuleOptions::default(),
                &EmitOptions::default(),
            )
            .unwrap()
            .text;
        let expected = r"export function g() {
  let algorithm;
  algorithm = {};
  return test(algorithm, false, keyUsages);
}";
        assert_eq!(&code[..expected.len()], expected);
    }

    #[test]
    fn transpile_bitshift_typescript() {
        // from https://github.com/denoland/deno/issues/14900
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.ts").unwrap();
        let source = r"
for (let i = 0; i < testVariable >> 1; i++) callCount++;
  ";
        let allocator = Allocator::default();
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::TypeScript,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        let code = program
            .transpile(
                &allocator,
                &TranspileOptions::default(),
                &TranspileModuleOptions::default(),
                &EmitOptions::default(),
            )
            .unwrap()
            .text;
        let expected = r"for (let i = 0; i < testVariable >> 1; i++) callCount++;";
        assert_eq!(&code[..expected.len()], expected);
    }

    #[test]
    fn jsx_spread_works() {
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.ts").unwrap();
        let source = r"const A = () => {
  return <div>{...[]}</div>;
};";
        let allocator = Allocator::default();
        let mut parsed_source = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::Tsx,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        assert!(
            parsed_source
                .transpile(
                    &allocator,
                    &TranspileOptions::default(),
                    &TranspileModuleOptions::default(),
                    &EmitOptions::default()
                )
                .is_ok()
        );
    }

    #[test]
    #[ignore = "oxc reports no error for a legacy octal literal in a module (swc reported two strict mode errors)"]
    fn diagnostic_octal_and_leading_zero_num_literals() {
        assert_eq!(
      get_diagnostic("077"),
      [
        "SyntaxError: Legacy octal literals are not available when targeting ECMAScript 5 and higher",
        "  |",
        "1 | 077",
        "  | ~~~",
        "    at https://deno.land/x/mod.ts:1:1",
        "",
        "SyntaxError: Legacy octal escape is not permitted in strict mode",
        "  |",
        "1 | 077",
        "  | ~~~",
        "    at https://deno.land/x/mod.ts:1:1",
      ]
      .join("\n")
    );
        assert_eq!(
            get_diagnostic("099"),
            [
                "SyntaxError: Legacy decimal escape is not permitted in strict mode",
                "  |",
                "1 | 099",
                "  | ~~~",
                "    at https://deno.land/x/mod.ts:1:1",
            ]
            .join("\n")
        );
    }

    #[test]
    fn diagnostic_missing_brace() {
        assert_eq!(
            get_diagnostic("function test() {"),
            [
                "SyntaxError: Expected `}` but found `EOF`",
                "  |",
                "1 | function test() {",
                "  |                  ~",
                "    at https://deno.land/x/mod.ts:1:18",
            ]
            .join("\n"),
        );
    }

    #[test]
    fn diagnostic_nullish_coalescing_with_logical_op() {
        assert_eq!(
            get_diagnostic("null || undefined ?? 'foo';"),
            [
                "SyntaxError: Logical expressions and coalesce expressions cannot be mixed",
                "  |",
                "1 | null || undefined ?? 'foo';",
                "  | ~~~~~~~~~~~~~~~~~~~~~~~~~~",
                "    at https://deno.land/x/mod.ts:1:1",
            ]
            .join("\n")
        );
        assert_eq!(
            get_diagnostic("null && undefined ?? 'foo';"),
            [
                "SyntaxError: Logical expressions and coalesce expressions cannot be mixed",
                "  |",
                "1 | null && undefined ?? 'foo';",
                "  | ~~~~~~~~~~~~~~~~~~~~~~~~~~",
                "    at https://deno.land/x/mod.ts:1:1",
            ]
            .join("\n"),
        );
    }

    #[test]
    fn diagnostic_missing_init_in_using() {
        assert_eq!(
            get_diagnostic("using test"),
            [
                "SyntaxError: Using declarations must have an initializer.",
                "  |",
                "1 | using test",
                "  |       ~~~~",
                "    at https://deno.land/x/mod.ts:1:7",
            ]
            .join("\n")
        );
    }

    #[test]
    fn diagnostic_invalid_left_hand_side_of_assignment() {
        assert_eq!(
            get_diagnostic("(true ? a : b) = 1;"),
            [
                "SyntaxError: Cannot assign to this expression",
                "  |",
                "1 | (true ? a : b) = 1;",
                "  |  ~~~~~~~~~~~~",
                "    at https://deno.land/x/mod.ts:1:2",
            ]
            .join("\n")
        );
    }

    fn get_diagnostic(source: &str) -> String {
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.ts").unwrap();
        let allocator = Allocator::default();
        let mut parsed_source = match parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::TypeScript,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        ) {
            Ok(parsed_source) => parsed_source,
            // unrecoverable syntax errors are reported when parsing
            Err(diagnostic) => return diagnostic.to_string(),
        };
        parsed_source
            .transpile(
                &allocator,
                &TranspileOptions::default(),
                &TranspileModuleOptions::default(),
                &EmitOptions::default(),
            )
            .err()
            .unwrap()
            .to_string()
    }

    #[test]
    fn source_map_properly_encoded() {
        // Ref https://github.com/denoland/deno/issues/10936
        // Ref https://github.com/swc-project/swc/issues/3288#issuecomment-1117252904

        let allocator = Allocator::default();
        let mut p = parse_program(
            &allocator,
            ParseParams {
                specifier: ModuleSpecifier::parse("file:///Users/ib/dev/deno/foo.ts").unwrap(),
                text: r#"export default function () {
    return "📣❓";
}"#
                .into(),
                media_type: MediaType::TypeScript,
                capture_tokens: true,
                scope_analysis: false,
                maybe_source_type: None,
            },
        )
        .unwrap();

        let transpiled = p
            .transpile(
                &allocator,
                &TranspileOptions::default(),
                &TranspileModuleOptions::default(),
                &EmitOptions::default(),
            )
            .unwrap();
        let lines: Vec<&str> = transpiled.text.split('\n').collect();
        let last_line = lines.last().unwrap();
        let input =
            last_line.trim_start_matches("//# sourceMappingURL=data:application/json;base64,");
        base64::prelude::BASE64_STANDARD.decode(input).unwrap();
    }

    #[test]
    fn test_verbatim_module_syntax() {
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.ts").unwrap();
        let source = r#"import type foo from "./foo.ts"; import bar from "./bar.ts"; import baz from "./baz.ts"; const b: baz = 1;"#;
        let allocator = Allocator::default();
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::TypeScript,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        let transpile_options =
            TranspileOptions { verbatim_module_syntax: true, ..Default::default() };
        let code = program
            .transpile(
                &allocator,
                &transpile_options,
                &TranspileModuleOptions::default(),
                &EmitOptions::default(),
            )
            .unwrap()
            .text;
        let (code, source_map) = split_inline_source_map(&code);
        assert_eq!(
            code,
            "import bar from \"./bar.ts\";\nimport baz from \"./baz.ts\";\nconst b = 1;\n"
        );
        assert_eq!(source_map["sourcesContent"], serde_json::json!([source]));
    }

    #[test]
    fn test_inline_source_map_newline() {
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.tsx").unwrap();
        let source = r#"{ const foo = "bar"; };"#;
        let allocator = Allocator::default();
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::Tsx,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        let emit_options =
            EmitOptions { source_map: SourceMapOption::Inline, ..Default::default() };
        let emit_result = program
            .transpile(
                &allocator,
                &TranspileOptions::default(),
                &TranspileModuleOptions::default(),
                &emit_options,
            )
            .unwrap();
        let (code, source_map) = split_inline_source_map(&emit_result.text);
        assert_eq!(code, "{\n  const foo = \"bar\";\n}\n;\n");
        assert_eq!(source_map["sources"], serde_json::json!(["https://deno.land/x/mod.tsx"]));
        assert_eq!(emit_result.source_map, None);
    }

    #[test]
    fn test_source_map() {
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.tsx").unwrap();
        let source = r#"{ const foo = "bar"; };"#;
        let allocator = Allocator::default();
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::Tsx,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        let emit_options =
            EmitOptions { source_map: SourceMapOption::Separate, ..Default::default() };
        let emit_result = program
            .transpile(
                &allocator,
                &TranspileOptions::default(),
                &TranspileModuleOptions::default(),
                &emit_options,
            )
            .unwrap();
        assert_eq!(
            &emit_result.text,
            r#"{
  const foo = "bar";
}
;
"#
        );
        assert_source_map_eq(
            emit_result.source_map.as_deref().unwrap(),
            r#"{"version":3,"sources":["https://deno.land/x/mod.tsx"],"sourcesContent":["{ const foo = \"bar\"; };"],"names":[]}"#,
        );
    }

    #[test]
    fn test_source_map_omits_contents_when_disabled() {
        let allocator = Allocator::default();
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.tsx").unwrap();
        let source = "const value = 1;";
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::Tsx,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        let result = program
            .transpile(
                &allocator,
                &TranspileOptions::default(),
                &TranspileModuleOptions::default(),
                &EmitOptions {
                    source_map: SourceMapOption::Separate,
                    inline_sources: false,
                    ..Default::default()
                },
            )
            .unwrap();
        let map: serde_json::Value =
            serde_json::from_str(result.source_map.as_deref().unwrap()).unwrap();
        assert!(map.get("sourcesContent").is_none_or(|contents| {
            contents
                .as_array()
                .is_some_and(|sources| sources.iter().all(serde_json::Value::is_null))
        }));
    }

    #[test]
    fn test_source_map_base() {
        #[track_caller]
        fn run_test(file_name: &str, base: &str, expected: &str) {
            let specifier = ModuleSpecifier::parse(file_name).unwrap();
            let source = r#"{ const foo = "bar"; };"#;
            let allocator = Allocator::default();
            let mut program = parse_program(
                &allocator,
                ParseParams {
                    specifier,
                    text: source.into(),
                    media_type: MediaType::Tsx,
                    capture_tokens: false,
                    maybe_source_type: None,
                    scope_analysis: false,
                },
            )
            .unwrap();
            let emit_options = EmitOptions {
                source_map: SourceMapOption::Separate,
                source_map_base: Some(ModuleSpecifier::parse(base).unwrap()),
                ..Default::default()
            };
            let emit_result = program
                .transpile(
                    &allocator,
                    &TranspileOptions::default(),
                    &TranspileModuleOptions::default(),
                    &emit_options,
                )
                .unwrap();
            assert_eq!(
                &emit_result.text,
                r#"{
  const foo = "bar";
}
;
"#
            );
            assert_source_map_eq(
                &emit_result.source_map.unwrap(),
                &serde_json::json!({
                  "version":3,
                  "sources":[expected],
                  "sourcesContent":["{ const foo = \"bar\"; };"],
                  "names":[],
                })
                .to_string(),
            );
        }

        run_test("https://deno.land/x/mod.tsx", "https://deno.land/x/", "mod.tsx");
        run_test("https://deno.land/x/mod.tsx", "https://deno.land/", "x/mod.tsx");
        run_test(
            "https://deno.land/x/mod.tsx",
            "file:///home/user/",
            "https://deno.land/x/mod.tsx",
        );
        run_test(
            "https://deno.land/x/mod.tsx",
            "https://example.com/",
            "https://deno.land/x/mod.tsx",
        );
        run_test(
            "https://example.com/base/",
            "https://example.com/base/",
            // maybe not ideal, but this is ok
            "https://example.com/base/",
        );
        run_test("file:///example.ts", "file:///", "example.ts");
        run_test("file:///sub_dir/example.ts", "file:///", "sub_dir/example.ts");
        run_test("file:///sub_dir/example.ts", "file:///sub_dir/", "example.ts");
    }

    #[test]
    fn test_source_map_with_file() {
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.tsx").unwrap();
        let source = r#"{ const foo = "bar"; };"#;
        let allocator = Allocator::default();
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::Tsx,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        let emit_options = EmitOptions {
            source_map: SourceMapOption::Separate,
            source_map_file: Some("mod.tsx".to_owned()),
            ..Default::default()
        };
        let emit_result = program
            .transpile(
                &allocator,
                &TranspileOptions::default(),
                &TranspileModuleOptions::default(),
                &emit_options,
            )
            .unwrap();
        assert_eq!(
            &emit_result.text,
            r#"{
  const foo = "bar";
}
;
"#
        );
        assert_source_map_eq(
            emit_result.source_map.as_deref().unwrap(),
            r#"{"version":3,"file":"mod.tsx","sources":["https://deno.land/x/mod.tsx"],"sourcesContent":["{ const foo = \"bar\"; };"],"names":[]}"#,
        );
    }

    #[test]
    fn test_no_source_map() {
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.tsx").unwrap();
        let source = r#"{ const foo = "bar"; };"#;
        let allocator = Allocator::default();
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::Tsx,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        let emit_options = EmitOptions { source_map: SourceMapOption::None, ..Default::default() };
        let emit_result = program
            .transpile(
                &allocator,
                &TranspileOptions::default(),
                &TranspileModuleOptions::default(),
                &emit_options,
            )
            .unwrap();
        assert_eq!(
            &emit_result.text,
            r#"{
  const foo = "bar";
}
;
"#
        );
        assert_eq!(emit_result.source_map, None);
    }

    #[test]
    fn should_not_panic_with_scope_analysis() {
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.ts").unwrap();
        let source = r"
const inspect: () => void = eval();

export function defaultFormatter(record: Record): string {
  for (let i = 0; i < 10; i++) {
    inspect(record);
  }
}

export function formatter(record: Record) {
}
";
        let allocator = Allocator::default();
        let mut program = parse_program(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::TypeScript,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: true,
            },
        )
        .unwrap();

        let emit_result = program.transpile(
            &allocator,
            &TranspileOptions::default(),
            &TranspileModuleOptions::default(),
            &EmitOptions::default(),
        );
        assert!(emit_result.is_ok());
    }

    #[test]
    fn test_source_map_separate() {
        let allocator = Allocator::default();
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.ts").unwrap();
        let source = "const a: number = 5;";
        let mut program = parse_module(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::TypeScript,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        let transpiled_source = program
            .transpile(
                &allocator,
                &TranspileOptions::default(),
                &TranspileModuleOptions::default(),
                &EmitOptions { source_map: SourceMapOption::Separate, ..Default::default() },
            )
            .unwrap();
        assert!(transpiled_source.source_map.is_some());
        assert!(!transpiled_source.text.contains("sourceMappingURL"));
    }

    #[test]
    fn test_transpile_verbatim_module_syntax() {
        let allocator = Allocator::default();
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.ts").unwrap();
        let source = r#"import type foo from "./foo.ts"; import bar from "./bar.ts"; import baz from "./baz.ts"; const b: baz = 1;"#;
        let mut program = parse_module(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::TypeScript,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        let transpile_options =
            TranspileOptions { verbatim_module_syntax: true, ..Default::default() };
        let code = program
            .transpile(
                &allocator,
                &transpile_options,
                &TranspileModuleOptions::default(),
                &EmitOptions::default(),
            )
            .unwrap()
            .text;
        // type-only import should be removed
        assert!(!code.contains("foo"));
        // value imports should be preserved
        assert!(code.contains("bar"));
        assert!(code.contains("baz"));
    }

    #[test]
    fn test_imports_not_used_as_values_options() {
        let allocator = Allocator::default();
        let specifier = ModuleSpecifier::parse("https://deno.land/x/mod.ts").unwrap();
        let source = r#"import { Foo } from "./foo.ts"; type Alias = Foo;"#;
        let mut program = parse_module(
            &allocator,
            ParseParams {
                specifier,
                text: source.into(),
                media_type: MediaType::TypeScript,
                capture_tokens: false,
                maybe_source_type: None,
                scope_analysis: false,
            },
        )
        .unwrap();
        let removed = program
            .transpile(
                &allocator,
                &TranspileOptions {
                    imports_not_used_as_values: ImportsNotUsedAsValues::Remove,
                    ..Default::default()
                },
                &TranspileModuleOptions::default(),
                &EmitOptions { source_map: SourceMapOption::None, ..Default::default() },
            )
            .unwrap()
            .text;
        let preserved = program
            .transpile(
                &allocator,
                &TranspileOptions {
                    imports_not_used_as_values: ImportsNotUsedAsValues::Preserve,
                    ..Default::default()
                },
                &TranspileModuleOptions::default(),
                &EmitOptions { source_map: SourceMapOption::None, ..Default::default() },
            )
            .unwrap()
            .text;
        let errored = program.transpile(
            &allocator,
            &TranspileOptions {
                imports_not_used_as_values: ImportsNotUsedAsValues::Error,
                ..Default::default()
            },
            &TranspileModuleOptions::default(),
            &EmitOptions { source_map: SourceMapOption::None, ..Default::default() },
        );
        assert!(!removed.contains("./foo.ts"));
        assert!(preserved.contains("import \"./foo.ts\";"));
        assert!(errored.unwrap_err().to_string().contains("only used as a type"));
    }
}
