use alloc::boxed::Box;

use crate::{
    common::{Atom, Name, TableName, TypeName},
    core::rule,
    types::{TableDef, TypeDef},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Command {
    TypeDef(TypeName, TypeDef),
    TableDef(TableName, TableDef),
    Rule(Rule),
    Fact(FunctionCall),
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Head {
    Match(FunctionCall),
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

impl Op {
    pub fn to_constraint_op(&self, is_sign: bool) -> rule::Op {
        match self {
            Op::Equ => rule::Op::Equ,
            Op::Neq => rule::Op::Neq,
            Op::Lt => {
                if is_sign {
                    rule::Op::Lt
                } else {
                    rule::Op::Ltu
                }
            }
            Op::Gt => {
                if is_sign {
                    rule::Op::Gt
                } else {
                    rule::Op::Gtu
                }
            }
            Op::Leq => {
                if is_sign {
                    rule::Op::Leq
                } else {
                    rule::Op::Lequ
                }
            }
            Op::Geq => {
                if is_sign {
                    rule::Op::Geq
                } else {
                    rule::Op::Gequ
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    AtomOrVariable(AtomOrVariable),
    FunctionCall(FunctionCall),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionCall(pub Function, pub Box<[Expr]>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AtomOrVariable {
    Atom(Atom),
    Variable(VarName),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Body {
    Let(VarName, FunctionCall),
    Insert(FunctionCall, Option<Expr>),
    Union(Expr, Expr),
}

pub type Function = Name;
pub type VarName = Name;
