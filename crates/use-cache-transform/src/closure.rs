use rustc_hash::FxHashSet as HashSet;

use deno_ast::swc::ast::*;
use deno_ast::swc::ecma_visit::{Visit, VisitWith};

/// Collect identifiers declared at the module level (imports, functions, vars)
pub fn collect_module_level_idents(item: &ModuleItem) -> HashSet<Id> {
    let mut idents = HashSet::default();

    match item {
        ModuleItem::Stmt(Stmt::Decl(Decl::Fn(FnDecl { ident, .. }))) => {
            idents.insert(ident.to_id());
        }
        ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
            decl: Decl::Fn(FnDecl { ident, .. }),
            ..
        })) => {
            idents.insert(ident.to_id());
        }
        ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) => {
            for decl in &var_decl.decls {
                collect_var_decl_idents(&decl.name, &mut idents);
            }
        }
        ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
            decl: Decl::Var(var_decl),
            ..
        })) => {
            for decl in &var_decl.decls {
                collect_var_decl_idents(&decl.name, &mut idents);
            }
        }
        ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) => {
            for spec in &import_decl.specifiers {
                match spec {
                    ImportSpecifier::Named(named) => {
                        idents.insert(named.local.to_id());
                    }
                    ImportSpecifier::Default(default) => {
                        idents.insert(default.local.to_id());
                    }
                    ImportSpecifier::Namespace(ns) => {
                        idents.insert(ns.local.to_id());
                    }
                }
            }
        }
        _ => {}
    }

    idents
}

fn collect_var_decl_idents(pattern: &Pat, idents: &mut HashSet<Id>) {
    match pattern {
        Pat::Ident(ident) => {
            idents.insert(ident.to_id());
        }
        Pat::Object(obj) => {
            for prop in &obj.props {
                match prop {
                    ObjectPatProp::Assign(assign) => {
                        idents.insert(assign.key.to_id());
                    }
                    ObjectPatProp::KeyValue(kv) => {
                        collect_var_decl_idents(&kv.value, idents);
                    }
                    ObjectPatProp::Rest(rest) => {
                        collect_var_decl_idents(&rest.arg, idents);
                    }
                }
            }
        }
        Pat::Array(arr) => {
            for elem in arr.elems.iter().flatten() {
                collect_var_decl_idents(elem, idents);
            }
        }
        _ => {}
    }
}

/// Walk a block/pattern to collect locally-declared identifiers into a scope set.
fn collect_bindings_from_pat(pat: &Pat, scope: &mut HashSet<Id>) {
    match pat {
        Pat::Ident(bi) => {
            scope.insert(bi.id.to_id());
        }
        Pat::Object(obj) => {
            for prop in &obj.props {
                match prop {
                    ObjectPatProp::Assign(a) => {
                        scope.insert(a.key.to_id());
                    }
                    ObjectPatProp::KeyValue(kv) => collect_bindings_from_pat(&kv.value, scope),
                    ObjectPatProp::Rest(r) => collect_bindings_from_pat(&r.arg, scope),
                }
            }
        }
        Pat::Array(arr) => {
            for elem in arr.elems.iter().flatten() {
                collect_bindings_from_pat(elem, scope);
            }
        }
        Pat::Rest(r) => collect_bindings_from_pat(&r.arg, scope),
        Pat::Assign(a) => collect_bindings_from_pat(&a.left, scope),
        _ => {}
    }
}

