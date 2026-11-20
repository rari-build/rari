// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use std::collections::{HashMap, HashSet};

use oxc::{
    ast::ast::*,
    ast_visit::{Visit, walk},
};

use crate::{JSStrExt, ParsedSource};

pub type CjsAnalysis = crate::ModuleExportsAndReExports;

impl ParsedSource<'_> {
    /// Analyzes the script for CommonJS exports and re-exports based on similar
    /// functionality to cjs-module-lexer (https://github.com/nodejs/cjs-module-lexer).
    ///
    /// The result describes the source as originally parsed and does not
    /// represent transpiled output.
    pub fn analyze_cjs(&self) -> CjsAnalysis {
        let scoping =
            oxc::semantic::SemanticBuilder::new().build(&self.program).semantic.into_scoping();
        let mut visitor = CjsVisitor {
            scoping: &scoping,
            exports: HashSet::new(),
            unsafe_exports: HashSet::new(),
            reexports: HashSet::new(),
            var_assignments: HashMap::new(),
            enumerated_key_scopes: Vec::new(),
        };
        visitor.visit_program(&self.program);
        visitor.take_result()
    }
}

struct CjsVisitor<'a> {
    scoping: &'a oxc::semantic::Scoping,
    exports: HashSet<String>,
    unsafe_exports: HashSet<String>,
    reexports: HashSet<String>,
    var_assignments: HashMap<oxc::semantic::SymbolId, String>,
    enumerated_key_scopes: Vec<(oxc::semantic::SymbolId, oxc::semantic::SymbolId)>,
}

impl CjsVisitor<'_> {
    fn take_result(self) -> CjsAnalysis {
        let mut exports = self
            .exports
            .into_iter()
            .filter(|name| !self.unsafe_exports.contains(name))
            .collect::<Vec<_>>();
        exports.sort_unstable();
        let mut reexports = self.reexports.into_iter().collect::<Vec<_>>();
        reexports.sort_unstable();
        CjsAnalysis { exports, reexports }
    }

    fn add_export(&mut self, export: &str) {
        self.exports.insert(export.to_string());
    }

    fn add_reexport(&mut self, reexport: &str) {
        self.reexports.insert(reexport.to_string());
    }

    fn set_reexport_assignment_value(&mut self, value: &str) {
        self.reexports.clear();
        self.add_reexport(value);
    }

    fn add_unsafe_getter(&mut self, name: &str) {
        self.unsafe_exports.insert(name.to_string());
        self.exports.remove(name);
    }

    fn get_member_require_value(
        &self,
        member: &MemberExpression,
        key: &IdentifierReference,
    ) -> Option<String> {
        let obj_ident = get_static_member_obj_ident(member)?;
        let object_symbol = get_reference_symbol_id(obj_ident, self.scoping)?;
        let key_symbol = get_reference_symbol_id(key, self.scoping)?;
        let member_key = get_computed_member_prop_ident(member)?;
        let member_key_symbol = get_reference_symbol_id(member_key, self.scoping)?;
        if member_key_symbol != key_symbol {
            return None;
        }
        if !self.enumerated_key_scopes.iter().any(|&(enumerated_key, enumerated_object)| {
            enumerated_key == key_symbol && enumerated_object == object_symbol
        }) {
            return None;
        }
        self.var_assignments.get(&object_symbol).cloned()
    }

    fn visit_object_define(&mut self, call_expr: &CallExpression) {
        let args = &call_expr.arguments;
        if args.len() < 3 {
            return;
        }
        match &args[1] {
            Argument::StringLiteral(str_lit) => {
                let lit_value = str_lit.value.to_str_lossy();
                if is_valid_object_define_obj_lit(&args[2]) {
                    self.add_export(&lit_value);
                } else {
                    self.add_unsafe_getter(&lit_value);
                }
            }
            Argument::Identifier(ident) => {
                if let Some(obj_lit) = get_arg_as_object_expr(&args[2])
                    && let Some(get_prop) = get_object_define_get_prop(obj_lit)
                    && let Some(return_expr) = get_prop_return_expr(get_prop)
                    && let Some(member) = return_expr.as_member_expression()
                    && let Some(require_value) = self.get_member_require_value(member, ident)
                {
                    self.add_reexport(&require_value);
                }
            }
            _ => {}
        }
    }

    fn visit_exports_right_expr(&mut self, right_expr: &Expression) {
        match unwrap_parenthesized(right_expr) {
            Expression::ObjectExpression(object_lit) => {
                for prop in &object_lit.properties {
                    match prop {
                        ObjectPropertyKind::ObjectProperty(prop) => {
                            if let Some(prop_name) = get_property_key_name(&prop.key) {
                                if prop_name == "__proto__"
                                    && !prop.computed
                                    && !prop.method
                                    && !prop.shorthand
                                    && prop.kind == PropertyKind::Init
                                {
                                    continue;
                                }
                                if is_supported_object_prop(prop.as_ref()) {
                                    self.add_export(&prop_name);
                                } else {
                                    self.add_unsafe_getter(&prop_name);
                                }
                            }
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            if let Some(require_value) =
                                get_expr_require_value(&spread.argument, self.scoping)
                            {
                                self.add_reexport(require_value);
                            }
                        }
                    }
                }
            }
            Expression::CallExpression(call_expr) => {
                if let Some(require_value) = get_call_expr_require_value(call_expr, self.scoping) {
                    self.set_reexport_assignment_value(require_value);
                }
            }
            Expression::AssignmentExpression(right_assign_expr)
                if right_assign_expr.operator == AssignmentOperator::Assign =>
            {
                self.visit_exports_right_expr(&right_assign_expr.right);
            }
            _ => {}
        }
    }
}

