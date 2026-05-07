use alloc::{boxed::Box, string::String};

use crate::{
    common::{Name, Variable},
    context::{FunctionType, RelationType},
    rule::Op,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ReplCommand {
    TypeDef(Name, TypeDef),
    Rule(Name, Rule),
    Fact(Fact),
    Query(Rule),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Command {
    TypeDef(Name, TypeDef),
    Rule(Rule),
    Fact(Fact),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeDef {
    Relation(RelationType),
    Function(FunctionType),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fact(pub FunctionCall, pub Option<Expr>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Rule {
    pub heads: Heads,
    pub bodys: Bodys,
}

pub type Heads = Box<[Head]>;
pub type Bodys = Box<[Body]>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Head {
    Pattern(Pattern),
    LetEq(Expr, Expr),
    Guard(Op, Expr, Expr),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Pattern {
    Wildcard,
    AtomOrVariable(AtomOrVariable),
    Constructor(String, Box<[Pattern]>),
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
    Union(Expr, Expr),
    Let(VarName, Expr),
}

pub type Function = Name;
pub type VarName = Name;
