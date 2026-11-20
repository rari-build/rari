use rari_ast::oxc::{
    ast::ast::{
        ArrowFunctionExpression, BindingPattern, BlockStatement, CatchClause, Class, Declaration,
        ExportDefaultDeclarationKind, ForInStatement, ForOfStatement, ForStatement,
        FormalParameters, Function, FunctionBody, IdentifierReference, ImportDeclarationSpecifier,
        Statement, VariableDeclaration, VariableDeclarationKind,
    },
    ast_visit::{Visit, walk},
    syntax::scope::ScopeFlags,
};
use rustc_hash::FxHashSet;

fn class_name(class: &Class<'_>) -> Option<String> {
    class.id.as_ref().map(|id| id.name.to_string())
}

fn function_name(function: &Function<'_>) -> Option<String> {
    function.id.as_ref().map(|id| id.name.to_string())
}

pub fn collect_module_level_idents(stmt: &Statement<'_>) -> FxHashSet<String> {
    let mut idents = FxHashSet::default();

    let declaration = match stmt {
        Statement::ExportDeclaration(export) => Some(&export.declaration),
        _ => stmt.as_declaration(),
    };

    if let Some(declaration) = declaration {
        match declaration {
            Declaration::FunctionDeclaration(function) => {
                idents.extend(function_name(function));
            }
            Declaration::VariableDeclaration(var_decl) => {
                for decl in &var_decl.declarations {
                    collect_module_var_idents(&decl.id, &mut idents);
                }
            }
            Declaration::ClassDeclaration(class) => {
                idents.extend(class_name(class));
            }
            _ => {}
        }
        return idents;
    }

    match stmt {
        Statement::ExportDefaultDeclaration(export) => match &export.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                idents.extend(function_name(function));
            }
            ExportDefaultDeclarationKind::ClassDeclaration(class) => {
                idents.extend(class_name(class));
            }
            _ => {}
        },
        Statement::ImportDeclaration(import) => {
            for spec in import.specifiers.iter().flatten() {
                let local = match spec {
                    ImportDeclarationSpecifier::ImportSpecifier(named) => &named.local,
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(default) => &default.local,
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(ns) => &ns.local,
                };
                idents.insert(local.name.to_string());
            }
        }
        _ => {}
    }

    idents
}

/// Names bound by a module-level variable pattern. This intentionally mirrors
/// the original transform: shorthand properties, nested object values, object
/// rests, and array elements are followed, but defaulted bindings (`x = 1`)
/// and array rests are not.
fn collect_module_var_idents(pattern: &BindingPattern<'_>, idents: &mut FxHashSet<String>) {
    match pattern {
        BindingPattern::BindingIdentifier(ident) => {
            idents.insert(ident.name.to_string());
        }
        BindingPattern::ObjectPattern(obj) => {
            for prop in &obj.properties {
                if prop.shorthand {
                    idents.extend(shorthand_binding_name(&prop.value));
                } else {
                    collect_module_var_idents(&prop.value, idents);
                }
            }
            if let Some(rest) = &obj.rest {
                collect_module_var_idents(&rest.argument, idents);
            }
        }
        BindingPattern::ArrayPattern(arr) => {
            for elem in arr.elements.iter().flatten() {
                collect_module_var_idents(elem, idents);
            }
        }
        BindingPattern::AssignmentPattern(_) => {}
    }
}

/// The binding of a shorthand property: `{ a }` or `{ a = 1 }`.
fn shorthand_binding_name(value: &BindingPattern<'_>) -> Option<String> {
    match value {
        BindingPattern::BindingIdentifier(ident) => Some(ident.name.to_string()),
        BindingPattern::AssignmentPattern(assign) => shorthand_binding_name(&assign.left),
        _ => None,
    }
}

/// Every name bound by a pattern, including defaulted bindings and rests.
fn collect_bound_names(pattern: &BindingPattern<'_>, names: &mut FxHashSet<String>) {
    match pattern {
        BindingPattern::BindingIdentifier(ident) => {
            names.insert(ident.name.to_string());
        }
        BindingPattern::ObjectPattern(obj) => {
            for prop in &obj.properties {
                collect_bound_names(&prop.value, names);
            }
            if let Some(rest) = &obj.rest {
                collect_bound_names(&rest.argument, names);
            }
        }
        BindingPattern::ArrayPattern(arr) => {
            for elem in arr.elements.iter().flatten() {
                collect_bound_names(elem, names);
            }
            if let Some(rest) = &arr.rest {
                collect_bound_names(&rest.argument, names);
            }
        }
        BindingPattern::AssignmentPattern(assign) => collect_bound_names(&assign.left, names),
    }
}

