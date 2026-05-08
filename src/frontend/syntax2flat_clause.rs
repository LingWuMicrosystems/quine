use alloc::{format, vec, vec::Vec};

use crate::{
    common::{Atom, ColumnIndex, TableName},
    core::rule,
    frontend::{
        error::TypeCheckError,
        syntax::{self, AtomOrVariable, Expr, FunctionCall, Head, Op, VarName},
    },
};

#[derive(Debug, Clone)]
pub enum FlatClause<V> {
    Lookup(V, TableName, ColumnIndex),
    Eq(V, V),
    ConstCompare(Op, V, Atom),
    Guard(Op, V, V),
}

pub fn syntax_rule2flat_clause(rule: &syntax::Rule) -> Result<rule::Rule, TypeCheckError> {
    let mut clauses = vec![];
    let mut variables = vec![];
    let mut counter = AnonymousVarCounter::default();
    for head in &rule.heads {
        match head {
            Head::Match(pattern) => {
                let (new_counter, _) =
                    function_call2flat_clause(&pattern, &mut variables, &mut clauses, counter);
                counter = new_counter;
            }
            Head::LetEq(expr, expr1) => {
                let (new_counter, expr) =
                    expr2flat_clause(expr, &mut clauses, &mut variables, counter);
                let (new_counter, expr1) =
                    expr2flat_clause(expr1, &mut clauses, &mut variables, new_counter);
                clauses.push(FlatClause::Eq(expr, expr1));
                counter = new_counter;
            }
            Head::Guard(op, expr, expr1) => {
                let (new_counter, expr) =
                    expr2flat_clause(expr, &mut clauses, &mut variables, counter);
                let (new_counter, expr1) =
                    expr2flat_clause(expr1, &mut clauses, &mut variables, new_counter);
                clauses.push(FlatClause::Guard(*op, expr, expr1));
                counter = new_counter;
            }
        }
    }
    todo!()
}

#[derive(Debug, Default, Clone, Copy)]
pub struct AnonymousVarCounter(pub usize);

impl AnonymousVarCounter {
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

pub fn function_call2flat_clause(
    call: &FunctionCall,
    vars: &mut Vec<VarName>,
    clauses: &mut Vec<FlatClause<VarName>>,
    mut counter: AnonymousVarCounter,
) -> (AnonymousVarCounter, VarName) {
    for i in 0..call.1.len() {
        let (new_counter, var) = expr2flat_clause(&call.1[i], clauses, vars, counter);
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

pub fn expr2flat_clause(
    expr: &Expr,
    clauses: &mut Vec<FlatClause<VarName>>,
    vars: &mut Vec<VarName>,
    counter: AnonymousVarCounter,
) -> (AnonymousVarCounter, VarName) {
    match expr {
        Expr::AtomOrVariable(AtomOrVariable::Atom(a)) => {
            let name = format!("t_{}", counter.0);
            clauses.push(FlatClause::ConstCompare(Op::Equ, name.clone(), a.clone()));
            (counter.next(), name)
        }
        Expr::AtomOrVariable(AtomOrVariable::Variable(v)) => (counter, v.clone()),
        Expr::FunctionCall(call) => function_call2flat_clause(&call, vars, clauses, counter),
    }
}
