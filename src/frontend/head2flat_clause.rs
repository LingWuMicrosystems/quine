use alloc::{vec, vec::Vec};

use crate::{
    common::{Atom, ColumnIndex, TableName, VarId},
    frontend::{
        error::CompileError,
        syntax::{AtomOrVariable, ConstructorPattern, Head, Op, Pattern, VarName},
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
            let (new_counter, expr) = pattern2flat_clause(expr, clauses, counter);
            let (new_counter, expr1) = pattern2flat_clause(expr1, clauses, new_counter);
            if let (Some(expr), Some(expr1)) = (expr, expr1) {
                clauses.push(FlatClause::Guard(Op::Equ, expr, expr1));
                new_counter
            } else {
                counter
            }
        }
        Head::Guard(op, expr, Pattern::AtomOrVariable(AtomOrVariable::Atom(a)))
        | Head::Guard(op, Pattern::AtomOrVariable(AtomOrVariable::Atom(a)), expr) => {
            let (new_counter, var) = pattern2flat_clause(expr, clauses, counter);
            if let Some(var) = var {
                clauses.push(FlatClause::ConstCompare(*op, var, a.clone()));
                new_counter
            } else {
                counter
            }
        }
        Head::Guard(op, expr, expr1) => {
            let (new_counter, expr) = pattern2flat_clause(expr, clauses, counter);
            let (new_counter, expr1) = pattern2flat_clause(expr1, clauses, new_counter);
            if let (Some(expr), Some(expr1)) = (expr, expr1) {
                clauses.push(FlatClause::Guard(*op, expr, expr1));
                new_counter
            } else {
                counter
            }
        }
    }
}

fn function_call2flat_clause(
    call: &ConstructorPattern,
    clauses: &mut Vec<FlatClause<NameOrVariable>>,
    mut counter: AnonymousVarCounter,
) -> (AnonymousVarCounter, NameOrVariable) {
    for i in 0..call.1.len() {
        let (new_counter, var) = pattern2flat_clause(&call.1[i], clauses, counter);
        if let Some(var) = var {
            clauses.push(FlatClause::Lookup(
                var,
                TableName(call.0.clone()),
                ColumnIndex(i),
            ));
            counter = new_counter;
        }
    }
    (counter.next(), NameOrVariable::Var(VarId(counter.0)))
}

fn pattern2flat_clause(
    pat: &Pattern,
    clauses: &mut Vec<FlatClause<NameOrVariable>>,
    counter: AnonymousVarCounter,
) -> (AnonymousVarCounter, Option<NameOrVariable>) {
    match pat {
        Pattern::Wildcard => (counter, None),
        Pattern::AtomOrVariable(AtomOrVariable::Atom(a)) => {
            let id = NameOrVariable::Var(VarId(counter.0));
            clauses.push(FlatClause::ConstCompare(Op::Equ, id.clone(), a.clone()));
            (counter.next(), Some(id))
        }
        Pattern::AtomOrVariable(AtomOrVariable::Variable(v)) => {
            (counter, Some(NameOrVariable::Name(v.clone())))
        }
        Pattern::Constructor(call) => {
            let (counter, var) = function_call2flat_clause(call, clauses, counter);
            (counter, Some(var))
        }
    }
}