fn collect_param_names(params: &FormalParameters<'_>, names: &mut FxHashSet<String>) {
    for param in &params.items {
        collect_bound_names(&param.pattern, names);
    }
    if let Some(rest) = &params.rest {
        collect_bound_names(&rest.rest.argument, names);
    }
}

struct ClosureCollector<'m> {
    module_idents: &'m FxHashSet<String>,
    fn_params: &'m FxHashSet<String>,
    function_scope_stack: Vec<FxHashSet<String>>,
    scope_stack: Vec<FxHashSet<String>>,
    found: Vec<String>,
    seen: FxHashSet<String>,
}

impl ClosureCollector<'_> {
    fn is_shadowed(&self, name: &str) -> bool {
        self.function_scope_stack.iter().rev().any(|scope| scope.contains(name))
            || self.scope_stack.iter().rev().any(|scope| scope.contains(name))
    }

    fn with_block_scope(&mut self, initial: FxHashSet<String>, visit: impl FnOnce(&mut Self)) {
        self.scope_stack.push(initial);
        visit(self);
        self.scope_stack.pop();
    }

    fn with_function_scope(&mut self, params: FxHashSet<String>, visit: impl FnOnce(&mut Self)) {
        self.function_scope_stack.push(params);
        visit(self);
        self.function_scope_stack.pop();
    }
}

impl<'a> Visit<'a> for ClosureCollector<'_> {
    fn visit_identifier_reference(&mut self, ident: &IdentifierReference<'a>) {
        let name = ident.name.as_str();
        if self.fn_params.contains(name) || self.is_shadowed(name) {
            return;
        }
        if self.module_idents.contains(name) && self.seen.insert(name.to_string()) {
            self.found.push(name.to_string());
        }
    }

    fn visit_statement(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::FunctionDeclaration(function) => {
                if let (Some(name), Some(scope)) =
                    (function_name(function), self.function_scope_stack.last_mut())
                {
                    scope.insert(name);
                }
            }
            Statement::ClassDeclaration(class) => {
                if let (Some(name), Some(scope)) = (class_name(class), self.scope_stack.last_mut())
                {
                    scope.insert(name);
                }
            }
            _ => {}
        }
        walk::walk_statement(self, stmt);
    }

    fn visit_variable_declaration(&mut self, var_decl: &VariableDeclaration<'a>) {
        let target_scope = if var_decl.kind == VariableDeclarationKind::Var {
            self.function_scope_stack.last_mut()
        } else {
            self.scope_stack.last_mut()
        };
        if let Some(scope) = target_scope {
            for decl in &var_decl.declarations {
                collect_bound_names(&decl.id, scope);
            }
        }
        walk::walk_variable_declaration(self, var_decl);
    }

    fn visit_block_statement(&mut self, block: &BlockStatement<'a>) {
        self.with_block_scope(FxHashSet::default(), |this| {
            walk::walk_block_statement(this, block);
        });
    }

    fn visit_function_body(&mut self, body: &FunctionBody<'a>) {
        // A nested function body is its own block scope (SWC modelled it as a
        // block statement), so its `let`/`const`/`class` bindings must not
        // leak into the enclosing scope.
        self.with_block_scope(FxHashSet::default(), |this| {
            walk::walk_function_body(this, body);
        });
    }

    fn visit_function(&mut self, function: &Function<'a>, flags: ScopeFlags) {
        let mut params = FxHashSet::default();
        collect_param_names(&function.params, &mut params);
        self.with_function_scope(params, |this| walk::walk_function(this, function, flags));
    }

    fn visit_arrow_function_expression(&mut self, arrow: &ArrowFunctionExpression<'a>) {
        let mut params = FxHashSet::default();
        collect_param_names(&arrow.params, &mut params);
        self.with_function_scope(params, |this| {
            walk::walk_arrow_function_expression(this, arrow);
        });
    }

    fn visit_catch_clause(&mut self, clause: &CatchClause<'a>) {
        let mut names = FxHashSet::default();
        if let Some(param) = &clause.param {
            collect_bound_names(&param.pattern, &mut names);
        }
        self.with_block_scope(names, |this| walk::walk_catch_clause(this, clause));
    }

    fn visit_for_statement(&mut self, stmt: &ForStatement<'a>) {
        self.with_block_scope(FxHashSet::default(), |this| walk::walk_for_statement(this, stmt));
    }

    fn visit_for_in_statement(&mut self, stmt: &ForInStatement<'a>) {
        self.with_block_scope(FxHashSet::default(), |this| walk::walk_for_in_statement(this, stmt));
    }

    fn visit_for_of_statement(&mut self, stmt: &ForOfStatement<'a>) {
        self.with_block_scope(FxHashSet::default(), |this| walk::walk_for_of_statement(this, stmt));
    }
}

