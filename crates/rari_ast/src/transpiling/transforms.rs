// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

#![expect(clippy::unwrap_used, reason = "Ported code not yet updated to the workspace lint rules")]

use oxc::{
    allocator::{Allocator, ArenaVec, CloneIn, GetAllocator},
    ast::{ast::*, builder::AstBuilder},
    ast_visit::{VisitMut, walk_mut},
    semantic::Scoping,
    span::{GetSpan, SPAN},
    syntax::operator::AssignmentOperator,
};

use super::{TranspileOptions, jsx_factory_import_names};
use crate::JSStrExt;

/// Transforms import declarations to variable declarations
/// with a dynamic import. This is used to provide import
/// declaration support in script contexts such as the Deno REPL.
pub struct ImportDeclsToVarDecls<'a> {
    ast: AstBuilder<'a>,
}

impl<'a> ImportDeclsToVarDecls<'a> {
    pub fn new(allocator: &'a Allocator) -> Self {
        Self { ast: AstBuilder::new(allocator) }
    }
}

impl<'a> VisitMut<'a> for ImportDeclsToVarDecls<'a> {
    fn visit_statements(&mut self, stmts: &mut oxc::allocator::Vec<'a, Statement<'a>>) {
        let mut new_stmts = ArenaVec::with_capacity_in(stmts.len(), &self.ast);
        for stmt in stmts.drain(..) {
            match stmt {
                Statement::ImportDeclaration(import_decl) => {
                    if import_decl.import_kind.is_type() {
                        // type-only import - remove entirely
                        continue;
                    }

                    let source_value = import_decl.source.value.to_str_lossy();
                    let with_clause = import_decl.with_clause.as_deref();

                    // Side-effect only import: `import "./mod.ts"` -> `await import("./mod.ts")`
                    if import_decl.specifiers.as_ref().is_none_or(|s| s.is_empty()) {
                        let import_expr = self.create_await_import_expr(&source_value, with_clause);
                        new_stmts.push(Statement::new_expression_statement(
                            import_decl.span,
                            import_expr,
                            &self.ast,
                        ));
                        continue;
                    }

                    let specifiers = import_decl.specifiers.as_ref().unwrap();

                    // Collect named/default imports for destructuring
                    let mut props = ArenaVec::new_in(&self.ast);
                    let mut namespace_name: Option<&str> = None;

                    for specifier in specifiers {
                        match specifier {
                            ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                                let key =
                                    PropertyKey::new_static_identifier(SPAN, "default", &self.ast);
                                let value = BindingPattern::new_binding_identifier(
                                    SPAN,
                                    s.local.name.as_str(),
                                    &self.ast,
                                );
                                props.push(BindingProperty::new(
                                    SPAN, key, value, false, false, &self.ast,
                                ));
                            }
                            ImportDeclarationSpecifier::ImportSpecifier(s) => {
                                let local_name = s.local.name.as_str();
                                let shorthand = s.imported.name() == local_name;
                                let key = import_property_key(&s.imported, &self.ast);
                                let value = BindingPattern::new_binding_identifier(
                                    SPAN, local_name, &self.ast,
                                );
                                props.push(BindingProperty::new(
                                    SPAN, key, value, shorthand, false, &self.ast,
                                ));
                            }
                            ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                                namespace_name = Some(s.local.name.as_str());
                            }
                        }
                    }

                    let import_expr = self.create_await_import_expr(&source_value, with_clause);

                    let mut declarators = ArenaVec::new_in(&self.ast);

                    if !props.is_empty() {
                        let pattern =
                            BindingPattern::new_object_pattern(SPAN, props, None, &self.ast);
                        declarators.push(VariableDeclarator::new(
                            SPAN,
                            pattern,
                            None,
                            Some(import_expr.clone_in(self.ast.allocator())),
                            false,
                            &self.ast,
                        ));
                    }

                    if let Some(ns_name) = namespace_name {
                        let pattern =
                            BindingPattern::new_binding_identifier(SPAN, ns_name, &self.ast);
                        declarators.push(VariableDeclarator::new(
                            SPAN,
                            pattern,
                            None,
                            Some(import_expr),
                            false,
                            &self.ast,
                        ));
                    }

                    if !declarators.is_empty() {
                        let var_decl = Statement::from(Declaration::new_variable_declaration(
                            import_decl.span,
                            VariableDeclarationKind::Const,
                            declarators,
                            false,
                            &self.ast,
                        ));
                        new_stmts.push(var_decl);
                    }
                }
                _ => {
                    new_stmts.push(stmt);
                }
            }
        }
        *stmts = new_stmts;

        // Visit children
        walk_mut::walk_statements(self, stmts);
    }
}

