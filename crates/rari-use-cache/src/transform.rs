use rustc_hash::FxHashSet;

use deno_ast::swc::ast::{
    Decl, DefaultDecl, ExportDecl, ExportDefaultDecl, ExportDefaultExpr, Expr, FnDecl, Id, Ident,
    Module, ModuleDecl, ModuleItem, Stmt,
};
use deno_ast::swc::codegen::{Emitter, text_writer::JsWriter};
use deno_ast::swc::common::sync::Lrc;
use deno_ast::swc::common::{DUMMY_SP, FileName, GLOBALS, Globals, SourceMap, SyntaxContext};
use deno_ast::swc::ecma_visit::VisitMut;
use deno_ast::swc::parser::{Parser, StringInput, Syntax, TsSyntax};

use crate::{closure, directive, hoist, id};

#[non_exhaustive]
pub struct TransformOutput {
    pub code: String,
    pub needs_react_cache: bool,
    pub needs_cache_wrapper: bool,
    pub needs_register_ref: bool,
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "Boolean flags track independent transform states, grouping into enum would be less clear"
)]
struct TransformVisitor {
    filename: String,
    hash_salt: String,
    cache_kinds: Vec<String>,
    index: usize,
    has_cache_fns: bool,
    module_idents: FxHashSet<Id>,
    needs_react_cache: bool,
    needs_cache_wrapper: bool,
    needs_register_ref: bool,
}

impl TransformVisitor {
    fn new(filename: &str, hash_salt: &str, cache_kinds: &[String]) -> Self {
        TransformVisitor {
            filename: filename.to_string(),
            hash_salt: hash_salt.to_string(),
            cache_kinds: cache_kinds.to_vec(),
            index: 0,
            has_cache_fns: false,
            module_idents: FxHashSet::default(),
            needs_react_cache: false,
            needs_cache_wrapper: false,
            needs_register_ref: false,
        }
    }
}

impl VisitMut for TransformVisitor {
    fn visit_mut_module(&mut self, module: &mut Module) {
        for item in &module.body {
            for ident in closure::collect_module_level_idents(item) {
                self.module_idents.insert(ident);
            }
        }

        let mut items = std::mem::take(&mut module.body);
        self.visit_mut_module_items(&mut items);
        module.body = items;
    }

    fn visit_mut_module_items(&mut self, items: &mut Vec<ModuleItem>) {
        let mut extra_items = Vec::new();
        let mut new_items = Vec::new();
        let mut i = 0;

        while i < items.len() {
            let processed = self.maybe_transform_item(&items[i], &mut extra_items);
            if let Some(replacement) = processed {
                new_items.append(&mut extra_items);
                new_items.push(replacement);
            } else {
                new_items.push(items[i].clone());
            }
            i += 1;
        }

        *items = new_items;
    }
}

