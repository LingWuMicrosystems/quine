use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::{
    engine::error::CompileError,
    engine::frontend::syntax::{AtomOrVariable, Body, Expr, FunctionCall},
    regraph::common::{Map, TableName, VarId},
    regraph::types::{BaseType, Type},
    regraph::{
        related_egraph::TableId,
        rule::{self, Action, ActionTail, ValueOrVariable, VariableRecord},
    },
};

pub fn bodys2action(
    bodys: &[Body],
    table_map: &Map<TableName, TableId>,
    head_variables: &VariableRecord,
) -> Result<Action, CompileError> {
    let mut lets = Vec::default();
    let mut tails = Vec::default();
    let mut variables = VariableRecord::default();
    for body in bodys {
        if let Some(tail) = body2action(body, table_map, head_variables, &mut variables, &mut lets)?
        {
            tails.push(tail);
        }
    }
    let tail = tails.into_boxed_slice();
    Ok(Action {
        // lets_map,
        lets: lets.into_boxed_slice(),
        tail,
    })
}

fn body2action(
    body: &Body,
    table_map: &Map<TableName, TableId>,
    head_variables: &VariableRecord,
    variables: &mut VariableRecord,
    lets: &mut Vec<rule::FunctionCall>,
) -> Result<Option<ActionTail>, CompileError> {
    match body {
        Body::Let(var_name, function_call) => {
            function_call_transform(function_call, table_map, head_variables, variables, lets)?;
            // TODO: type check
            variables.insert_var(Some(var_name.clone()), Type::Base(BaseType::Id));
            Ok(None)
        }
        Body::Insert(function_call, expr) => {
            let table_id = table_map
                .get(&function_call.0)
                .ok_or_else(|| CompileError::InvalidTableName(function_call.0.clone()))?;
            let args = function_call
                .1
                .iter()
                .map(|arg| -> Result<ValueOrVariable, CompileError> {
                    expr_transform(arg, table_map, head_variables, variables, lets)
                })
                .collect::<Result<Box<[ValueOrVariable]>, CompileError>>()?;
            let expr = if let Some(expr) = expr {
                Some(expr_transform(
                    expr,
                    table_map,
                    head_variables,
                    variables,
                    lets,
                )?)
            } else {
                None
            };
            Ok(Some(ActionTail::Insert(*table_id, args, expr)))
        }
        Body::Union(expr, expr1) => {
            let id1 = expr_transform(expr, table_map, head_variables, variables, lets)?;
            let id2 = expr_transform(expr1, table_map, head_variables, variables, lets)?;
            Ok(Some(ActionTail::Union(id1, id2)))
        }
    }
}

pub fn function_call_transform(
    call: &FunctionCall,
    table_map: &Map<TableName, TableId>,
    head_variables: &VariableRecord,
    variables: &mut VariableRecord,
    lets: &mut Vec<rule::FunctionCall>,
) -> Result<VarId, CompileError> {
    let args = call
        .1
        .iter()
        .flat_map(|expr| expr_transform(expr, table_map, head_variables, variables, lets))
        .collect();

    let f = rule::FunctionCall {
        is_native: false,
        offset: table_map
            .get(&call.0)
            .cloned()
            .ok_or_else(|| CompileError::InvalidTableName(call.0.clone()))?,
        args,
    };
    // type check
    let r = variables.insert_var(None, Type::Base(BaseType::Id));
    lets.push(f);
    Ok(r)
}

fn expr_transform(
    expr: &Expr,
    table_map: &Map<TableName, TableId>,
    head_variables: &VariableRecord,
    variables: &mut VariableRecord,
    lets: &mut Vec<rule::FunctionCall>,
) -> Result<rule::ValueOrVariable, CompileError> {
    match expr {
        Expr::AtomOrVariable(a) => atom_or_variable_transform(a, head_variables, variables),
        Expr::FunctionCall(function_call) => Ok(ValueOrVariable::Variable(
            function_call_transform(function_call, table_map, head_variables, variables, lets)?,
        )),
    }
}

fn atom_or_variable_transform(
    a: &AtomOrVariable,
    head_variables: &VariableRecord,
    variables: &mut VariableRecord,
) -> Result<rule::ValueOrVariable, CompileError> {
    match a {
        AtomOrVariable::Atom(a) => Ok(ValueOrVariable::Value(a.clone().to_value())),
        AtomOrVariable::Variable(v) => {
            if let Some(offset) = head_variables.get_offset(v) {
                Ok(ValueOrVariable::Variable(VarId(offset)))
            } else if let Some(id) = variables.get_offset(v) {
                Ok(ValueOrVariable::Variable(VarId(id)))
            } else {
                Err(CompileError::VariableNotDefine(v.clone()))
            }
        }
    }
}