/// Collect all identifier references in a function body that reference outer scope.
/// `module_idents` = all identifiers declared at module level
/// `fn_params` = function parameter identifiers
///
/// Uses scope tracking: walks the body tracking VarDecl, Function params, and block-scoped
/// declarations, pushing/popping scopes so that identifiers shadowed by local bindings are
/// not treated as module captures.
pub fn collect_closure_idents(
    body: &BlockStmt,
    module_idents: &HashSet<Id>,
    fn_params: &HashSet<Id>,
    fn_ident: Option<&Ident>,
) -> Vec<String> {
    struct ClosureCollector {
        module_idents: HashSet<Id>,
        fn_params: HashSet<Id>,
        function_scope_stack: Vec<HashSet<Id>>,
        scope_stack: Vec<HashSet<Id>>,
        found: Vec<String>,
    }

    impl ClosureCollector {
        fn is_shadowed(&self, id: &Id) -> bool {
            for scope in self.function_scope_stack.iter().rev() {
                if scope.contains(id) {
                    return true;
                }
            }
            for scope in self.scope_stack.iter().rev() {
                if scope.contains(id) {
                    return true;
                }
            }
            false
        }

        fn collect_var_decl_bindings(&mut self, var_decl: &VarDecl) {
            let target_scope = if var_decl.kind == VarDeclKind::Var {
                self.function_scope_stack.last_mut()
            } else {
                self.scope_stack.last_mut()
            };

            if let Some(scope) = target_scope {
                for decl in &var_decl.decls {
                    collect_bindings_from_pat(&decl.name, scope);
                }
            }
        }

        fn collect_decl_binding(&mut self, decl: &Decl) {
            match decl {
                Decl::Fn(fn_decl) => {
                    if let Some(scope) = self.function_scope_stack.last_mut() {
                        scope.insert(fn_decl.ident.to_id());
                    }
                }
                Decl::Class(class_decl) => {
                    if let Some(scope) = self.scope_stack.last_mut() {
                        scope.insert(class_decl.ident.to_id());
                    }
                }
                _ => {}
            }
        }
    }

    impl Visit for ClosureCollector {
        fn visit_ident(&mut self, ident: &Ident) {
            let id = ident.to_id();
            if self.fn_params.contains(&id) || self.is_shadowed(&id) {
                return;
            }
            if self.module_idents.contains(&id) {
                let name = ident.sym.to_string();
                if !self.found.contains(&name) {
                    self.found.push(name);
                }
            }
        }

        fn visit_block_stmt(&mut self, block: &BlockStmt) {
            self.scope_stack.push(HashSet::default());
            block.visit_children_with(self);
            self.scope_stack.pop();
        }

        fn visit_stmt(&mut self, stmt: &Stmt) {
            if let Stmt::Decl(decl) = stmt {
                self.collect_decl_binding(decl);
            }
            stmt.visit_children_with(self);
        }

        fn visit_var_decl(&mut self, var_decl: &VarDecl) {
            self.collect_var_decl_bindings(var_decl);
            var_decl.visit_children_with(self);
        }

        fn visit_function(&mut self, f: &Function) {
            let mut new_function_scope = HashSet::default();
            for param in &f.params {
                collect_bindings_from_pat(&param.pat, &mut new_function_scope);
            }
            self.function_scope_stack.push(new_function_scope);
            f.visit_children_with(self);
            self.function_scope_stack.pop();
        }

        fn visit_arrow_expr(&mut self, a: &ArrowExpr) {
            let mut new_function_scope = HashSet::default();
            for param in &a.params {
                collect_bindings_from_pat(param, &mut new_function_scope);
            }
            self.function_scope_stack.push(new_function_scope);
            a.visit_children_with(self);
            self.function_scope_stack.pop();
        }

        fn visit_catch_clause(&mut self, clause: &CatchClause) {
            let mut new_scope = HashSet::default();
            if let Some(param) = &clause.param {
                collect_bindings_from_pat(param, &mut new_scope);
            }
            self.scope_stack.push(new_scope);
            clause.visit_children_with(self);
            self.scope_stack.pop();
        }

        fn visit_for_stmt(&mut self, stmt: &ForStmt) {
            self.scope_stack.push(HashSet::default());
            stmt.visit_children_with(self);
            self.scope_stack.pop();
        }

        fn visit_for_in_stmt(&mut self, stmt: &ForInStmt) {
            self.scope_stack.push(HashSet::default());
            stmt.visit_children_with(self);
            self.scope_stack.pop();
        }

        fn visit_for_of_stmt(&mut self, stmt: &ForOfStmt) {
            self.scope_stack.push(HashSet::default());
            stmt.visit_children_with(self);
            self.scope_stack.pop();
        }
    }

    let mut outer_scope = fn_params.clone();
    if let Some(ident) = fn_ident {
        outer_scope.insert(ident.to_id());
    }

    let mut collector = ClosureCollector {
        module_idents: module_idents.clone(),
        fn_params: fn_params.clone(),
        function_scope_stack: vec![HashSet::default()],
        scope_stack: vec![outer_scope],
        found: Vec::new(),
    };

    for stmt in &body.stmts {
        collector.visit_stmt(stmt);
    }

    collector.found
}