/// Module-level names referenced from `body` (in first-use order) that are not
/// shadowed by a parameter or a local binding.
#[expect(clippy::implicit_hasher)]
pub fn collect_closure_idents(
    body: &FunctionBody<'_>,
    module_idents: &FxHashSet<String>,
    fn_params: &FxHashSet<String>,
    fn_ident: Option<&str>,
) -> Vec<String> {
    let mut outer_scope = fn_params.clone();
    if let Some(name) = fn_ident {
        outer_scope.insert(name.to_string());
    }

    let mut collector = ClosureCollector {
        module_idents,
        fn_params,
        function_scope_stack: vec![FxHashSet::default()],
        scope_stack: vec![outer_scope],
        found: Vec::new(),
        seen: FxHashSet::default(),
    };

    for stmt in &body.statements {
        collector.visit_statement(stmt);
    }

    collector.found
}

pub fn collect_fn_params(function: &Function<'_>) -> FxHashSet<String> {
    let mut params = FxHashSet::default();
    collect_param_names(&function.params, &mut params);
    params
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "Test helpers fail fast when a snippet parses unexpectedly"
)]
mod tests {
    use rari_ast::oxc::{
        allocator::Allocator,
        ast::ast::{Function, Statement},
        parser::Parser,
        span::SourceType,
    };

    use super::*;

    fn module_idents(source: &str) -> FxHashSet<String> {
        let allocator = Allocator::default();
        let ret = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let stmt = ret.program.body.first().expect("expected a statement");
        collect_module_level_idents(stmt)
    }

    fn fn_params(source: &str) -> FxHashSet<String> {
        let allocator = Allocator::default();
        let ret = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let Some(Statement::FunctionDeclaration(function)) = ret.program.body.first() else {
            panic!("expected a function declaration");
        };
        let function: &Function<'_> = function;
        collect_fn_params(function)
    }

    fn single(idents: FxHashSet<String>) -> String {
        assert_eq!(idents.len(), 1);
        idents.into_iter().next().expect("expected one identifier")
    }

    #[test]
    fn test_collect_fn_params_simple() {
        assert_eq!(fn_params("function f(a, b) {}").len(), 2);
    }

    #[test]
    fn test_collect_module_idents_import() {
        assert_eq!(single(module_idents("import React from 'react'")), "React");
    }

    #[test]
    fn test_collect_module_idents_fn_decl() {
        assert_eq!(module_idents("function myFunction() {}").len(), 1);
    }

    #[test]
    fn test_collect_module_idents_export_default_fn() {
        assert_eq!(single(module_idents("export default function myFn() {}")), "myFn");
    }

    #[test]
    fn test_collect_module_idents_class_decl() {
        assert_eq!(single(module_idents("class Foo {}")), "Foo");
    }

    #[test]
    fn test_collect_module_idents_export_decl_class() {
        assert_eq!(single(module_idents("export class Bar {}")), "Bar");
    }

    #[test]
    fn test_collect_module_idents_export_default_class() {
        assert_eq!(single(module_idents("export default class Baz {}")), "Baz");
    }

    #[test]
    fn test_collect_fn_params_with_patterns() {
        let params = fn_params("function f({ x }, ...rest) {}");
        assert_eq!(params.len(), 2);
        assert!(params.contains("x") && params.contains("rest"));
    }

    #[test]
    fn test_nested_function_body_bindings_do_not_leak() {
        let allocator = Allocator::default();
        let source = "const shared = 1\nasync function f() { function inner() { const shared = 2 } return shared }";
        let ret = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let mut module = FxHashSet::default();
        for stmt in &ret.program.body {
            module.extend(collect_module_level_idents(stmt));
        }
        let Some(Statement::FunctionDeclaration(function)) = ret.program.body.get(1) else {
            panic!("expected a function declaration");
        };
        let body = function.body.as_ref().expect("expected a body");
        let found = collect_closure_idents(body, &module, &FxHashSet::default(), Some("f"));
        assert_eq!(found, vec!["shared".to_string()]);
    }
}