impl<'a> Visit<'a> for CjsVisitor<'_> {
    fn visit_variable_declaration(&mut self, decl: &VariableDeclaration<'a>) {
        for declarator in &decl.declarations {
            walk::walk_variable_declarator(self, declarator);
            if let Some(id) = declarator.id.get_binding_identifier()
                && let Some(symbol_id) = id.symbol_id.get()
            {
                if let Some(init) = &declarator.init {
                    if let Some(require_value) = get_expr_require_value(init, self.scoping) {
                        self.var_assignments.insert(symbol_id, require_value.to_string());
                    } else {
                        self.var_assignments.remove(&symbol_id);
                    }
                }
            }
        }
    }

    fn visit_call_expression(&mut self, call_expr: &CallExpression<'a>) {
        let enumerated_key_scope =
            get_for_each_key_scope(call_expr, &self.var_assignments, self.scoping);
        if let Some(scope) = enumerated_key_scope {
            self.enumerated_key_scopes.push(scope);
        }

        if is_object_define(call_expr, self.scoping) {
            self.visit_object_define(call_expr);
        } else if is_export_callee(&call_expr.callee)
            && let Some(name) =
                get_export_require_arg(call_expr, &self.var_assignments, self.scoping)
        {
            self.add_reexport(&name);
        }

        walk::walk_call_expression(self, call_expr);
        if enumerated_key_scope.is_some() {
            self.enumerated_key_scopes.pop();
        }
    }

    fn visit_assignment_expression(&mut self, assign_expr: &AssignmentExpression<'a>) {
        walk::walk_assignment_expression(self, assign_expr);

        if assign_expr.operator == AssignmentOperator::Assign {
            match &assign_expr.left {
                AssignmentTarget::AssignmentTargetIdentifier(_) => {}
                _ => {
                    if let Some(member) = assign_expr.left.as_member_expression() {
                        if is_module_exports_member(member, self.scoping) {
                            self.visit_exports_right_expr(&assign_expr.right);
                        } else if is_module_exports_or_exports_obj(member, self.scoping) {
                            if let Some(prop_name) = get_member_expr_prop_text(member) {
                                if prop_name != "__proto__" {
                                    self.add_export(&prop_name);
                                }
                            } else if let Some(right_member) =
                                assign_expr.right.as_member_expression()
                                && let Some(computed_key) = get_computed_member_prop_ident(member)
                                && let Some(require_value) =
                                    self.get_member_require_value(right_member, computed_key)
                            {
                                self.add_reexport(&require_value);
                            }
                        }
                    }
                }
            }
        }

        if let AssignmentTarget::AssignmentTargetIdentifier(ident) = &assign_expr.left
            && let Some(reference_id) = ident.reference_id.get()
        {
            let reference = self.scoping.get_reference(reference_id);
            if reference.is_write()
                && let Some(symbol_id) = reference.symbol_id()
            {
                self.var_assignments.remove(&symbol_id);
            }
        }
    }
}

// --- Helper functions ---

fn is_module_exports_or_exports_obj(
    member: &MemberExpression,
    scoping: &oxc::semantic::Scoping,
) -> bool {
    is_module_exports_expr(member.object(), scoping) || is_exports_expr(member.object(), scoping)
}

fn is_module_exports_expr(expr: &Expression, scoping: &oxc::semantic::Scoping) -> bool {
    if let Some(member) = expr.as_member_expression() {
        is_module_exports_member(member, scoping)
    } else {
        false
    }
}

fn is_module_exports_member(member: &MemberExpression, scoping: &oxc::semantic::Scoping) -> bool {
    if let Some(ident) = get_expr_ident(member.object())
        && is_global_identifier("module", ident, scoping)
    {
        return get_member_expr_prop_text(member).map(|p| p == "exports").unwrap_or(false);
    }
    false
}

fn is_exports_expr(expr: &Expression, scoping: &oxc::semantic::Scoping) -> bool {
    get_expr_ident(expr).is_some_and(|ident| is_global_identifier("exports", ident, scoping))
}

fn get_expr_ident<'a>(expr: &'a Expression<'a>) -> Option<&'a IdentifierReference<'a>> {
    if let Expression::Identifier(ident) = expr { Some(ident) } else { None }
}

fn is_global_identifier(
    name: &str,
    ident: &IdentifierReference,
    scoping: &oxc::semantic::Scoping,
) -> bool {
    ident.name == name && get_reference_symbol_id(ident, scoping).is_none()
}

fn get_reference_symbol_id(
    ident: &IdentifierReference,
    scoping: &oxc::semantic::Scoping,
) -> Option<oxc::semantic::SymbolId> {
    ident.reference_id.get().and_then(|id| scoping.get_reference(id).symbol_id())
}

fn get_expr_require_value<'a>(
    expr: &'a Expression,
    scoping: &oxc::semantic::Scoping,
) -> Option<&'a str> {
    match expr {
        Expression::CallExpression(call_expr) => get_call_expr_require_value(call_expr, scoping),
        Expression::ParenthesizedExpression(paren) => {
            get_expr_require_value(&paren.expression, scoping)
        }
        _ => None,
    }
}

fn get_call_expr_require_value<'a>(
    call_expr: &'a CallExpression,
    scoping: &oxc::semantic::Scoping,
) -> Option<&'a str> {
    let callee = get_expr_ident(&call_expr.callee)?;
    let callee_name = callee.name.as_str();
    match callee_name {
        "require" if is_global_identifier("require", callee, scoping) => {
            let arg = call_expr.arguments.first()?;
            if let Argument::StringLiteral(str_lit) = arg {
                return str_lit.value.as_str();
            }
        }
        "_interopRequireWildcard" => {
            let arg = call_expr.arguments.first()?;
            if let Argument::CallExpression(inner_call) = arg {
                return get_call_expr_require_value(inner_call, scoping);
            }
        }
        _ => {}
    }
    None
}

fn get_expr_ident_name<'a>(expr: &'a Expression) -> Option<&'a str> {
    match expr {
        Expression::Identifier(ident) => Some(ident.name.as_str()),
        _ => None,
    }
}

fn get_static_member_obj_ident<'a>(
    member: &'a MemberExpression<'a>,
) -> Option<&'a IdentifierReference<'a>> {
    get_expr_ident(member.object())
}

fn get_computed_member_prop_ident<'a>(
    member: &'a MemberExpression<'a>,
) -> Option<&'a IdentifierReference<'a>> {
    match member {
        MemberExpression::ComputedMemberExpression(computed) => {
            get_expr_ident(&computed.expression)
        }
        _ => None,
    }
}

fn get_for_each_key_scope(
    call_expr: &CallExpression,
    var_assignments: &HashMap<oxc::semantic::SymbolId, String>,
    scoping: &oxc::semantic::Scoping,
) -> Option<(oxc::semantic::SymbolId, oxc::semantic::SymbolId)> {
    let for_each = call_expr.callee.as_member_expression()?;
    if get_member_expr_prop_text(for_each)?.as_str() != "forEach" {
        return None;
    }

    let Expression::CallExpression(keys_call) = for_each.object() else {
        return None;
    };
    let keys_callee = get_callee_member_expr(&keys_call.callee)?;
    let object_ident = get_expr_ident(keys_callee.object())?;
    if !is_global_identifier("Object", object_ident, scoping)
        || get_member_expr_prop_text(keys_callee)?.as_str() != "keys"
    {
        return None;
    }

    let [Argument::Identifier(object_ident)] = keys_call.arguments.as_slice() else {
        return None;
    };
    let object_symbol = get_reference_symbol_id(object_ident, scoping)?;
    if !var_assignments.contains_key(&object_symbol) {
        return None;
    }

    let callback = call_expr.arguments.first()?;
    let params = match callback {
        Argument::ArrowFunctionExpression(arrow) => &arrow.params,
        Argument::FunctionExpression(function) => &function.params,
        _ => return None,
    };
    let key = params.items.first()?.pattern.get_binding_identifier()?;
    Some((key.symbol_id.get()?, object_symbol))
}

