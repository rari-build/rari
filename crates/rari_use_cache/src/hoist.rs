use deno_ast::swc::{
    ast::{
        ArrayLit, ArrowExpr, AwaitExpr, BinExpr, BinaryOp, BindingIdent, BlockStmt,
        BlockStmtOrExpr, CallExpr, Callee, CatchClause, Decl, Expr, ExprOrSpread, ExprStmt, FnDecl,
        FnExpr, Function, Ident, IdentName, IfStmt, KeyValueProp, Lit, MemberExpr, MemberProp,
        ModuleItem, Null, Number, ObjectLit, Param, ParenExpr, Pat, Prop, PropName, PropOrSpread,
        RestPat, ReturnStmt, Stmt, Str, ThrowStmt, TryStmt, UnaryExpr, UnaryOp, VarDecl,
        VarDeclKind, VarDeclarator,
    },
    common::{DUMMY_SP, SyntaxContext},
};

fn id(name: &str) -> Ident {
    Ident::new(name.into(), DUMMY_SP, SyntaxContext::default())
}

fn id_name(name: &str) -> IdentName {
    IdentName { span: DUMMY_SP, sym: name.into() }
}

#[expect(
    clippy::unnecessary_box_returns,
    reason = "Consistent with deno_ast's AST building patterns"
)]
fn ident_expr(name: &str) -> Box<Expr> {
    Box::new(Expr::Ident(id(name)))
}

fn str_lit(s: &str) -> Str {
    Str { span: DUMMY_SP, value: s.into(), raw: None }
}

fn num(n: f64) -> Number {
    Number { span: DUMMY_SP, value: n, raw: None }
}

#[expect(
    clippy::unnecessary_box_returns,
    reason = "Consistent with deno_ast's AST building patterns"
)]
fn null_expr() -> Box<Expr> {
    Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP })))
}

fn is_directive_stmt(stmt: &Stmt) -> bool {
    if let Stmt::Expr(ExprStmt { expr, .. }) = stmt
        && let Expr::Lit(Lit::Str(Str { value, .. })) = expr.as_ref()
    {
        let s = value.to_string_lossy();
        return s == "use cache"
            || s.starts_with("use cache: ")
            || s == "use server"
            || s == "use client";
    }
    false
}

/// Strip leading directive statements (string expression statements like "use cache") from a block.
fn strip_directives_from_body(body: &BlockStmt) -> BlockStmt {
    let mut stmts = Vec::with_capacity(body.stmts.len());
    let mut stripping = true;
    for stmt in &body.stmts {
        if stripping && is_directive_stmt(stmt) {
            continue;
        }
        stripping = false;
        stmts.push(stmt.clone());
    }

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
        ident: Some(Ident::new(fn_decl.ident.sym.clone(), DUMMY_SP, SyntaxContext::default())),
        function: Box::new(Function {
            span: DUMMY_SP,
            ctxt: SyntaxContext::default(),
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
        ctxt: SyntaxContext::default(),
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
            ctxt: SyntaxContext::default(),
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

#[expect(
    clippy::unnecessary_box_returns,
    reason = "Consistent with deno_ast's AST building patterns"
)]
fn create_value_descriptor(value: &str) -> Box<Expr> {
    Box::new(Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(id_name("value")),
            value: Box::new(Expr::Lit(Lit::Str(str_lit(value)))),
        })))],
    }))
}

#[expect(
    clippy::cast_precision_loss,
    reason = "param_count represents function parameters, typically <10, well within f64 precision"
)]
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
            ctxt: SyntaxContext::default(),
            decorators: vec![],
            params: vec![],
            body: Some(BlockStmt {
                span: DUMMY_SP,
                ctxt: SyntaxContext::default(),
                stmts: vec![Stmt::Return(ReturnStmt {
                    span: DUMMY_SP,
                    arg: Some(Box::new(Expr::Call(CallExpr {
                        span: DUMMY_SP,
                        ctxt: SyntaxContext::default(),
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
                            ExprOrSpread { spread: None, expr: create_args_slice_expr() },
                        ],
                        type_args: None,
                    }))),
                })],
            }),
            is_async: false,
            is_generator: false,
            type_params: None,
            return_type: None,
        }),
    });

    ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
        span: DUMMY_SP,
        ctxt: SyntaxContext::default(),
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