pub fn collect_fn_params(f: &Function) -> HashSet<Id> {
    let mut params = HashSet::default();
    for param in &f.params {
        collect_pat_idents(&param.pat, &mut params);
    }
    params
}

fn collect_pat_idents(pattern: &Pat, idents: &mut HashSet<Id>) {
    match pattern {
        Pat::Ident(ident) => {
            idents.insert(ident.to_id());
        }
        Pat::Object(obj) => {
            for prop in &obj.props {
                match prop {
                    ObjectPatProp::Assign(assign) => {
                        idents.insert(assign.key.to_id());
                    }
                    ObjectPatProp::KeyValue(kv) => {
                        collect_pat_idents(&kv.value, idents);
                    }
                    ObjectPatProp::Rest(rest) => {
                        collect_pat_idents(&rest.arg, idents);
                    }
                }
            }
        }
        Pat::Array(arr) => {
            for elem in arr.elems.iter().flatten() {
                collect_pat_idents(elem, idents);
            }
        }
        Pat::Rest(rest) => {
            collect_pat_idents(&rest.arg, idents);
        }
        Pat::Assign(assign) => {
            collect_pat_idents(&assign.left, idents);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deno_ast::swc::common::{Span, SyntaxContext};

    fn ident(name: &str) -> Ident {
        Ident::new(name.into(), Span::default(), SyntaxContext::default())
    }

    fn make_function(params: Vec<Param>) -> Function {
        Function {
            span: Default::default(),
            ctxt: SyntaxContext::default(),
            decorators: vec![],
            params,
            body: None,
            is_async: false,
            is_generator: false,
            type_params: None,
            return_type: None,
        }
    }

    #[test]
    fn test_collect_fn_params_simple() {
        let f = make_function(vec![
            Param {
                span: Default::default(),
                decorators: vec![],
                pat: Pat::Ident(BindingIdent { id: ident("a"), type_ann: None }),
            },
            Param {
                span: Default::default(),
                decorators: vec![],
                pat: Pat::Ident(BindingIdent { id: ident("b"), type_ann: None }),
            },
        ]);

        let params = collect_fn_params(&f);
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_collect_module_idents_import() {
        let item = ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
            span: Default::default(),
            specifiers: vec![ImportSpecifier::Default(ImportDefaultSpecifier {
                span: Default::default(),
                local: ident("React"),
            })],
            src: Box::new(Str { span: Default::default(), value: "react".into(), raw: None }),
            with: None,
            phase: ImportPhase::Evaluation,
            type_only: false,
        }));

        let idents = collect_module_level_idents(&item);
        assert_eq!(idents.len(), 1);
        let id = idents.into_iter().next().expect("expected imported React identifier");
        assert_eq!(id.0.to_string(), "React");
    }

    #[test]
    fn test_collect_module_idents_fn_decl() {
        let item = ModuleItem::Stmt(Stmt::Decl(Decl::Fn(FnDecl {
            ident: ident("myFunction"),
            function: Box::new(make_function(vec![])),
            declare: false,
        })));

        let idents = collect_module_level_idents(&item);
        assert_eq!(idents.len(), 1);
    }

    #[test]
    fn test_collect_fn_params_with_patterns() {
        let f = make_function(vec![
            Param {
                span: Default::default(),
                decorators: vec![],
                pat: Pat::Object(ObjectPat {
                    span: Default::default(),
                    props: vec![ObjectPatProp::Assign(AssignPatProp {
                        span: Default::default(),
                        key: ident("x").into(),
                        value: None,
                    })],
                    optional: false,
                    type_ann: None,
                }),
            },
            Param {
                span: Default::default(),
                decorators: vec![],
                pat: Pat::Rest(RestPat {
                    span: Default::default(),
                    arg: Box::new(Pat::Ident(BindingIdent { id: ident("rest"), type_ann: None })),
                    type_ann: None,
                    dot3_token: Default::default(),
                }),
            },
        ]);

        let params = collect_fn_params(&f);
        assert_eq!(params.len(), 2);
    }
}