fn get_member_expr_prop_text(member: &MemberExpression) -> Option<String> {
    match member {
        MemberExpression::StaticMemberExpression(static_member) => {
            Some(static_member.property.name.to_string())
        }
        MemberExpression::ComputedMemberExpression(computed) => {
            match unwrap_parenthesized(&computed.expression) {
                Expression::StringLiteral(str_lit) => Some(str_lit.value.to_string_lossy()),
                Expression::NumericLiteral(num) => Some(number_property_name(num.value)),
                _ => None,
            }
        }
        MemberExpression::PrivateFieldExpression(_) => None,
    }
}

fn is_object_define(call_expr: &CallExpression, scoping: &oxc::semantic::Scoping) -> bool {
    is_object_define_callee(&call_expr.callee, scoping)
        && call_expr.arguments.len() >= 3
        && is_module_exports_or_exports_arg(&call_expr.arguments[0], scoping)
        && !call_expr.arguments[0].is_spread()
        && !call_expr.arguments[1].is_spread()
        && !call_expr.arguments[2].is_spread()
}

fn is_module_exports_or_exports_arg(arg: &Argument, scoping: &oxc::semantic::Scoping) -> bool {
    match arg {
        Argument::Identifier(ident) => is_global_identifier("exports", ident, scoping),
        _ => {
            if let Some(member) = arg.as_member_expression() {
                is_module_exports_member(member, scoping)
            } else {
                false
            }
        }
    }
}

fn is_object_define_callee(callee: &Expression, scoping: &oxc::semantic::Scoping) -> bool {
    if let Some(member) = get_callee_member_expr(callee) {
        get_expr_ident(member.object())
            .is_some_and(|ident| is_global_identifier("Object", ident, scoping))
            && get_member_expr_prop_text(member).map(|p| p == "defineProperty").unwrap_or(false)
    } else {
        false
    }
}

fn is_export_callee(callee: &Expression) -> bool {
    if let Some(member) = get_callee_member_expr(callee) {
        if get_expr_ident_name(member.object()).is_some()
            && let Some(right_side) = get_member_expr_prop_text(member)
        {
            return matches!(right_side.as_str(), "__exportStar" | "__export");
        }
    } else if let Some(name) = get_expr_ident_name(callee) {
        return matches!(name, "__exportStar" | "__export");
    }
    false
}

fn get_callee_member_expr<'a>(expr: &'a Expression<'a>) -> Option<&'a MemberExpression<'a>> {
    if let Some(member) = expr.as_member_expression() {
        return Some(member);
    }
    if let Expression::ParenthesizedExpression(paren) = expr
        && let Expression::SequenceExpression(seq) = &paren.expression
    {
        if seq.expressions.len() != 2 {
            return None;
        }
        let first = &seq.expressions[0];
        if let Expression::NumericLiteral(num) = first {
            if num.value != 0f64 {
                return None;
            }
        } else {
            return None;
        }
        return seq.expressions[1].as_member_expression();
    }
    None
}

fn get_export_require_arg(
    call_expr: &CallExpression,
    var_assignments: &HashMap<oxc::semantic::SymbolId, String>,
    scoping: &oxc::semantic::Scoping,
) -> Option<String> {
    if call_expr.arguments.iter().any(oxc::ast::ast::Argument::is_spread) {
        return None;
    }

    if call_expr.arguments.len() == 1
        || (call_expr.arguments.len() == 2 && is_exports_arg(&call_expr.arguments[1], scoping))
    {
        if let Some(require_value) = get_arg_expr_require_value(&call_expr.arguments[0], scoping) {
            return Some(require_value.to_string());
        }
        if let Argument::Identifier(ident) = &call_expr.arguments[0] {
            return get_reference_symbol_id(ident, scoping)
                .and_then(|id| var_assignments.get(&id).cloned());
        }
    }
    None
}

fn is_exports_arg(arg: &Argument, scoping: &oxc::semantic::Scoping) -> bool {
    match arg {
        Argument::Identifier(ident) => is_global_identifier("exports", ident, scoping),
        _ => false,
    }
}

fn get_arg_expr_require_value<'a>(
    arg: &'a Argument,
    scoping: &oxc::semantic::Scoping,
) -> Option<&'a str> {
    match arg {
        Argument::CallExpression(call_expr) => get_call_expr_require_value(call_expr, scoping),
        Argument::ParenthesizedExpression(paren) => {
            get_expr_require_value(&paren.expression, scoping)
        }
        _ => None,
    }
}

fn is_valid_object_define_obj_lit(arg: &Argument) -> bool {
    let Some(obj) = get_arg_as_object_expr(arg) else {
        return false;
    };
    has_supported_get_prop(obj)
}

fn get_arg_as_object_expr<'a>(arg: &'a Argument) -> Option<&'a ObjectExpression<'a>> {
    match arg {
        Argument::ObjectExpression(obj) => Some(obj),
        Argument::ParenthesizedExpression(paren) => get_expr_as_object_expr(&paren.expression),
        _ => None,
    }
}

fn get_expr_as_object_expr<'a>(expr: &'a Expression) -> Option<&'a ObjectExpression<'a>> {
    match expr {
        Expression::ObjectExpression(obj) => Some(obj),
        Expression::ParenthesizedExpression(paren) => get_expr_as_object_expr(&paren.expression),
        _ => None,
    }
}

fn has_supported_get_prop(obj: &ObjectExpression) -> bool {
    match get_effective_get_prop(obj) {
        Ok(Some(get_prop)) => is_supported_get_prop(get_prop),
        Ok(None) => true,
        Err(()) => false,
    }
}

fn get_object_define_get_prop<'a>(obj: &'a ObjectExpression) -> Option<&'a ObjectProperty<'a>> {
    get_effective_get_prop(obj).ok().flatten()
}

fn get_effective_get_prop<'a>(
    obj: &'a ObjectExpression,
) -> Result<Option<&'a ObjectProperty<'a>>, ()> {
    for property in obj.properties.iter().rev() {
        match property {
            ObjectPropertyKind::ObjectProperty(prop) => {
                let name = get_property_key_name(&prop.key);
                if prop.computed && name.is_none() {
                    return Err(());
                }
                if name.as_deref() == Some("get") {
                    return Ok(Some(prop));
                }
            }
            ObjectPropertyKind::SpreadProperty(_) => return Err(()),
        }
    }
    Ok(None)
}

