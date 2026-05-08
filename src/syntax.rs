use alloc::{boxed::Box, string::String};

use crate::{
    common::{Atom, Name, Set, TableName, TypeName},
    types::{TableDef, TypeDef},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Command {
    TypeDef(TypeName, TypeDef),
    TableDef(TableName, TableDef),
    Rule(Rule),
    Fact(Fact),
    // repl only
    Query(Heads),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FactConstructor(pub Name, Box<[Fact]>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Fact {
    Atom(Atom),
    FactConstructor(FactConstructor),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Rule {
    pub heads: Heads,
    pub bodys: Bodys,
}

pub type Heads = Box<[Head]>;
pub type Bodys = Box<[Body]>;

pub trait VarExtractor {
    fn extract_vars(&self) -> Set<VarName>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Head {
    Pattern(Pattern),
    LetEq(Expr, Expr),
    Guard(Op, Expr, Expr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Op {
    Equ,
    Neq,
    Lt,
    Gt,
    Leq,
    Geq,
}

impl VarExtractor for Head {
    fn extract_vars(&self) -> Set<VarName> {
        match self {
            Head::Pattern(pattern) => pattern.extract_vars(),
            Head::LetEq(expr, expr1) | Head::Guard(_, expr, expr1) => expr
                .extract_vars()
                .union(&expr1.extract_vars())
                .cloned()
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Pattern {
    Wildcard,
    AtomOrVariable(AtomOrVariable),
    Constructor(String, Box<[Pattern]>),
}

impl VarExtractor for Pattern {
    fn extract_vars(&self) -> Set<VarName> {
        match self {
            Pattern::Wildcard => todo!(),
            Pattern::AtomOrVariable(e) => e.extract_vars(),
            Pattern::Constructor(_, patterns) => {
                patterns.iter().fold(Set::default(), |acc, pattern| {
                    acc.union(&pattern.extract_vars()).cloned().collect()
                })
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    AtomOrVariable(AtomOrVariable),
    FunctionCall(FunctionCall),
}

impl VarExtractor for Expr {
    fn extract_vars(&self) -> Set<VarName> {
        match self {
            Expr::AtomOrVariable(e) => e.extract_vars(),
            Expr::FunctionCall(call) => call.extract_vars(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionCall(pub Function, pub Box<[Expr]>);

impl VarExtractor for FunctionCall {
    fn extract_vars(&self) -> Set<VarName> {
        self.1.iter().fold(Set::default(), |acc, arg| {
            acc.union(&arg.extract_vars()).cloned().collect()
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AtomOrVariable {
    Atom(Atom),
    Variable(VarName),
}

impl VarExtractor for AtomOrVariable {
    fn extract_vars(&self) -> Set<VarName> {
        match self {
            AtomOrVariable::Atom(_) => Set::default(),
            AtomOrVariable::Variable(v) => Set::from_iter([v.clone()]),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Body {
    Let(VarName, FunctionCall),
    Insert(FunctionCall, Option<Expr>),
    Union(Expr, Expr),
}

pub type Function = Name;
pub type VarName = Name;