#[expect(
    clippy::unnecessary_box_returns,
    reason = "Consistent with deno_ast's AST building patterns"
)]
fn create_args_slice_expr() -> Box<Expr> {
    // Array.prototype.slice.call(arguments)
    let callee = Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(Expr::Ident(id("Array"))),
            prop: MemberProp::Ident(id_name("prototype")),
        })),
        prop: MemberProp::Ident(id_name("slice")),
    });
    let call_callee = Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(callee),
        prop: MemberProp::Ident(id_name("call")),
    });
    Box::new(Expr::Call(CallExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::default(),
        callee: Callee::Expr(Box::new(call_callee)),
        args: vec![ExprOrSpread { spread: None, expr: ident_expr("arguments") }],
        type_args: None,
    }))
}

fn create_register_ref_statement(cache_name: &str, ref_id: &str) -> ModuleItem {
    ModuleItem::Stmt(Stmt::Expr(ExprStmt {
        span: DUMMY_SP,
        expr: Box::new(Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::default(),
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

#[non_exhaustive]
pub struct CacheDeclarationInput<'a> {
    pub fn_decl: &'a FnDecl,
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
pub fn create_cache_declarations(
    extra_items: &mut Vec<ModuleItem>,
    input: CacheDeclarationInput<'_>,
) {
    let param_count =
        input.fn_decl.function.params.len() + usize::from(!input.closure_vars.is_empty());

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

/// Build the `apply` call argument array: `[$$ACTION_BOUND_ARGS, ...args]` or `[...args]`.
fn build_apply_args_array(closure_vars_empty: bool) -> ArrayLit {
    if closure_vars_empty {
        ArrayLit {
            span: DUMMY_SP,
            elems: vec![Some(ExprOrSpread { spread: Some(DUMMY_SP), expr: ident_expr("args") })],
        }
    } else {
        let mut elems: Vec<Option<ExprOrSpread>> = vec![Some(ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Ident(id("$$ACTION_BOUND_ARGS"))),
        })];
        elems.push(Some(ExprOrSpread { spread: Some(DUMMY_SP), expr: ident_expr("args") }));
        ArrayLit { span: DUMMY_SP, elems }
    }
}

/// Build the `try { return cacheName.apply(null, [...]); } catch (e) { if (e?.then) return await e; throw e; }` body.
#[expect(
    clippy::needless_pass_by_value,
    reason = "ArrayLit is constructed inline and ownership transfer is cleaner"
)]
fn build_try_body(cache_name: &str, apply_args_arr: ArrayLit) -> Stmt {
    let apply_call = || {
        Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::default(),
            callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                span: DUMMY_SP,
                obj: ident_expr(cache_name),
                prop: MemberProp::Ident(id_name("apply")),
            }))),
            args: vec![
                ExprOrSpread { spread: None, expr: null_expr() },
                ExprOrSpread { spread: None, expr: Box::new(Expr::Array(apply_args_arr.clone())) },
            ],
            type_args: None,
        })
    };

    Stmt::Try(Box::new(TryStmt {
        span: DUMMY_SP,
        block: BlockStmt {
            span: DUMMY_SP,
            ctxt: SyntaxContext::default(),
            stmts: vec![Stmt::Return(ReturnStmt {
                span: DUMMY_SP,
                arg: Some(Box::new(apply_call())),
            })],
        },
        handler: Some(CatchClause {
            span: DUMMY_SP,
            param: Some(Pat::Ident(BindingIdent { id: id("e"), type_ann: None })),
            body: BlockStmt {
                span: DUMMY_SP,
                ctxt: SyntaxContext::default(),
                stmts: vec![
                    Stmt::If(IfStmt {
                        span: DUMMY_SP,
                        test: Box::new(Expr::Bin(BinExpr {
                            span: DUMMY_SP,
                            op: BinaryOp::LogicalAnd,
                            left: Box::new(Expr::Bin(BinExpr {
                                span: DUMMY_SP,
                                op: BinaryOp::NotEqEq,
                                left: ident_expr("e"),
                                right: Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))),
                            })),
                            right: Box::new(Expr::Bin(BinExpr {
                                span: DUMMY_SP,
                                op: BinaryOp::EqEqEq,
                                left: Box::new(Expr::Unary(UnaryExpr {
                                    span: DUMMY_SP,
                                    op: UnaryOp::TypeOf,
                                    arg: Box::new(Expr::Member(MemberExpr {
                                        span: DUMMY_SP,
                                        obj: ident_expr("e"),
                                        prop: MemberProp::Ident(id_name("then")),
                                    })),
                                })),
                                right: Box::new(Expr::Lit(Lit::Str(str_lit("function")))),
                            })),
                        })),
                        cons: Box::new(Stmt::Return(ReturnStmt {
                            span: DUMMY_SP,
                            arg: Some(Box::new(Expr::Await(AwaitExpr {
                                span: DUMMY_SP,
                                arg: ident_expr("e"),
                            }))),
                        })),
                        alt: None,
                    }),
                    Stmt::Throw(ThrowStmt { span: DUMMY_SP, arg: ident_expr("e") }),
                ],
            },
        }),
        finalizer: None,
    }))
}