impl<'a> ImportDeclsToVarDecls<'a> {
    fn create_await_import_expr(
        &self,
        source: &str,
        with_clause: Option<&WithClause<'a>>,
    ) -> Expression<'a> {
        create_await_import_expr(&self.ast, source, with_clause)
    }
}

fn import_property_key<'a>(
    imported: &ModuleExportName<'a>,
    ast: &AstBuilder<'a>,
) -> PropertyKey<'a> {
    match imported {
        ModuleExportName::IdentifierName(name) => {
            PropertyKey::new_static_identifier(SPAN, name.name, ast)
        }
        ModuleExportName::StringLiteral(name) => {
            PropertyKey::new_string_literal(SPAN, name.value, None, ast)
        }
        ModuleExportName::IdentifierReference(_) => unreachable!(),
    }
}

pub fn preserve_type_only_imports(
    program: &mut Program<'_>,
    scoping: &Scoping,
    options: &TranspileOptions,
) {
    let jsx_factory_names = jsx_factory_import_names(program, options);
    for stmt in &mut program.body {
        let Statement::ImportDeclaration(import_decl) = stmt else {
            continue;
        };
        if import_decl.import_kind.is_type() {
            continue;
        }
        let Some(specifiers) = import_decl.specifiers.as_ref() else {
            continue;
        };

        let mut has_type_only_import = false;
        let mut has_value_import = false;
        for specifier in specifiers {
            let explicit_type = matches!(
                specifier,
                ImportDeclarationSpecifier::ImportSpecifier(import_specifier)
                    if import_specifier.import_kind.is_type()
            );
            let mut has_type_reference = explicit_type;
            let mut has_value_reference = false;
            for reference in scoping.get_resolved_references(specifier.local().symbol_id()) {
                if reference.is_type() {
                    has_type_reference = true;
                } else {
                    has_value_reference = true;
                }
            }
            if !explicit_type {
                has_value_reference |= jsx_factory_names
                    .iter()
                    .flatten()
                    .any(|name| *name == specifier.local().name.as_str());
            }
            has_type_only_import |= has_type_reference;
            has_value_import |= has_value_reference;
        }

        if has_type_only_import && !has_value_import && !options.verbatim_module_syntax {
            import_decl.specifiers = None;
        }
    }
}

pub struct SortClassElementsBySourceOrder;

impl<'a> VisitMut<'a> for SortClassElementsBySourceOrder {
    fn visit_class(&mut self, class: &mut Class<'a>) {
        let body = &mut class.body.body;
        if body.windows(2).any(|elements| elements[0].span().start > elements[1].span().start) {
            body.sort_by_key(|element| element.span().start);
        }
        walk_mut::walk_class(self, class);
    }
}

/// Strips export declarations and exports on named exports so the
/// code can be used in script contexts. This is useful for example
/// in the Deno REPL.
pub struct StripExports<'a> {
    ast: AstBuilder<'a>,
}

impl<'a> StripExports<'a> {
    pub fn new(allocator: &'a Allocator) -> Self {
        Self { ast: AstBuilder::new(allocator) }
    }
}

