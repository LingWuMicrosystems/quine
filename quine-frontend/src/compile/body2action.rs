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
    compile::unify,
    env::DataTypeEnv,
    error::CompileError,
    interner::Interner,
    syntax::{AtomOrVariable, Body, Expr, FunctionCall},
};
use quine_core::types::TableDef;

pub struct CompileCtx<'a> {
    pub table_map: &'a Map<String, TableId>,
    pub table_defs: &'a [TableDef],
    pub data_types: &'a DataTypeEnv,
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
            let ret_ty = infer_call_type(ctx, function_call)?;
            function_call_transform(ctx, function_call)?;
            ctx.variables
                .insert_var(Some(var_name.clone()), ret_ty);
            Ok(None)
        }
        Body::Insert(_, function_call, expr) => {
            let table_id = ctx
                .table_map
                .get(&function_call.0)
                .ok_or_else(|| CompileError::InvalidTableName(function_call.0.clone()))?;
            let table_def = &ctx.table_defs[*table_id];
            let arity = table_def.1.len().saturating_sub(1);
            if function_call.1.len() != arity {
                return Err(CompileError::InvalidTableWidth(
                    function_call.1.len(),
                    arity,
                ));
            }
            // Type-check key columns
            for (col, arg) in function_call.1.iter().enumerate() {
                let expected = &table_def.1[col];
                let actual = infer_expr_type(ctx, arg)?;
                unify(&actual, expected)?;
            }
            // Type-check result column if explicit
            if let Some(expr) = expr {
                let expected = &table_def.1[arity];
                let actual = infer_expr_type(ctx, expr)?;
                unify(&actual, expected)?;
            }
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
            let t1 = infer_expr_type(ctx, expr)?;
            let t2 = infer_expr_type(ctx, expr1)?;
            unify(&t1, &t2)?;
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
            Type::Base(sig.ret.clone()),
        )
    } else {
        let offset = ctx
            .table_map
            .get(&call.0)
            .cloned()
            .ok_or_else(|| CompileError::InvalidTableName(call.0.clone()))?;
        let ret_ty = ctx
            .data_types
            .get_constructor_type(&call.0)
            .map_or(Type::Base(BaseType::Id), Type::Name);
        (
            rule::FunctionCall {
                is_native: false,
                offset,
                args,
            },
            ret_ty,
        )
    };
    let r = ctx.variables.insert_var(None, ret_ty);
    ctx.lets.push(f);
    Ok(r)
}

fn infer_call_type(
    ctx: &CompileCtx,
    call: &FunctionCall,
) -> Result<Type, CompileError> {
    if ctx.native_names.contains_key(&call.0) {
        let sig = &ctx.native_signatures[&call.0];
        Ok(Type::Base(sig.ret.clone()))
    } else {
        ctx.data_types
            .get_constructor_type(&call.0)
            .map(Type::Name)
            .or_else(|| {
                // Check if it's a relation table (no parent type)
                ctx.table_map
                    .get(&call.0)
                    .map(|_| Type::Base(BaseType::Id))
            })
            .ok_or_else(|| CompileError::InvalidTableName(call.0.clone()))
    }
}

fn infer_expr_type(ctx: &CompileCtx, expr: &Expr) -> Result<Type, CompileError> {
    match expr {
        Expr::AtomOrVariable(AtomOrVariable::Atom(a)) => Ok(Type::Base(a.get_type())),
        Expr::AtomOrVariable(AtomOrVariable::Variable(v)) => {
            if let Some(offset) = ctx.head_variables.get_offset(v) {
                Ok(ctx.head_variables.get_type(offset).unwrap().clone())
            } else if let Some(offset) = ctx.variables.get_offset(v) {
                Ok(ctx.variables.get_type(offset).unwrap().clone())
            } else {
                Err(CompileError::VariableNotDefine(v.clone()))
            }
        }
        Expr::FunctionCall(call) => infer_call_type(ctx, call),
    }
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
