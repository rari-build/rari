use deno_ast::swc::ast::*;
use deno_ast::swc::common::DUMMY_SP;

fn id(name: &str) -> Ident {
    Ident::new(name.into(), DUMMY_SP, Default::default())
}

fn id_name(name: &str) -> IdentName {
    IdentName { span: DUMMY_SP, sym: name.into() }
}

fn ident_expr(name: &str) -> Box<Expr> {
    Box::new(Expr::Ident(id(name)))
}

fn str_lit(s: &str) -> Str {
    Str { span: DUMMY_SP, value: s.into(), raw: None }
}

fn num(n: f64) -> Number {
    Number { span: DUMMY_SP, value: n, raw: None }
}

fn null_expr() -> Box<Expr> {
    Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP })))
}

/// Strip leading directive statements (string expression statements like "use cache") from a block.
fn strip_directives_from_body(body: &BlockStmt) -> BlockStmt {
    let stmts: Vec<Stmt> = body
        .stmts
        .iter()
        .filter(|stmt| match stmt {
            Stmt::Expr(ExprStmt { expr, .. }) => {
                if let Expr::Lit(Lit::Str(Str { value, .. })) = expr.as_ref() {
                    let s = value.to_string_lossy();
                    !(s == "use cache"
                        || s.starts_with("use cache: ")
                        || s == "use server"
                        || s == "use client")
                } else {
                    true
                }
            }
            _ => true,
        })
        .cloned()
        .collect();

    BlockStmt { span: body.span, ctxt: body.ctxt, stmts }
}

fn create_inner_function(
    fn_decl: &FnDecl,
    closure_vars: &[String],
    inner_name: &str,
) -> ModuleItem {
    let mut new_params = Vec::new();
    if !closure_vars.is_empty() {
        new_params.push(Param {
            span: DUMMY_SP,
            decorators: vec![],
            pat: Pat::Ident(BindingIdent { id: id("$$ACTION_BOUND_ARGS"), type_ann: None }),
        });
    }
    new_params.extend(fn_decl.function.params.clone());

    let clean_body = fn_decl.function.body.as_ref().map(strip_directives_from_body);

    let fn_expr = Expr::Fn(FnExpr {
        ident: Some(Ident::new(fn_decl.ident.sym.clone(), DUMMY_SP, Default::default())),
        function: Box::new(Function {
            span: DUMMY_SP,
            ctxt: Default::default(),
            decorators: fn_decl.function.decorators.clone(),
            params: new_params,
            body: clean_body,
            is_async: fn_decl.function.is_async,
            is_generator: fn_decl.function.is_generator,
            type_params: fn_decl.function.type_params.clone(),
            return_type: fn_decl.function.return_type.clone(),
        }),
    });

    ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
        span: DUMMY_SP,
        ctxt: Default::default(),
        kind: VarDeclKind::Const,
        declare: false,
        decls: vec![VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Ident(BindingIdent { id: id(inner_name), type_ann: None }),
            init: Some(Box::new(fn_expr)),
            definite: false,
        }],
    }))))
}

fn create_name_define_statement(inner_name: &str, export_name: &str) -> ModuleItem {
    ModuleItem::Stmt(Stmt::Expr(ExprStmt {
        span: DUMMY_SP,
        expr: Box::new(Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: Default::default(),
            callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(Expr::Ident(id("Object"))),
                prop: MemberProp::Ident(id_name("defineProperty")),
            }))),
            args: vec![
                ExprOrSpread { spread: None, expr: ident_expr(inner_name) },
                ExprOrSpread { spread: None, expr: Box::new(Expr::Lit(Lit::Str(str_lit("name")))) },
                ExprOrSpread { spread: None, expr: create_value_descriptor(export_name) },
            ],
            type_args: None,
        })),
    }))
}

fn create_value_descriptor(value: &str) -> Box<Expr> {
    Box::new(Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(id_name("value")),
            value: Box::new(Expr::Lit(Lit::Str(str_lit(value)))),
        })))],
    }))
}

