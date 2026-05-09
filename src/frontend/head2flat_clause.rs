use alloc::{format, vec, vec::Vec};

use crate::{
    common::{Atom, ColumnIndex, TableName},
    frontend::{
        error::CompileError,
        syntax::{AtomOrVariable, Expr, FunctionCall, Head, Op, VarName},
        utils::AnonymousVarCounter,
    },
};

#[derive(Debug, Clone)]
pub enum FlatClause<V> {
    Lookup(V, TableName, ColumnIndex),
    ConstCompare(Op, V, Atom),
    Guard(Op, V, V),
}

pub fn heads2flat_clause(heads: &[Head]) -> Result<Vec<FlatClause<VarName>>, CompileError> {
    let mut clauses = vec![];
    let mut counter = AnonymousVarCounter::default();
    for head in heads.iter() {
        counter = head2flat_clause(head, &mut clauses, counter);
    }
    Ok(clauses)
}

fn head2flat_clause(
    head: &Head,
    clauses: &mut Vec<FlatClause<VarName>>,
    counter: AnonymousVarCounter,
) -> AnonymousVarCounter {
    match head {
        Head::Match(pattern) => {
            let (new_counter, _) = function_call2flat_clause(&pattern, clauses, counter);
            return new_counter;
        }
        Head::LetEq(expr, expr1) => {
            let (new_counter, expr) = expr2flat_clause(expr, clauses, counter);
            let (new_counter, expr1) = expr2flat_clause(expr1, clauses, new_counter);
            clauses.push(FlatClause::Guard(Op::Equ, expr, expr1));
            return new_counter;
        }
        Head::Guard(op, expr, Expr::AtomOrVariable(AtomOrVariable::Atom(a)))
        | Head::Guard(op, Expr::AtomOrVariable(AtomOrVariable::Atom(a)), expr) => {
            let (new_counter, var) = expr2flat_clause(expr, clauses, counter);
            clauses.push(FlatClause::ConstCompare(*op, var, a.clone()));
            return new_counter;
        }
        Head::Guard(op, expr, expr1) => {
            let (new_counter, expr) = expr2flat_clause(expr, clauses, counter);
            let (new_counter, expr1) = expr2flat_clause(expr1, clauses, new_counter);
            clauses.push(FlatClause::Guard(*op, expr, expr1));
            return new_counter;
        }
    }
}

fn function_call2flat_clause(
    call: &FunctionCall,
    clauses: &mut Vec<FlatClause<VarName>>,
    mut counter: AnonymousVarCounter,
) -> (AnonymousVarCounter, VarName) {
    for i in 0..call.1.len() {
        let (new_counter, var) = expr2flat_clause(&call.1[i], clauses, counter);
        clauses.push(FlatClause::Lookup(
            var,
            TableName(call.0.clone()),
            ColumnIndex(i),
        ));
        counter = new_counter;
    }
    let out = format!("t_{}", counter.0);
    (counter.next(), out)
}

fn expr2flat_clause(
    expr: &Expr,
    clauses: &mut Vec<FlatClause<VarName>>,
    counter: AnonymousVarCounter,
) -> (AnonymousVarCounter, VarName) {
    match expr {
        Expr::AtomOrVariable(AtomOrVariable::Atom(a)) => {
            let name = format!("t_{}", counter.0);
            clauses.push(FlatClause::ConstCompare(Op::Equ, name.clone(), a.clone()));
            (counter.next(), name)
        }
        Expr::AtomOrVariable(AtomOrVariable::Variable(v)) => (counter, v.clone()),
        Expr::FunctionCall(call) => function_call2flat_clause(&call, clauses, counter),
    }
}
