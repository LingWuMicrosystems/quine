use alloc::vec::Vec;

use crate::{
    common::{Map, Variable},
    core::rule::{Action, VariableRecord},
    frontend::syntax::{Body, FunctionCall, VarName},
    types::BaseType,
};

pub fn body2action(body: &[Body], var_record: &VariableRecord) -> Action {
    // match body {
    //     Body::Let(_, function_call) => todo!(),
    //     Body::Insert(function_call, expr) => todo!(),
    //     Body::Union(expr, expr1) => todo!(),
    // }
    todo!()
}

pub fn function_call2action(
    body: &FunctionCall,
    var_record: &Map<VarName, (Variable, BaseType)>,
) -> Vec<Action> {
    todo!()
}