fn is_supported_object_prop(prop: &ObjectProperty) -> bool {
    if prop.method || prop.shorthand {
        return true;
    }
    // It's a key-value prop or getter
    if prop.kind == PropertyKind::Get {
        // getter
        match &prop.value {
            Expression::FunctionExpression(func) => {
                func.body.as_ref().map(|b| is_non_side_effect_block_stmt(b)).unwrap_or(false)
            }
            _ => false,
        }
    } else {
        true
    }
}

fn is_supported_get_prop(prop: &ObjectProperty) -> bool {
    if prop.method {
        // method shorthand
        match &prop.value {
            Expression::FunctionExpression(func) => {
                func.body.as_ref().map(|b| is_non_side_effect_block_stmt(b)).unwrap_or(false)
            }
            _ => false,
        }
    } else {
        // key-value
        match &prop.value {
            Expression::FunctionExpression(func) => {
                func.body.as_ref().map(|b| is_non_side_effect_block_stmt(b)).unwrap_or(false)
            }
            Expression::ArrowFunctionExpression(arrow) => match &arrow.body {
                ArrowFunctionBody::FunctionBody(body) => is_non_side_effect_block_stmt(body),
                body => body.as_expression().is_some_and(is_non_side_effect_expr),
            },
            _ => false,
        }
    }
}

fn get_prop_return_expr<'a>(prop: &'a ObjectProperty<'a>) -> Option<&'a Expression<'a>> {
    if prop.method {
        match &prop.value {
            Expression::FunctionExpression(func) => {
                func.body.as_ref().and_then(|b| get_block_stmt_return_expr(b))
            }
            _ => None,
        }
    } else {
        match &prop.value {
            Expression::FunctionExpression(func) => {
                func.body.as_ref().and_then(|b| get_block_stmt_return_expr(b))
            }
            Expression::ArrowFunctionExpression(arrow) => match &arrow.body {
                ArrowFunctionBody::FunctionBody(body) => get_block_stmt_return_expr(body),
                body => body.as_expression(),
            },
            _ => None,
        }
    }
}

fn is_non_side_effect_block_stmt(stmt: &FunctionBody) -> bool {
    match get_block_stmt_return_expr(stmt) {
        Some(expr) => is_non_side_effect_expr(expr),
        None => false,
    }
}

fn get_block_stmt_return_expr<'a>(body: &'a FunctionBody<'a>) -> Option<&'a Expression<'a>> {
    if body.statements.len() != 1 {
        return None;
    }
    if let Statement::ReturnStatement(ret) = &body.statements[0] {
        ret.argument.as_ref()
    } else {
        None
    }
}

fn is_non_side_effect_expr(expr: &Expression) -> bool {
    if let Expression::ParenthesizedExpression(paren) = expr {
        return is_non_side_effect_expr(&paren.expression);
    }
    if let Some(member) = expr.as_member_expression() {
        return is_non_side_effect_expr(member.object()) && is_non_side_effect_member_prop(member);
    }
    matches!(
        expr,
        Expression::Identifier(_)
            | Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
    ) || matches!(expr, Expression::ObjectExpression(object) if is_non_side_effect_object(object))
}

fn is_non_side_effect_object(object: &ObjectExpression) -> bool {
    object.properties.iter().all(|property| match property {
        ObjectPropertyKind::ObjectProperty(property) => {
            (!property.computed || is_non_side_effect_expr(property.key.to_expression()))
                && (property.method
                    || matches!(property.kind, PropertyKind::Get | PropertyKind::Set)
                    || is_non_side_effect_expr(&property.value))
        }
        ObjectPropertyKind::SpreadProperty(_) => false,
    })
}

fn is_non_side_effect_member_prop(member: &MemberExpression) -> bool {
    match member {
        MemberExpression::StaticMemberExpression(_)
        | MemberExpression::PrivateFieldExpression(_) => true,
        MemberExpression::ComputedMemberExpression(computed) => {
            is_non_side_effect_expr(&computed.expression)
        }
    }
}

fn get_property_key_name(key: &PropertyKey) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(ident) => Some(ident.name.to_string()),
        PropertyKey::StringLiteral(str_lit) => Some(str_lit.value.to_string_lossy()),
        PropertyKey::NumericLiteral(num) => Some(number_property_name(num.value)),
        _ => None,
    }
}

fn number_property_name(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value.is_infinite() {
        return if value.is_sign_positive() { "Infinity" } else { "-Infinity" }.to_string();
    }
    if value.abs() >= 1e21 || value.abs() < 1e-6 {
        let scientific = format!("{value:e}");
        let Some((mantissa, exponent)) = scientific.split_once('e') else {
            return scientific;
        };
        let Ok(exponent) = exponent.parse::<i32>() else {
            return scientific;
        };
        format!("{mantissa}e{}{exponent}", if exponent >= 0 { "+" } else { "" })
    } else {
        value.to_string()
    }
}

fn unwrap_parenthesized<'a>(mut expr: &'a Expression<'a>) -> &'a Expression<'a> {
    while let Expression::ParenthesizedExpression(paren) = expr {
        expr = &paren.expression;
    }
    expr
}

#[cfg(test)]
#[expect(clippy::unwrap_used)]
mod test {
    use std::cell::RefCell;

    use oxc::allocator::Allocator;

    use super::*;
    use crate::{MediaType, ModuleSpecifier, ParseParams, parse_script};

    struct CjsAnalysisTester {
        analysis: RefCell<CjsAnalysis>,
    }

    impl CjsAnalysisTester {
        pub fn assert_exports(&self, mut values: Vec<&str>) {
            values.sort_unstable();
            let mut analysis = self.analysis.borrow_mut();
            assert_eq!(analysis.exports, values);
            analysis.exports.clear();
        }

        pub fn assert_reexports(&self, mut values: Vec<&str>) {
            values.sort_unstable();
            let mut analysis = self.analysis.borrow_mut();
            assert_eq!(analysis.reexports, values);
            analysis.reexports.clear();
        }

        pub fn assert_empty(&self) {
            let analysis = self.analysis.borrow();
            assert!(analysis.exports.is_empty(), "Had exports: {}", analysis.exports.join(", "));
            assert!(
                analysis.reexports.is_empty(),
                "Had reexports: {}",
                analysis.reexports.join(", ")
            );
        }
    }

