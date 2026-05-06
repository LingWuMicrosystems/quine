use alloc::{string::String, vec::Vec};
use smallvec::SmallVec;

use crate::{common::Variable, rule::Op};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Command {
    pub heads: Heads,
    pub bodys: Bodys,
}

pub type Heads = SmallVec<[Pattern; 4]>;
pub type Bodys = SmallVec<[Body; 4]>;

pub enum Head {
    Pattern(Pattern),
    LetEq(Expr, Expr),
    Guard(Op, Expr, Expr),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Pattern {
    Wildcard,
    AtomOrVariable(AtomOrVariable),
    Constructor(String, Vec<Pattern>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    AtomOrVariable(AtomOrVariable),
    FunctionCall(FunctionCall),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionCall(pub Function, pub Vec<Expr>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AtomOrVariable {
    Atom(Atom),
    Variable(Variable),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Atom {
    Int(i64),
    Uint(u64),
    Bool(bool),
    Str(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Body {
    Insert(FunctionCall),
    Union(Variable, Variable),
    // Let(VarName, Expr, Box<Constructor>), // let t = (Add x y) in (Insert (edge t z) 1)
}

pub type Function = Name;
pub type VarName = Name;
pub type Name = String;