fn create_cache_wrapper(
    cache_name: &str,
    inner_name: &str,
    ref_id: &str,
    cache_kind: &str,
    param_count: usize,
) -> ModuleItem {
    let inner_fn = Expr::Fn(FnExpr {
        ident: None,
        function: Box::new(Function {
            span: DUMMY_SP,
            ctxt: Default::default(),
            decorators: vec![],
            params: vec![Param {
                span: DUMMY_SP,
                decorators: vec![],
                pat: Pat::Rest(RestPat {
                    span: DUMMY_SP,
                    dot3_token: DUMMY_SP,
                    arg: Box::new(Pat::Ident(BindingIdent { id: id("args"), type_ann: None })),
                    type_ann: None,
                }),
            }],
            body: Some(BlockStmt {
                span: DUMMY_SP,
                ctxt: Default::default(),
                stmts: vec![Stmt::Return(ReturnStmt {
                    span: DUMMY_SP,
                    arg: Some(Box::new(Expr::Await(AwaitExpr {
                        span: DUMMY_SP,
                        arg: Box::new(Expr::Call(CallExpr {
                            span: DUMMY_SP,
                            ctxt: Default::default(),
                            callee: Callee::Expr(ident_expr("$$cache__")),
                            args: vec![
                                ExprOrSpread {
                                    spread: None,
                                    expr: Box::new(Expr::Lit(Lit::Str(str_lit(cache_kind)))),
                                },
                                ExprOrSpread {
                                    spread: None,
                                    expr: Box::new(Expr::Lit(Lit::Str(str_lit(ref_id)))),
                                },
                                ExprOrSpread {
                                    spread: None,
                                    expr: Box::new(Expr::Lit(Lit::Num(num(param_count as f64)))),
                                },
                                ExprOrSpread { spread: None, expr: ident_expr(inner_name) },
                                ExprOrSpread { spread: None, expr: create_args_slice_expr("args") },
                            ],
                            type_args: None,
                        })),
                    }))),
                })],
            }),
            is_async: true,
            is_generator: false,
            type_params: None,
            return_type: None,
        }),
    });

    ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
        span: DUMMY_SP,
        ctxt: Default::default(),
        kind: VarDeclKind::Var,
        declare: false,
        decls: vec![VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Ident(BindingIdent { id: id(cache_name), type_ann: None }),
            init: Some(Box::new(inner_fn)),
            definite: false,
        }],
    }))))
}

fn create_args_slice_expr(arg_name: &str) -> Box<Expr> {
    Box::new(ident_expr(arg_name).as_ref().clone())
}

fn create_register_ref_statement(cache_name: &str, ref_id: &str) -> ModuleItem {
    ModuleItem::Stmt(Stmt::Expr(ExprStmt {
        span: DUMMY_SP,
        expr: Box::new(Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: Default::default(),
            callee: Callee::Expr(ident_expr("registerServerReference")),
            args: vec![
                ExprOrSpread { spread: None, expr: ident_expr(cache_name) },
                ExprOrSpread { spread: None, expr: Box::new(Expr::Lit(Lit::Str(str_lit(ref_id)))) },
                ExprOrSpread { spread: None, expr: null_expr() },
            ],
            type_args: None,
        })),
    }))
}

pub struct CacheDeclarationInput<'a> {
    pub fn_decl: &'a FnDecl,
    pub export_name: &'a str,
    pub closure_vars: &'a [String],
    pub ref_id: &'a str,
    pub cache_kind: &'a str,
    pub cache_name: &'a str,
    pub inner_name: &'a str,
}

pub fn create_cache_declarations(
    extra_items: &mut Vec<ModuleItem>,
    input: CacheDeclarationInput<'_>,
) {
    let param_count = input.fn_decl.function.params.len();

    extra_items.push(create_inner_function(input.fn_decl, input.closure_vars, input.inner_name));
    extra_items.push(create_name_define_statement(input.inner_name, input.export_name));
    extra_items.push(create_cache_wrapper(
        input.cache_name,
        input.inner_name,
        input.ref_id,
        input.cache_kind,
        param_count,
    ));
    extra_items.push(create_register_ref_statement(input.cache_name, input.ref_id));
}

pub fn create_bound_replacement(
    export_name: &str,
    cache_name: &str,
    ref_id: &str,
    closure_vars: &[String],
) -> ModuleItem {
    let mut bind_args = vec![ExprOrSpread { spread: None, expr: null_expr() }];

    if !closure_vars.is_empty() {
        let mut enc_args = vec![ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Lit(Lit::Str(str_lit(ref_id)))),
        }];
        for var_name in closure_vars {
            enc_args.push(ExprOrSpread { spread: None, expr: ident_expr(var_name) });
        }

        bind_args.push(ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Call(CallExpr {
                span: DUMMY_SP,
                ctxt: Default::default(),
                callee: Callee::Expr(ident_expr("encodeBoundArgs")),
                args: enc_args,
                type_args: None,
            })),
        });
    }

    let bind_expr = Expr::Call(CallExpr {
        span: DUMMY_SP,
        ctxt: Default::default(),
        callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: ident_expr(cache_name),
            prop: MemberProp::Ident(id_name("bind")),
        }))),
        args: bind_args,
        type_args: None,
    });

    ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
        span: DUMMY_SP,
        ctxt: Default::default(),
        kind: VarDeclKind::Var,
        declare: false,
        decls: vec![VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Ident(BindingIdent { id: id(export_name), type_ann: None }),
            init: Some(Box::new(bind_expr)),
            definite: false,
        }],
    }))))
}