    impl Drop for CjsAnalysisTester {
        fn drop(&mut self) {
            // ensures that all values have been asserted for
            if !std::thread::panicking() {
                self.assert_empty();
            }
        }
    }

    fn parse_cjs(source: &str) -> CjsAnalysisTester {
        let allocator = Allocator::default();
        let parsed_source = parse_script(
            &allocator,
            ParseParams {
                specifier: ModuleSpecifier::parse("file:///example.js").unwrap(),
                text: source.into(),
                media_type: MediaType::Cjs,
                capture_tokens: true,
                scope_analysis: false,
                maybe_source_type: None,
            },
        )
        .unwrap();
        let analysis = parsed_source.analyze_cjs();
        CjsAnalysisTester { analysis: RefCell::new(analysis) }
    }

    // Tests ported from https://github.com/nodejs/cjs-module-lexer/blob/main/test/_unit.js
    // MIT License - Copyright (C) 2018-2020 Guy Bedford

    #[test]
    fn esbuild_hint_style() {
        let tester = parse_cjs("0 && (module.exports = {a, b, c}) && __exportStar(require('fs'));");

        tester.assert_exports(vec!["a", "b", "c"]);
        tester.assert_reexports(vec!["fs"]);
    }

    #[test]
    fn getter_opt_outs() {
        let tester = parse_cjs(
            r"
    Object.defineProperty(exports, 'a', {
        enumerable: true,
        get: function () {
          return q.p;
        }
      });
      if (false) {
        Object.defineProperty(exports, 'a', {
          enumerable: false,
          get: function () {
            return dynamic();
          }
        });
      }",
        );

        tester.assert_empty();
    }

    #[test]
    fn typescript_reexports() {
        let tester = parse_cjs(
            r#"
    "use strict";
      function __export(m) {
          for (var p in m) if (!exports.hasOwnProperty(p)) exports[p] = m[p];
      }
      Object.defineProperty(exports, "__esModule", { value: true });
      __export(require("external1"));
      tslib.__export(require("external2"));
      __exportStar(require("external3"));
      tslib1.__exportStar(require("external4"));
      "use strict";
      Object.defineProperty(exports, "__esModule", { value: true });
      var color_factory_1 = require("./color-factory");
      Object.defineProperty(exports, "colorFactory", { enumerable: true, get: function () { return color_factory_1.colorFactory; }, });
    "#,
        );

        tester.assert_exports(vec!["__esModule", "colorFactory"]);
        tester.assert_reexports(vec!["external1", "external2", "external3", "external4"]);
    }

    #[test]
    fn typescript_reexports_via_bound_ident() {
        let tester = parse_cjs(
            r#"
    "use strict";
      var reexported_1 = require("./reexported");
      __export(reexported_1);
    "#,
        );

        tester.assert_reexports(vec!["./reexported"]);
    }

    #[test]
    fn typescript_enum_exports() {
        let tester = parse_cjs(
            r#"
    "use strict";
      var MyEnum;
      (function (MyEnum) {
        MyEnum[MyEnum["A"] = 0] = "A";
        MyEnum[MyEnum["B"] = 1] = "B";
      })(MyEnum || (MyEnum = {}));
      exports.MyEnum = MyEnum;
      exports.Outer = (exports.Inner = {});
    "#,
        );

        tester.assert_exports(vec!["Inner", "MyEnum", "Outer"]);
    }

    #[test]
    fn nested_exports_assignment_in_expression() {
        let tester = parse_cjs(
            r#"
    "use strict";
      exports.A = (exports.B = (exports.C = {}));
    "#,
        );

        tester.assert_exports(vec!["A", "B", "C"]);
    }

    #[test]
    fn shadowed_bindings_are_not_commonjs_globals() {
        let tester = parse_cjs(
            r#"
      const dep = require("real");
      function shadowed(require, dep, exports, module) {
        const local = require("fake");
        __export(dep);
        exports.notAnExport = 1;
        module.exports = { notAReplacement: 1 };
        __export(local);
      }
      __export(dep);
    "#,
        );

        tester.assert_exports(vec![]);
        tester.assert_reexports(vec!["real"]);
    }

    #[test]
    fn parenthesized_export_rhs_and_require_are_detected() {
        let tester = parse_cjs(
            r#"
      module.exports = (require("./dependency"));
      exports.value = ((1));
      module.exports = (({ objectKey: true }));
    "#,
        );

        tester.assert_exports(vec!["objectKey", "value"]);
        tester.assert_reexports(vec!["./dependency"]);
    }

    #[test]
    fn compound_assignments_still_visit_nested_exports() {
        let tester = parse_cjs("exports.answer += (exports.nested = 42);");
        tester.assert_exports(vec!["nested"]);
    }

    #[test]
    fn only_module_exports_reassignment_replaces_the_reexport_target() {
        let tester = parse_cjs(
            r#"
      module.exports = require("./original");
      exports = require("./alias");
    "#,
        );
        tester.assert_reexports(vec!["./original"]);

        let tester = parse_cjs("module.exports = exports = require('./chained');");
        tester.assert_reexports(vec!["./chained"]);
    }

    #[test]
    fn computed_member_reexports_require_key_enumeration() {
        let tester = parse_cjs(
            r#"
      const dependency = require("./dependency");
      exports[key] = dependency[key];
      Object.defineProperty(exports, key, {
        enumerable: true,
        get: () => dependency[key]
      });
      exports.named = dependency.named;
    "#,
        );
        tester.assert_exports(vec!["named"]);
        tester.assert_reexports(vec![]);

        let tester = parse_cjs(
            r#"
      const dependency = require("./dependency");
      Object.keys(dependency).forEach(key => {
        exports[key] = dependency[key];
      });
    "#,
        );
        tester.assert_reexports(vec!["./dependency"]);

        let tester = parse_cjs(
            r#"
      const dependency = require("./dependency");
      Object.keys(dependency).forEach(key => {
        Object.defineProperty(exports, key, {
          enumerable: true,
          get: () => dependency[key]
        });
      });
    "#,
        );
        tester.assert_reexports(vec!["./dependency"]);

        let tester = parse_cjs(
            r#"
      const dependency = require("./dependency");
      Object.keys(dependency).forEach(key => {
        exports[key] = dependency.one;
      });
      Object.keys(dependency).forEach(key => {
        Object.defineProperty(exports, key, {
          enumerable: true,
          get: () => dependency.one
        });
      });
    "#,
        );
        tester.assert_reexports(vec![]);
        tester.assert_exports(vec![]);
    }

