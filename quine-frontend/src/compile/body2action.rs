use alloc::{boxed::Box, string::String, vec::Vec};
use quine_core::{
    common::*,
    related_egraph::*,
    rule::{self, *},
    types::{BaseType, Type},
};

use crate::{
    NativeSignature,
    compile::atom_to_value,
    error::CompileError,
    interner::Interner,
    syntax::{AtomOrVariable, Body, Expr, FunctionCall},
};

pub struct CompileCtx<'a> {
    pub table_map: &'a Map<String, TableId>,
    pub head_variables: &'a VariableRecord,
    pub variables: VariableRecord,
    pub lets: Vec<rule::FunctionCall>,
    pub interner: &'a mut Interner,
    pub native_names: &'a Map<String, usize>,
    pub native_signatures: &'a Map<String, NativeSignature>,
}

pub fn bodys2action(ctx: &mut CompileCtx, bodys: &[Body]) -> Result<Action, CompileError> {
    let mut tails = Vec::default();
    for body in bodys {
        if let Some(tail) = body2action(ctx, body)? {
            tails.push(tail);
        }
    }
    Ok(Action {
        lets: ctx.lets.drain(..).collect(),
        tail: tails.into_boxed_slice(),
    })
}

fn body2action(ctx: &mut CompileCtx, body: &Body) -> Result<Option<ActionTail>, CompileError> {
    match body {
        Body::Let(_, var_name, function_call) => {
            function_call_transform(ctx, function_call)?;
            ctx.variables
                .insert_var(Some(var_name.clone()), Type::Base(BaseType::Id));
            Ok(None)
        }
        Body::Insert(_, function_call, expr) => {
            let table_id = ctx
                .table_map
                .get(&function_call.0)
                .ok_or_else(|| CompileError::InvalidTableName(function_call.0.clone()))?;
            let args = function_call
                .1
                .iter()
                .map(|arg| expr_transform(ctx, arg))
                .collect::<Result<Box<[ValueOrVariable]>, _>>()?;
            let expr = if let Some(expr) = expr {
                Some(expr_transform(ctx, expr)?)
            } else {
                None
            };
            Ok(Some(ActionTail::Insert(*table_id, args, expr)))
        }
        Body::Union(_, expr, expr1) => {
            let id1 = expr_transform(ctx, expr)?;
            let id2 = expr_transform(ctx, expr1)?;
            Ok(Some(ActionTail::Union(id1, id2)))
        }
    }
}

pub fn function_call_transform(
    ctx: &mut CompileCtx,
    call: &FunctionCall,
) -> Result<VarId, CompileError> {
    let args = call
        .1
        .iter()
        .flat_map(|expr| expr_transform(ctx, expr))
        .collect();

    let (f, ret_ty) = if let Some(&offset) = ctx.native_names.get(&call.0) {
        let sig = &ctx.native_signatures[&call.0];
        (
            rule::FunctionCall {
                is_native: true,
                offset,
                args,
            },
            sig.ret.clone(),
        )
    } else {
        let offset = ctx
            .table_map
            .get(&call.0)
            .cloned()
            .ok_or_else(|| CompileError::InvalidTableName(call.0.clone()))?;
        (
            rule::FunctionCall {
                is_native: false,
                offset,
                args,
            },
            BaseType::Id,
        )
    };
    let r = ctx.variables.insert_var(None, Type::Base(ret_ty));
    ctx.lets.push(f);
    Ok(r)
}

fn expr_transform(
    ctx: &mut CompileCtx,
    expr: &Expr,
) -> Result<rule::ValueOrVariable, CompileError> {
    match expr {
        Expr::AtomOrVariable(a) => atom_or_variable_transform(ctx, a),
        Expr::FunctionCall(call) => Ok(ValueOrVariable::Variable(function_call_transform(
            ctx, call,
        )?)),
    }
}

fn atom_or_variable_transform(
    ctx: &mut CompileCtx,
    a: &AtomOrVariable,
) -> Result<rule::ValueOrVariable, CompileError> {
    match a {
        AtomOrVariable::Atom(a) => Ok(ValueOrVariable::Value(atom_to_value(
            a.clone(),
            ctx.interner,
        ))),
        AtomOrVariable::Variable(v) => {
            if let Some(offset) = ctx.head_variables.get_offset(v) {
                Ok(ValueOrVariable::Variable(VarId(offset)))
            } else if let Some(id) = ctx.variables.get_offset(v) {
                Ok(ValueOrVariable::Variable(VarId(id)))
            } else {
                Err(CompileError::VariableNotDefine(v.clone()))
            }
        }
    }
}