/// Build the inner function body containing the try/catch around cacheName.apply.
fn build_inner_body(cache_name: &str, has_bound_args: bool) -> BlockStmt {
    let apply_args_arr = build_apply_args_array(!has_bound_args);
    BlockStmt {
        span: DUMMY_SP,
        ctxt: SyntaxContext::default(),
        stmts: vec![build_try_body(cache_name, apply_args_arr)],
    }
}

/// Build the rest parameter `...args` Param.
fn build_rest_args_param() -> Param {
    Param {
        span: DUMMY_SP,
        decorators: vec![],
        pat: Pat::Rest(RestPat {
            span: DUMMY_SP,
            dot3_token: DUMMY_SP,
            arg: Box::new(Pat::Ident(BindingIdent { id: id("args"), type_ann: None })),
            type_ann: None,
        }),
    }
}

pub fn create_bound_replacement(
    export_name: &str,
    cache_name: &str,
    ref_id: &str,
    closure_vars: &[String],
) -> ModuleItem {
    // We emit an async wrapper so that the throw-a-Promise (suspense signal)
    // from `$$cache__` is converted to a real await/rejection by the `await`
    // in the caller's component. Without this, the throw propagates up the
    // call stack to the React RSC serializer, which can't recognise a raw
    // Promise as a suspense signal and renders it as `[object Promise]`.

    let has_bound_args = !closure_vars.is_empty();
    let inner_body = build_inner_body(cache_name, has_bound_args);

    let final_init: Box<Expr> = if has_bound_args {
        // Closure captures present — wrap in an arrow that pre-binds the
        // encoded closure args, then returns an async function for the user args.
        //
        // ($$ba => async (...args) => { try { return cacheName.apply(null, [$$ba, ...args]); }
        //                              catch (e) { if (e && typeof e.then === 'function') { await e; } throw e; } })
        // (encodeBoundArgs(refId, ...closure_vars))
        let mut enc_args = vec![ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Lit(Lit::Str(str_lit(ref_id)))),
        }];
        for var_name in closure_vars {
            enc_args.push(ExprOrSpread { spread: None, expr: ident_expr(var_name) });
        }
        let bound_arg_call = Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::default(),
            callee: Callee::Expr(ident_expr("encodeBoundArgs")),
            args: enc_args,
            type_args: None,
        });

        // Inner async arrow: async (...args) => { ... }
        let inner_async = Expr::Arrow(ArrowExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::default(),
            params: vec![Pat::Rest(RestPat {
                span: DUMMY_SP,
                dot3_token: DUMMY_SP,
                arg: Box::new(Pat::Ident(BindingIdent { id: id("args"), type_ann: None })),
                type_ann: None,
            })],
            body: Box::new(BlockStmtOrExpr::BlockStmt(inner_body)),
            is_async: true,
            is_generator: false,
            type_params: None,
            return_type: None,
        });

        // Outer arrow: $$ACTION_BOUND_ARGS => inner_async
        let outer_arrow = Expr::Arrow(ArrowExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::default(),
            params: vec![Pat::Ident(BindingIdent {
                id: id("$$ACTION_BOUND_ARGS"),
                type_ann: None,
            })],
            body: Box::new(BlockStmtOrExpr::Expr(Box::new(inner_async))),
            is_async: false,
            is_generator: false,
            type_params: None,
            return_type: None,
        });

        // IIFE: (outer_arrow)(encodeBoundArgs(...))
        Box::new(Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::default(),
            callee: Callee::Expr(Box::new(Expr::Paren(ParenExpr {
                span: DUMMY_SP,
                expr: Box::new(outer_arrow),
            }))),
            args: vec![ExprOrSpread { spread: None, expr: Box::new(bound_arg_call) }],
            type_args: None,
        }))
    } else {
        // No closure captures — simple async function.
        Box::new(Expr::Fn(FnExpr {
            ident: None,
            function: Box::new(Function {
                span: DUMMY_SP,
                ctxt: SyntaxContext::default(),
                decorators: vec![],
                params: vec![build_rest_args_param()],
                body: Some(inner_body),
                is_async: true,
                is_generator: false,
                type_params: None,
                return_type: None,
            }),
        }))
    };

    ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
        span: DUMMY_SP,
        ctxt: SyntaxContext::default(),
        kind: VarDeclKind::Var,
        declare: false,
        decls: vec![VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Ident(BindingIdent { id: id(export_name), type_ann: None }),
            init: Some(final_init),
            definite: false,
        }],
    }))))
}