impl TransformVisitor {
    #[expect(
        clippy::too_many_lines,
        reason = "Function handles complex AST pattern matching for various export patterns"
    )]
    fn maybe_transform_item(
        &mut self,
        item: &ModuleItem,
        extra_items: &mut Vec<ModuleItem>,
    ) -> Option<ModuleItem> {
        let (fn_decl, is_exported, is_default_export, reference_export_name, local_binding_name) =
            match item {
                ModuleItem::Stmt(Stmt::Decl(Decl::Fn(fn_decl))) => {
                    let export_name = fn_decl.ident.sym.to_string();
                    (
                        fn_decl.clone(),
                        false,
                        false,
                        export_name.clone(),
                        export_name,
                    )
                }
                ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
                    decl: Decl::Fn(fn_decl),
                    ..
                })) => {
                    let export_name = fn_decl.ident.sym.to_string();
                    (
                        fn_decl.clone(),
                        true,
                        false,
                        export_name.clone(),
                        export_name,
                    )
                }
                ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultDecl(ExportDefaultDecl {
                    decl: DefaultDecl::Fn(fn_expr),
                    ..
                })) => {
                    let ident = fn_expr.ident.clone().unwrap_or_else(|| {
                        Ident::new(
                            "$$RSC_SERVER_CACHE_DEFAULT_EXPORT".into(),
                            DUMMY_SP,
                            SyntaxContext::default(),
                        )
                    });
                    let reference_export_name = fn_expr
                        .ident
                        .as_ref()
                        .map(|ident| ident.sym.to_string())
                        .unwrap_or_else(|| "default".to_string());
                    let local_binding_name = ident.sym.to_string();
                    (
                        FnDecl {
                            ident,
                            function: fn_expr.function.clone(),
                            declare: false,
                        },
                        true,
                        true,
                        reference_export_name,
                        local_binding_name,
                    )
                }
                _ => return None,
            };

        let body = fn_decl.function.body.as_ref()?;

        if !directive::has_use_cache_directive(body) {
            return None;
        }

        self.has_cache_fns = true;
        let cache_kind = directive::extract_cache_kind(body).unwrap_or_else(|| {
            if self.cache_kinds.is_empty() {
                "default".to_string()
            } else {
                self.cache_kinds[0].clone()
            }
        });

        let fn_params = closure::collect_fn_params(&fn_decl.function);
        let closure_vars = closure::collect_closure_idents(
            body,
            &self.module_idents,
            &fn_params,
            Some(&fn_decl.ident),
        );

        let ref_id = id::generate_reference_id(
            &self.hash_salt,
            &self.filename,
            &reference_export_name,
            true,
        );

        let cache_name = id::generate_cache_export_name(self.index, &reference_export_name);
        let inner_name = id::generate_cache_inner_name(self.index, &reference_export_name);

        hoist::create_cache_declarations(
            extra_items,
            hoist::CacheDeclarationInput {
                fn_decl: &fn_decl,
                export_name: &reference_export_name,
                closure_vars: &closure_vars,
                ref_id: &ref_id,
                cache_kind: &cache_kind,
                cache_name: &cache_name,
                inner_name: &inner_name,
            },
        );

        self.needs_cache_wrapper = true;
        self.needs_register_ref = true;

        let replacement = hoist::create_bound_replacement(
            &local_binding_name,
            &cache_name,
            &ref_id,
            &closure_vars,
        );

        self.index += 1;

        if is_default_export {
            extra_items.push(replacement);
            Some(ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(
                ExportDefaultExpr {
                    span: DUMMY_SP,
                    expr: Box::new(Expr::Ident(Ident::new(
                        local_binding_name.into(),
                        DUMMY_SP,
                        SyntaxContext::default(),
                    ))),
                },
            )))
        } else if is_exported {
            Some(ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
                span: DUMMY_SP,
                decl: match replacement {
                    ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) => Decl::Var(var_decl),
                    _ => return None,
                },
            })))
        } else {
            Some(replacement)
        }
    }
}

/// Transforms source code to add cache functionality.
///
/// # Errors
///
/// Returns an error if parsing fails or code generation fails.
pub fn transform_source(
    source: &str,
    filename: &str,
    hash_salt: &str,
    cache_kinds: &[String],
) -> Result<TransformOutput, String> {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        GLOBALS.set(&Globals::default(), || {
            let cm: Lrc<SourceMap> =
                SourceMap::new(deno_ast::swc::common::FilePathMapping::empty()).into();
            let fm = cm.new_source_file(
                FileName::Real(std::path::PathBuf::from(filename)).into(),
                source.to_string(),
            );

            let mut parser = Parser::new(
                Syntax::Typescript(TsSyntax {
                    tsx: true,
                    ..Default::default()
                }),
                StringInput::from(&*fm),
                None,
            );

            let mut module: Module = parser
                .parse_module()
                .map_err(|e| format!("Parse error: {e:?}"))?;

            let mut visitor = TransformVisitor::new(filename, hash_salt, cache_kinds);
            visitor.visit_mut_module(&mut module);

            if !visitor.has_cache_fns {
                return Ok(TransformOutput {
                    code: source.to_string(),
                    needs_react_cache: false,
                    needs_cache_wrapper: false,
                    needs_register_ref: false,
                });
            }

            let mut code_buf = Vec::new();
            {
                let mut emitter = Emitter {
                    cfg: deno_ast::swc::codegen::Config::default(),
                    cm: std::rc::Rc::clone(&cm),
                    comments: None,
                    wr: JsWriter::new(std::rc::Rc::clone(&cm), "\n", &mut code_buf, None),
                };
                emitter
                    .emit_module(&module)
                    .map_err(|e| format!("Codegen error: {e:?}"))?;
            }
            let code = String::from_utf8(code_buf).map_err(|e| format!("UTF-8 error: {e:?}"))?;

            Ok(TransformOutput {
                code,
                needs_react_cache: visitor.needs_react_cache,
                needs_cache_wrapper: visitor.needs_cache_wrapper,
                needs_register_ref: visitor.needs_register_ref,
            })
        })
    }));

    match result {
        Ok(inner) => inner,
        Err(panic) => {
            let msg = if let Some(s) = panic.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = panic.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "Unknown panic during transformation".to_string()
            };
            Err(format!("Panic: {msg}"))
        }
    }
}
