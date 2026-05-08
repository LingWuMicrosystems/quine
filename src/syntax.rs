use alloc::{boxed::Box, string::String};

use crate::{
    common::{Atom, Name},
    core::rule::Op,
    types::{TableDef, TypeDef},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Command {
    TypeDef(Name, TypeDef),
    TableDef(Name, TableDef),
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
    Variable(VarName),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Body {
    Let(FunctionCall),
    Insert(FunctionCall, Option<Expr>),
    Union(Expr, Expr),
}

pub type Function = Name;
pub type VarName = Name;