impl<'a> VisitMut<'a> for StripExports<'a> {
    fn visit_statements(&mut self, stmts: &mut oxc::allocator::Vec<'a, Statement<'a>>) {
        let mut new_stmts = ArenaVec::with_capacity_in(stmts.len(), &self.ast);
        for stmt in stmts.drain(..) {
            match stmt {
                Statement::ExportAllDeclaration(export_all) => {
                    let source_value = export_all.source.value.to_str_lossy();
                    let import_expr = self
                        .create_await_import_expr(&source_value, export_all.with_clause.as_deref());
                    new_stmts.push(Statement::new_expression_statement(
                        SPAN,
                        import_expr,
                        &self.ast,
                    ));
                }
                Statement::ExportFromDeclaration(export_from) => {
                    let source_value = export_from.source.value.to_str_lossy();
                    let import_expr = self.create_await_import_expr(
                        &source_value,
                        export_from.with_clause.as_deref(),
                    );
                    new_stmts.push(Statement::new_expression_statement(
                        SPAN,
                        import_expr,
                        &self.ast,
                    ));
                }
                Statement::ExportDeclaration(export_decl) => {
                    new_stmts.push(Statement::from(export_decl.unbox().declaration));
                }
                Statement::ExportNamedDeclaration(_) => {
                    // `export { test }` -> remove entirely
                }
                Statement::ExportDefaultDeclaration(export_default) => {
                    match export_default.unbox().declaration {
                        ExportDefaultDeclarationKind::FunctionDeclaration(func) => {
                            if func.id.is_some() {
                                new_stmts.push(Statement::FunctionDeclaration(func));
                            }
                        }
                        ExportDefaultDeclarationKind::ClassDeclaration(class) => {
                            if class.id.is_some() {
                                new_stmts.push(Statement::ClassDeclaration(class));
                            }
                        }
                        ExportDefaultDeclarationKind::TSInterfaceDeclaration(_) => {
                            // type only - discard
                        }
                        kind => {
                            // `export default <expr>` -> `<expr>;`
                            // ExportDefaultDeclarationKind inherits Expression variants,
                            // so all remaining variants are expressions.
                            if kind.is_expression() {
                                let expr = kind.into_expression();
                                new_stmts.push(Statement::new_expression_statement(
                                    SPAN, expr, &self.ast,
                                ));
                            }
                        }
                    }
                }
                _ => {
                    new_stmts.push(stmt);
                }
            }
        }
        *stmts = new_stmts;
        // Don't recurse into nested scopes - only strip top-level exports
    }
}

impl<'a> StripExports<'a> {
    fn create_await_import_expr(
        &self,
        source: &str,
        with_clause: Option<&WithClause<'a>>,
    ) -> Expression<'a> {
        create_await_import_expr(&self.ast, source, with_clause)
    }
}

/// Creates `await import("<source>")`, carrying over any import attributes as
/// the dynamic import's `{ with: { ... } }` options object.
fn create_await_import_expr<'a>(
    ast: &AstBuilder<'a>,
    source: &str,
    with_clause: Option<&WithClause<'a>>,
) -> Expression<'a> {
    let source_in_alloc =
        oxc::allocator::StringBuilder::from_str_in(source, ast.allocator()).into_str();
    let import_arg = Expression::new_string_literal(SPAN, source_in_alloc, None, ast);
    let options =
        with_clause.filter(|with_clause| !with_clause.with_entries.is_empty()).map(|with_clause| {
            let mut attributes = ArenaVec::new_in(ast);
            for attribute in &with_clause.with_entries {
                let key = match &attribute.key {
                    ImportAttributeKey::Identifier(ident) => {
                        PropertyKey::new_static_identifier(SPAN, ident.name, ast)
                    }
                    ImportAttributeKey::StringLiteral(str_) => {
                        PropertyKey::new_string_literal(SPAN, str_.value, None, ast)
                    }
                };
                attributes.push(ObjectPropertyKind::new_object_property(
                    SPAN,
                    PropertyKind::Init,
                    key,
                    Expression::new_string_literal(SPAN, attribute.value.value, None, ast),
                    false,
                    false,
                    false,
                    ast,
                ));
            }
            let mut properties = ArenaVec::new_in(ast);
            properties.push(ObjectPropertyKind::new_object_property(
                SPAN,
                PropertyKind::Init,
                PropertyKey::new_static_identifier(SPAN, "with", ast),
                Expression::new_object_expression(SPAN, attributes, ast),
                false,
                false,
                false,
                ast,
            ));
            Expression::new_object_expression(SPAN, properties, ast)
        });
    let import_call = Expression::new_import_expression(SPAN, import_arg, options, None, ast);
    Expression::new_await_expression(SPAN, import_call, ast)
}