    #[test]
    fn reassigned_require_aliases_are_not_reexports() {
        let tester = parse_cjs(
            r#"
      var dependency = require("./dependency");
      __export(dependency);
      dependency = {};
      __export(dependency);
    "#,
        );
        tester.assert_reexports(vec!["./dependency"]);
    }

    #[test]
    fn assignment_rhs_can_resolve_the_previous_require_alias() {
        let tester = parse_cjs(
            r#"
      var dependency = require("./dependency");
      dependency = __export(dependency);
      __export(dependency);
    "#,
        );
        tester.assert_reexports(vec!["./dependency"]);
    }

    #[test]
    fn var_redeclaration_initializer_can_resolve_the_previous_alias() {
        let tester = parse_cjs(
            r#"
      var dependency = require("./dependency");
      var dependency = __export(dependency);
      __export(dependency);
    "#,
        );
        tester.assert_reexports(vec!["./dependency"]);
    }

    #[test]
    fn numeric_export_keys_are_stringified() {
        let tester = parse_cjs(
            r#"
      module.exports = { 1: a, 2.5: b, 1e21: c, 1e-7: d };
      exports[3] = c;
      exports["4"] = d;
    "#,
        );

        tester.assert_exports(vec!["1", "1e+21", "1e-7", "2.5", "3", "4"]);
    }

    #[test]
    fn object_literal_proto_setter_is_not_an_export() {
        let tester = parse_cjs("module.exports = { __proto__: prototype, actual: value };");

        tester.assert_exports(vec!["actual"]);
    }

    #[test]
    fn last_effective_getter_controls_unsafe_export_filtering() {
        let tester = parse_cjs(
            r#"
      Object.defineProperty(exports, "safe", {
        get: () => dynamic(),
        get: () => dependency.value
      });
      Object.defineProperty(exports, "unsafe", {
        get: () => dependency.value,
        get: () => dynamic()
      });
    "#,
        );

        tester.assert_exports(vec!["safe"]);
    }

    #[test]
    fn unsafe_getter_exclusions_apply_after_local_export_analysis() {
        let tester = parse_cjs(
            r#"
      Object.defineProperty(exports, "unsafe", {
        get: () => dynamic()
      });
      exports.unsafe = 1;
      module.exports = { unsafe: 2, safe: 3 };
      Object.defineProperty(exports, "./dependency", {
        get: () => dynamic()
      });
      module.exports = require("./dependency");
    "#,
        );
        tester.assert_exports(vec!["safe"]);
        tester.assert_reexports(vec!["./dependency"]);
    }

    #[test]
    fn getter_safety_checks_object_initializers_and_dynamic_overrides() {
        let tester = parse_cjs(
            r#"
      Object.defineProperty(exports, "safe", {
        get: () => ({ value: 1 })
      });
      Object.defineProperty(exports, "unsafeValue", {
        get: () => ({ value: dynamic() })
      });
      Object.defineProperty(exports, "unsafeKey", {
        get: () => ({ [dynamic()]: 1 })
      });
      Object.defineProperty(exports, "unsafeSpread", {
        get: () => dependency.value,
        ...overrides
      });
      Object.defineProperty(exports, "unsafeDynamicKey", {
        get: () => dependency.value,
        [getKey()]: descriptor
      });
    "#,
        );
        tester.assert_exports(vec!["safe"]);
    }

    #[test]
    fn rollup_babel_reexport_getter() {
        let tester = parse_cjs(
            r#"
      Object.defineProperty(exports, 'a', {
        enumerable: true,
        get: function () {
          return q.p;
        }
      });
      Object.defineProperty(exports, 'b', {
        enumerable: false,
        get: function () {
          return q.p;
        }
      });
      Object.defineProperty(exports, "c", {
        get: function get () {
          return q['p' ];
        }
      });
      Object.defineProperty(exports, 'd', {
        get: function () {
          return __ns.val;
        }
      });
      Object.defineProperty(exports, 'e', {
        get () {
          return external;
        }
      });
      Object.defineProperty(exports, "f", {
        get: function get () {
          return q['p' ];
        }
      });
    "#,
        );

        tester.assert_exports(vec![
            "a", "c", "d", "e", "f", // changed code from cjs-module-lexer tests so it parses
            // added support -- why does cjs-module-lexer match c and d and not b?
            "b",
        ]);
    }

