use alloc::{vec, vec::Vec};

use crate::{
    common::{Atom, ColumnIndex, TableName, VarId},
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NameOrVariable {
    Var(VarId),
    Name(VarName),
}

pub fn heads2flat_clause(heads: &[Head]) -> Result<Vec<FlatClause<NameOrVariable>>, CompileError> {
    let mut clauses = vec![];
    let mut counter = AnonymousVarCounter::default();
    for head in heads.iter() {
        counter = head2flat_clause(head, &mut clauses, counter);
    }
    Ok(clauses)
}

fn head2flat_clause(
    head: &Head,
    clauses: &mut Vec<FlatClause<NameOrVariable>>,
    counter: AnonymousVarCounter,
) -> AnonymousVarCounter {
    match head {
        Head::Match(pattern) => {
            let (new_counter, _) = function_call2flat_clause(pattern, clauses, counter);
            new_counter
        }
        Head::LetEq(expr, expr1) => {
            let (new_counter, expr) = expr2flat_clause(expr, clauses, counter);
            let (new_counter, expr1) = expr2flat_clause(expr1, clauses, new_counter);
            clauses.push(FlatClause::Guard(Op::Equ, expr, expr1));
            new_counter
        }
        Head::Guard(op, expr, Expr::AtomOrVariable(AtomOrVariable::Atom(a)))
        | Head::Guard(op, Expr::AtomOrVariable(AtomOrVariable::Atom(a)), expr) => {
            let (new_counter, var) = expr2flat_clause(expr, clauses, counter);
            clauses.push(FlatClause::ConstCompare(*op, var, a.clone()));
            new_counter
        }
        Head::Guard(op, expr, expr1) => {
            let (new_counter, expr) = expr2flat_clause(expr, clauses, counter);
            let (new_counter, expr1) = expr2flat_clause(expr1, clauses, new_counter);
            clauses.push(FlatClause::Guard(*op, expr, expr1));
            new_counter
        }
    }
}

fn function_call2flat_clause(
    call: &FunctionCall,
    clauses: &mut Vec<FlatClause<NameOrVariable>>,
    mut counter: AnonymousVarCounter,
) -> (AnonymousVarCounter, NameOrVariable) {
    for i in 0..call.1.len() {
        let (new_counter, var) = expr2flat_clause(&call.1[i], clauses, counter);
        clauses.push(FlatClause::Lookup(
            var,
            TableName(call.0.clone()),
            ColumnIndex(i),
        ));
        counter = new_counter;
    }
    (counter.next(), NameOrVariable::Var(VarId(counter.0)))
}

fn expr2flat_clause(
    expr: &Expr,
    clauses: &mut Vec<FlatClause<NameOrVariable>>,
    counter: AnonymousVarCounter,
) -> (AnonymousVarCounter, NameOrVariable) {
    match expr {
        Expr::AtomOrVariable(AtomOrVariable::Atom(a)) => {
            let id = NameOrVariable::Var(VarId(counter.0));
            clauses.push(FlatClause::ConstCompare(Op::Equ, id.clone(), a.clone()));
            (counter.next(), id)
        }
        Expr::AtomOrVariable(AtomOrVariable::Variable(v)) => {
            (counter, NameOrVariable::Name(v.clone()))
        }
        Expr::FunctionCall(call) => function_call2flat_clause(call, clauses, counter),
    }
}