/// Converts the module declarations that are allowed in a CommonJS script
/// (`export import x = ...` and `export = ...`) so that the program can be
/// transformed as a script:
///
/// - `export import x = require("./x")` -> `import x = require("./x"); exports.x = x;`
///   (the import equals is then lowered to `const x = require("./x")` by the
///   TypeScript transform)
///
/// `export = x` is lowered to `module.exports = x` by the TypeScript transform
/// when the program is a script, so it is left alone.
pub struct CjsScriptExports<'a> {
    ast: AstBuilder<'a>,
}

impl<'a> CjsScriptExports<'a> {
    pub fn new(allocator: &'a Allocator) -> Self {
        Self { ast: AstBuilder::new(allocator) }
    }

    pub fn transform(&self, program: &mut Program<'a>) {
        let stmts = &mut program.body;
        let mut new_stmts = ArenaVec::with_capacity_in(stmts.len(), &self.ast);
        for stmt in stmts.drain(..) {
            match stmt {
                Statement::ExportDeclaration(export_decl)
                    if matches!(
                        export_decl.declaration,
                        Declaration::TSImportEqualsDeclaration(_)
                    ) =>
                {
                    let Declaration::TSImportEqualsDeclaration(import_equals) =
                        export_decl.unbox().declaration
                    else {
                        unreachable!()
                    };
                    let is_type = import_equals.import_kind.is_type();
                    let name = import_equals.id.name;
                    new_stmts.push(Statement::TSImportEqualsDeclaration(import_equals));
                    if !is_type {
                        // exports.name = name;
                        let exports_member = Expression::new_static_member_expression(
                            SPAN,
                            Expression::new_identifier(SPAN, "exports", &self.ast),
                            IdentifierName::new(SPAN, name, &self.ast),
                            false,
                            &self.ast,
                        );
                        let Expression::StaticMemberExpression(member) = exports_member else {
                            unreachable!()
                        };
                        let assignment = Expression::new_assignment_expression(
                            SPAN,
                            AssignmentOperator::Assign,
                            AssignmentTarget::StaticMemberExpression(member),
                            Expression::new_identifier(SPAN, name, &self.ast),
                            &self.ast,
                        );
                        new_stmts
                            .push(Statement::new_expression_statement(SPAN, assignment, &self.ast));
                    }
                }
                stmt => new_stmts.push(stmt),
            }
        }
        *stmts = new_stmts;
        program.source_type = program.source_type.with_script(true);
    }
}

#[cfg(test)]
mod test {
    use oxc::{allocator::Allocator, codegen::Codegen, parser::Parser, span::SourceType};

    use super::*;

    #[track_caller]
    fn test_transform_import(src: &str, expected_output: &str) {
        let allocator = Allocator::default();
        let mut transform = ImportDeclsToVarDecls::new(&allocator);
        run_transform(&allocator, &mut transform, src, expected_output);
    }

    #[track_caller]
    fn test_transform_strip(src: &str, expected_output: &str) {
        let allocator = Allocator::default();
        let mut transform = StripExports::new(&allocator);
        run_transform(&allocator, &mut transform, src, expected_output);
    }