    #[test]
    #[expect(clippy::too_many_lines)]
    fn rollup_babel_reexports() {
        let tester = parse_cjs(
            r#"
    "use strict";
      exports.__esModule = true;
      not.detect = require("ignored");
      var _external = require("external");
      // Babel <7.12.0, loose mode
      Object.keys(_external).forEach(function (key) {
        if (key === "default" || key === "__esModule") return;
        exports[key] = _external[key];
      });
      var _external2 = require("external2");
      // Babel <7.12.0
      Object.keys(_external2).forEach(function (key) {
        if (key === "default" || /*comment!*/ key === "__esModule") return;
        Object.defineProperty(exports, key, {
          enumerable: true,
          get: function () {
            return _external2[key];
          }
        });
      });
      var _external001 = require("external001");
      // Babel >=7.12.0, loose mode
      Object.keys(_external001).forEach(function (key) {
        if (key === "default" || key === "__esModule") return;
        if (key in exports && exports[key] === _external001[key]) return;
        exports[key] = _external001[key];
      });
      var _external003 = require("external003");
      // Babel >=7.12.0, loose mode, reexports conflicts filter
      Object.keys(_external003).forEach(function (key) {
        if (key === "default" || key === "__esModule") return;
        if (Object.prototype.hasOwnProperty.call(_exportNames, key)) return;
        if (key in exports && exports[key] === _external003[key]) return;
        exports[key] = _external003[key];
      });
      var _external002 = require("external002");
      // Babel >=7.12.0
      Object.keys(_external002).forEach(function (key) {
        if (key === "default" || key === "__esModule") return;
        if (key in exports && exports[key] === _external002[key]) return;
        Object.defineProperty(exports, key, {
          enumerable: true,
          get: function () {
            return _external002[key];
          }
        });
      });
      var _external004 = require("external004");
      // Babel >=7.12.0, reexports conflict filter
      Object.keys(_external004).forEach(function (key) {
        if (key === "default" || key === "__esModule") return;
        if (Object.prototype.hasOwnProperty.call(_exportNames, key)) return;
        if (key in exports && exports[key] === _external004[key]) return;
        Object.defineProperty(exports, key, {
          enumerable: true,
          get: function () {
            return _external004[key];
          }
        });
      });
      let external3 = require('external3');
      const external4 = require('external4');
      Object.keys(external3).forEach(function (k) {
        if (k !== 'default') Object.defineProperty(exports, k, {
          enumerable: true,
          get: function () {
            return external3[k];
          }
        });
      });
      Object.keys(external4).forEach(function (k) {
        if (k !== 'default') exports[k] = external4[k];
      });
      const externalǽ = require('external😃');
      Object.keys(externalǽ).forEach(function (k) {
        if (k !== 'default') exports[k] = externalǽ[k];
      });
      let external5 = require('e5');
      let external6 = require('e6');
      Object.keys(external5).forEach(function (k) {
        if (k !== 'default' && !Object.hasOwnProperty.call(exports, k)) exports[k] = external5[k];
      });
      const not = require('not');
      Object.keys(not).forEach(function (k) {
        if (k !== 'default' && !a().hasOwnProperty(k)) exports[k] = not[k];
      });
      Object.keys(external6).forEach(function (k) {
        if (k !== 'default' && !exports.hasOwnProperty(k)) exports[k] = external6[k];
      });
      const external𤭢 = require('external𤭢');
      Object.keys(external𤭢).forEach(function (k) {
        if (k !== 'default') exports[k] = external𤭢[k];
      });
      const notexternal1 = require('notexternal1');
      Object.keys(notexternal1);
      const notexternal2 = require('notexternal2');
      Object.keys(notexternal2).each(function(){
      });
      const notexternal3 = require('notexternal3');
      Object.keys(notexternal2).forEach(function () {
      });
      const notexternal4 = require('notexternal4');
      Object.keys(notexternal2).forEach(function (x) {
      });
      const notexternal5 = require('notexternal5');
      Object.keys(notexternal5).forEach(function (x) {
        if (true);
      });
      const notexternal6 = require('notexternal6');
      Object.keys(notexternal6).forEach(function (x) {
        if (x);
      });
      const notexternal7 = require('notexternal7');
      Object.keys(notexternal7).forEach(function(x){
        if (x ==='default');
      });
      const notexternal8 = require('notexternal8');
      Object.keys(notexternal8).forEach(function(x){
        if (x ==='default'||y);
      });
      const notexternal9 = require('notexternal9');
      Object.keys(notexternal9).forEach(function(x){
        if (x ==='default'||x==='__esM');
      });
      const notexternal10 = require('notexternal10');
      Object.keys(notexternal10).forEach(function(x){
        if (x !=='default') return
      });
      const notexternal11 = require('notexternal11');
      Object.keys(notexternal11).forEach(function(x){
        if (x ==='default'||x==='__esModule') return
      });
      const notexternal12 = require('notexternal12');
      Object.keys(notexternal12).forEach(function(x){
        if (x ==='default'||x==='__esModule') return
        exports[y] = notexternal12[y];
      });
      const notexternal13 = require('notexternal13');
      Object.keys(notexternal13).forEach(function(x){
        if (x ==='default'||x==='__esModule') return
        exports[y] = notexternal13[y];
      });
      const notexternal14 = require('notexternal14');
      Object.keys(notexternal14).forEach(function(x){
        if (x ==='default'||x==='__esModule') return
        Object.defineProperty(exports, k, {
          enumerable: false,
          get: function () {
            return external14[k];
          }
        });
      });
      const notexternal15 = require('notexternal15');
      Object.keys(notexternal15).forEach(function(x){
        if (x ==='default'||x==='__esModule') return
        Object.defineProperty(exports, k, {
          enumerable: false,
          get: function () {
            return externalnone[k];
          }
        });
      });
      const notexternal16 = require('notexternal16');
      Object.keys(notexternal16).forEach(function(x){
        if (x ==='default'||x==='__esModule') return
        exports[x] = notexternal16[x];
        extra;
      });
      {
        const notexternal17 = require('notexternal17');
        Object.keys(notexternal17).forEach(function(x){
          if (x ==='default'||x==='__esModule') return
          exports[x] = notexternal17[x];
        });
      }
      var _styles = require("./styles");
      Object.keys(_styles).forEach(function (key) {
        if (key === "default" || key === "__esModule") return;
        if (Object.prototype.hasOwnProperty.call(_exportNames, key)) return;
        Object.defineProperty(exports, key, {
          enumerable: true,
          get: function get() {
            return _styles[key];
          }
        });
      });
      var _styles2 = require("./styles2");
      Object.keys(_styles2).forEach(function (key) {
        if (key === "default" || key === "__esModule") return;
        if (Object.prototype.hasOwnProperty.call(_exportNames, key)) return;
        Object.defineProperty(exports, key, {
          enumerable: true,
          get () {
            return _styles2[key];
          }
        });
      });
      var _Accordion = _interopRequireWildcard(require("./Accordion"));
      Object.keys(_Accordion).forEach(function (key) {
        if (key === "default" || key === "__esModule") return;
        if (Object.prototype.hasOwnProperty.call(_exportNames, key)) return;
        Object.defineProperty(exports, key, {
          enumerable: true,
          get: function () {
            return _Accordion[key];
          }
        });
      });
    "#,
        );

        tester.assert_exports(vec!["__esModule"]);
        tester.assert_reexports(vec![
            "external",
            "external2",
            "external001",
            "external003",
            "external002",
            "external004",
            "external3",
            "external4",
            "external😃",
            "e5",
            "e6",
            "external𤭢",
            "./styles",
            "./styles2",
            "./Accordion",
            // extra values not matched in cjs-module-lexer
            "not",           // why not?
            "notexternal16", // might as well match
            "notexternal17", // might as well match
        ]);
    }

    #[test]
    fn module_exports_reexport_spread() {
        let tester = parse_cjs(
            r"
    module.exports = {
        ...a,
        ...b,
        ...require('dep1'),
        c: d,
        ...require('dep2'),
        name
      };
    ",
        );

        tester.assert_exports(vec!["c", "name"]);
        tester.assert_reexports(vec!["dep1", "dep2"]);
    }

    #[test]
    fn shebang() {
        parse_cjs(r"#!");

        let tester = parse_cjs(
            r"#! (  {
        exports.asdf = 'asdf';
      ",
        );

        tester.assert_exports(vec!["asdf"]);
    }

    #[test]
    fn non_identifiers() {
        let tester = parse_cjs(
            r#"
      module.exports = { 'ab cd': foo };
      exports['not identifier'] = 'asdf';
      exports['\''] = 1;
      exports['@notidentifier'] = 'asdf';
      Object.defineProperty(exports, "%notidentifier", { value: x });
      Object.defineProperty(exports, 'hm🤔', { value: x });
      exports['⨉'] = 45;
      exports['α'] = 54;
      exports.package = 'STRICT RESERVED!';
      exports.var = 'RESERVED';
    "#,
        );

        tester.assert_exports(vec![
            "%notidentifier",
            "ab cd",
            "not identifier",
            "\'",
            "@notidentifier",
            "hm🤔",
            "⨉",
            "α",
            "package",
            "var",
        ]);
    }

