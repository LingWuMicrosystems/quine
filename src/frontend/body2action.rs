use alloc::vec::Vec;

use crate::{
    common::{Map, TableName, VarId},
    core::{
        regraph::TableId,
        rule::{self, Action, ActionTail, ValueOrVariable, VariableRecord},
    },
    frontend::{
        error::CompileError,
        head2flat_clause::NameOrVariable,
        syntax::{AtomOrVariable, Body, Expr, FunctionCall, VarName},
    },
};

pub fn bodys2action(
    bodys: &[Body],
    table_map: &Map<TableName, TableId>,
    var_record: &VariableRecord,
) -> Result<Action, CompileError> {
    let mut lets = Vec::default();
    let mut lets_map = Map::default();
    let mut tails = Vec::default();
    for body in bodys {
        if let Some(tail) = body2action(body, table_map, var_record, &mut lets, &mut lets_map)? {
            tails.push(tail);
        }
    }
    let tail = tails.into_boxed_slice();
    Ok(Action {
        lets_map,
        lets: lets.into_boxed_slice(),
        tail,
    })
}

fn body2action(
    body: &Body,
    table_map: &Map<TableName, TableId>,
    var_record: &VariableRecord,
    lets: &mut Vec<rule::FunctionCall>,
    lets_map: &mut Map<VarName, VarId>,
) -> Result<Option<ActionTail>, CompileError> {
    match body {
        Body::Let(var_name, function_call) => {
            let call =
                function_call_transform(function_call, table_map, var_record, lets, lets_map)?;
            let id = lets.len();
            lets_map.insert(var_name.clone(), VarId(id));
            lets.push(call);
            Ok(None)
        }
        Body::Insert(function_call, expr) => {
            let call =
                function_call_transform(function_call, table_map, var_record, lets, lets_map)?;
            let table_id = call.offset;
            let args = call.args.clone();
            lets.push(call.clone());
            let expr = if let Some(expr) = expr {
                Some(expr_transform(expr, table_map, var_record, lets, lets_map)?)
            } else {
                None
            };
            Ok(Some(ActionTail::Insert(table_id, args, expr)))
        }
        Body::Union(expr, expr1) => {
            let id1 = expr_transform(expr, table_map, var_record, lets, lets_map)?;
            let id2 = expr_transform(expr1, table_map, var_record, lets, lets_map)?;
            Ok(Some(ActionTail::Union(id1, id2)))
        }
    }
}

pub fn function_call_transform(
    call: &FunctionCall,
    table_map: &Map<TableName, TableId>,
    var_record: &VariableRecord,
    lets: &mut Vec<rule::FunctionCall>,
    lets_map: &Map<VarName, VarId>,
) -> Result<rule::FunctionCall, CompileError> {
    let args = call
        .1
        .iter()
        .flat_map(|expr| expr_transform(expr, table_map, var_record, lets, lets_map))
        .collect();

    let r = rule::FunctionCall {
        is_native: false,
        offset: table_map
            .get(&TableName(call.0.clone()))
            .cloned()
            .ok_or_else(|| CompileError::InvalidTableName(TableName(call.0.clone())))?,
        args,
    };
    Ok(r)
}

fn expr_transform(
    expr: &Expr,
    table_map: &Map<TableName, TableId>,
    var_record: &VariableRecord,
    lets: &mut Vec<rule::FunctionCall>,
    lets_map: &Map<VarName, VarId>,
) -> Result<rule::ValueOrVariable, CompileError> {
    match expr {
        Expr::AtomOrVariable(a) => atom_or_variable_transform(a, var_record, lets_map),
        Expr::FunctionCall(function_call) => {
            let call =
                function_call_transform(function_call, table_map, var_record, lets, lets_map)?;
            let id = lets.len();
            lets.push(call);
            Ok(ValueOrVariable::Variable(VarId(id)))
        }
    }
}

fn atom_or_variable_transform(
    a: &AtomOrVariable,
    var_record: &VariableRecord,
    lets_map: &Map<VarName, VarId>,
) -> Result<rule::ValueOrVariable, CompileError> {
    match a {
        AtomOrVariable::Atom(a) => Ok(ValueOrVariable::Value(a.clone().to_value())),
        AtomOrVariable::Variable(v) => {
            let id = var_record
                .iter()
                .enumerate()
                .find_map(|(offset, (_, var))| {
                    if let Some(id) = var
                        && id == v
                    {
                        Some(VarId(offset))
                    } else {
                        None
                    }
                })
                .or_else(|| lets_map.get(v).cloned())
                .ok_or_else(|| {
                    CompileError::InvalidVariableName(NameOrVariable::Name(v.clone()))
                })?;
            Ok(ValueOrVariable::Variable(id))
        }
    }
}