    #[track_caller]
    fn run_transform<'a>(
        allocator: &'a Allocator,
        transform: &mut impl VisitMut<'a>,
        src: &str,
        expected_output: &str,
    ) {
        let source_type = SourceType::tsx();
        let src_in_arena = allocator.alloc_str(src);
        let mut ret = Parser::new(allocator, src_in_arena, source_type).parse();
        transform.visit_program(&mut ret.program);
        let output = Codegen::new().build(&ret.program).code;
        let output = output.trim_end();
        assert_eq!(output, expected_output);
    }

    // === ImportDeclsToVarDecls tests ===

    #[test]
    fn test_downlevel_imports_type_only() {
        test_transform_import(r#"import type { test } from "./mod.ts";"#, "");
    }

    #[test]
    fn test_downlevel_imports_specifier_only() {
        test_transform_import(r#"import "./mod.ts";"#, r#"await import("./mod.ts");"#);
        test_transform_import(r#"import {} from "./mod.ts";"#, r#"await import("./mod.ts");"#);
    }

    #[test]
    fn test_downlevel_imports_default() {
        test_transform_import(
            r#"import mod from "./mod.ts";"#,
            r#"const { default: mod } = await import("./mod.ts");"#,
        );
    }

    #[test]
    fn test_downlevel_imports_named() {
        test_transform_import(
            r#"import { A } from "./mod.ts";"#,
            r#"const { A } = await import("./mod.ts");"#,
        );
        test_transform_import(
            r#"import { A, B, C } from "./mod.ts";"#,
            r#"const { A, B, C } = await import("./mod.ts");"#,
        );
        test_transform_import(
            r#"import { A as LocalA, B, C as LocalC } from "./mod.ts";"#,
            r#"const { A: LocalA, B, C: LocalC } = await import("./mod.ts");"#,
        );
        test_transform_import(
            r#"import { "foo-bar" as value } from "./mod.ts";"#,
            r#"const { "foo-bar": value } = await import("./mod.ts");"#,
        );
    }

    #[test]
    fn test_downlevel_imports_namespace() {
        test_transform_import(
            r#"import * as mod from "./mod.ts";"#,
            r#"const mod = await import("./mod.ts");"#,
        );
    }

    #[test]
    fn test_downlevel_imports_mixed() {
        test_transform_import(
            r#"import myDefault, { A, B as LocalB } from "./mod.ts";"#,
            r#"const { default: myDefault, A, B: LocalB } = await import("./mod.ts");"#,
        );
        test_transform_import(
            r#"import myDefault, * as mod from "./mod.ts";"#,
            r#"const { default: myDefault } = await import("./mod.ts"), mod = await import("./mod.ts");"#,
        );
    }

    #[test]
    fn test_downlevel_imports_assertions() {
        test_transform_import(
            r#"import data from "./mod.json" with { type: "json" };"#,
            r#"const { default: data } = await import("./mod.json", { with: { type: "json" } });"#,
        );
    }

    // === StripExports tests ===

    #[test]
    fn test_strip_exports_export_all() {
        test_transform_strip(r#"export * from "./test.ts";"#, r#"await import("./test.ts");"#);
    }

    #[test]
    fn test_strip_exports_export_named() {
        test_transform_strip(
            r#"export { test } from "./test.ts";"#,
            r#"await import("./test.ts");"#,
        );
        test_transform_strip(r"export { test };", "");
    }

    #[test]
    fn test_strip_exports_assertions() {
        test_transform_strip(
            r#"export { default as data } from "./mod.json" with { type: "json" };"#,
            r#"await import("./mod.json", { with: { type: "json" } });"#,
        );
    }

    #[test]
    fn test_strip_exports_export_all_assertions() {
        test_transform_strip(
            r#"export * from "./mod.json" with { type: "json" };"#,
            r#"await import("./mod.json", { with: { type: "json" } });"#,
        );
    }

    #[test]
    fn test_strip_exports_export_default_expr() {
        test_transform_strip("export default 5;", "5;");
    }

    #[test]
    fn test_strip_exports_export_default_decl_name() {
        test_transform_strip("export default class Test {}", "class Test {}");
        test_transform_strip("export default function test() {}", "function test() {}");
    }

    #[test]
    fn test_strip_exports_export_default_decl_no_name() {
        test_transform_strip("export default class {}", "");
        test_transform_strip("export default function() {}", "");
    }

    #[test]
    fn test_strip_exports_export_named_decls() {
        test_transform_strip("export class Test {}", "class Test {}");
        test_transform_strip("export function test() {}", "function test() {}");
        test_transform_strip("export enum Test {}", "enum Test {}");
        test_transform_strip("export namespace Test {}", "namespace Test {}");
    }

    #[test]
    fn test_strip_exports_not_in_namespace() {
        test_transform_strip(
            "namespace Test { export class Test {} }",
            "namespace Test {\n\texport class Test {}\n}",
        );
    }
}