    #[test]
    fn literal_exports() {
        let tester = parse_cjs(r"module.exports = { a, b: c, d, 'e': f };");

        tester.assert_exports(vec!["a", "b", "d", "e"]);
    }

    #[test]
    fn literal_exports_example() {
        let tester = parse_cjs(
            r"
      module.exports = {
        a: a,
        b: b,
        // cjs-module-lexer bails on the object literal
        // after this property because it encounters require
        e: require('d'),
        f: 'f'
      }
    ",
        );

        tester.assert_exports(vec!["a", "b", "e", "f"]);
    }

    #[test]
    fn literal_exports_complex() {
        let tester = parse_cjs(
            r#"
      function defineProp(name, value) {
        delete module.exports[name];
        module.exports[name] = value;
        return value;
      }

      module.exports = {
        Parser: Parser,
        Tokenizer: require("./Tokenizer.js"),
        ElementType: require("domelementtype"),
        DomHandler: DomHandler,
        get FeedHandler() {
            return defineProp("FeedHandler", require("./FeedHandler.js"));
        },
        get Stream() {
            return defineProp("Stream", require("./Stream.js"));
        },
        get WritableStream() {
            return defineProp("WritableStream", require("./WritableStream.js"));
        },
        get ProxyHandler() {
            return defineProp("ProxyHandler", require("./ProxyHandler.js"));
        },
        get DomUtils() {
            return defineProp("DomUtils", require("domutils"));
        },
        get CollectingHandler() {
            return defineProp(
                "CollectingHandler",
                require("./CollectingHandler.js")
            );
        },
        // For legacy support
        DefaultHandler: DomHandler,
        get RssHandler() {
            return defineProp("RssHandler", this.FeedHandler);
        },
        //helper methods
        parseDOM: function(data, options) {
            var handler = new DomHandler(options);
            new Parser(handler, options).end(data);
            return handler.dom;
        },
        parseFeed: function(feed, options) {
            var handler = new module.exports.FeedHandler(options);
            new Parser(handler, options).end(feed);
            return handler.dom;
        },
        createDomStream: function(cb, options, elementCb) {
            var handler = new DomHandler(cb, options, elementCb);
            return new Parser(handler, options);
        },
        // List of all events that the parser emits
        EVENTS: {
            /* Format: eventname: number of arguments */
            attribute: 2,
            cdatastart: 0,
            cdataend: 0,
            text: 1,
            processinginstruction: 2,
            comment: 1,
            commentend: 0,
            closetag: 1,
            opentag: 2,
            opentagname: 1,
            error: 1,
            end: 0
        }
      };
    "#,
        );

        #[rustfmt::skip]
    tester.assert_exports(
      vec![
        "Parser",
        "Tokenizer",
        // added support for below - cjs-module-lexer bails early because it encountered,
        // a require expression, but we can understand this
        "ElementType",
        "DefaultHandler",
        "DomHandler",
        "EVENTS",
        "createDomStream",
        "parseDOM",
        "parseFeed",
      ]
    );
    }

    #[test]
    fn define_property_value() {
        let tester = parse_cjs(
            r#"
      Object.defineProperty(exports, 'namedExport', { enumerable: false, value: true });
      Object.defineProperty(exports, 'namedExport', { configurable: false, value: true });
      Object.defineProperty(exports, 'a', {
        enumerable: false,
        get () {
          return p;
        }
      });
      Object.defineProperty(exports, 'b', {
        configurable: true,
        get () {
          return p;
        }
      });
      Object.defineProperty(exports, 'c', {
        get: () => p
      });
      Object.defineProperty(exports, 'd', {
        enumerable: true,
        get: function () {
          return dynamic();
        }
      });
      Object.defineProperty(exports, 'e', {
        enumerable: true,
        get () {
          return 'str';
        }
      });
      Object.defineProperty(module.exports, 'thing', { value: true });
      Object.defineProperty(exports, "other", { enumerable: true, value: true });
      Object.defineProperty(exports, "__esModule", { value: true });
    "#,
        );

        tester.assert_exports(vec![
            "__esModule",
            "other",
            "thing",
            // added support for below
            "a",
            "b",
            "c",
            "e",
            "namedExport",
        ]);
    }

    #[test]
    fn module_assign() {
        let tester = parse_cjs(
            r#"
      module.exports.asdf = 'asdf';
      exports = 'asdf';
      module.exports = require('./asdf');
      if (maybe)
        module.exports = require("./another");
    "#,
        );

        tester.assert_exports(vec!["asdf"]);
        tester.assert_reexports(vec!["./another"]);
    }

    #[test]
    fn template_string_expression_ambiguity() {
        let tester = parse_cjs(
            r"
      `$`
      import('a');
      ``
      exports.a = 'a';
      `a$b`
      exports['b'] = 'b';
      `{$}`
      exports['b'].b;
    ",
        );

        tester.assert_exports(vec!["a", "b"]);
    }

    #[test]
    fn ts_4_4() {
        // cjs-module-lexer does not support this, but we might as well
        // This is how TS 4.4 started outputting, but was fixed in TS 4.6.
        let tester = parse_cjs(
            r#"
      (0, tslib_1.__exportStar)(require("./foo"), exports);
      "#,
        );

        tester.assert_reexports(vec!["./foo"]);
    }

    #[test]
    fn multiple_assigns_in_statement() {
        let tester = parse_cjs(
            r"
      exports.x = exports.extract = require('./lib/extract.js')
      ",
        );

        tester.assert_exports(vec!["x", "extract"]);
    }

    #[test]
    fn multiple_assigns_before_module_exports() {
        let tester = parse_cjs(
            r"
        (function(){
          const BigInteger = 1;
          const SecureRandom = 2;
          if (typeof exports !== 'undefined') {
            a= window.something = exports = module.exports = {
                  default: BigInteger,
                  BigInteger: BigInteger,
                  SecureRandom: SecureRandom,
              };
          }
        }).call(this)
    ",
        );
        tester.assert_exports(vec!["BigInteger", "SecureRandom", "default"]);
    }

    #[test]
    fn chain_assigns_after_module_exports() {
        let tester = parse_cjs(
            r"
      module.exports = a = b = exports = {
        a: 1,
        b: 2,
        c,
      }
    ",
        );
        tester.assert_exports(vec!["a", "b", "c"]);
    }
}
