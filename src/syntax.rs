use core::fmt::Display;

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

impl Display for Atom {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Atom::Int(i) => write!(f, "{i}i"),
            Atom::Uint(u) => write!(f, "{u}u"),
            Atom::Bool(b) => write!(f, "{b}"),
            Atom::Str(s) => write!(f, "\"{}\"", s.escape_debug()),
        }
    }
}

impl Display for AtomOrVariable {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AtomOrVariable::Atom(atom) => write!(f, "{atom}"),
            AtomOrVariable::Variable(v) => write!(f, "{v}"),
        }
    }
}

impl FunctionCall {
    fn fmt_internal(&self, f: &mut core::fmt::Formatter<'_>, paren: bool) -> core::fmt::Result {
        let g = &self.0;
        let args = &self.1;

        if paren && ! args.is_empty() {
            write!(f, "(")?;
        }

        write!(f, "{g}");
        for arg in args {
            write!(f, " ")?;
            arg.fmt_internal(f, true)?;
        }

        if paren && ! args.is_empty() {
            write!(f, ")")?;
        }

        Ok(())
    }
}

impl Expr {
    fn fmt_internal(&self, f: &mut core::fmt::Formatter<'_>, paren: bool) -> core::fmt::Result {
        match self {
            Expr::AtomOrVariable(aov) => write!(f, "{aov}"),
            Expr::FunctionCall(call) => call.fmt_internal(f, paren),
        }
    }
}

impl Display for FunctionCall {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.fmt_internal(f, false)
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.fmt_internal(f, false)
    }
}

impl Display for Body {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Body::Insert(call, expr) => {
                write!(f, "insert {call}")?;

                if let Some(expr) = expr {
                    write!(f, " -> {expr}")?;
                }

                Ok(())
            },
            Body::Union(l, r) => {
                write!(f, "union ")?;
                l.fmt_internal(f, true)?;
                write!(f, " ")?;
                r.fmt_internal(f, true)
            },
        }
    }
}

impl Pattern {
    fn fmt_internal(&self, f: &mut core::fmt::Formatter<'_>, paren: bool) -> core::fmt::Result {
        match self {
            Pattern::Wildcard => write!(f, "_"),
            Pattern::AtomOrVariable(aov) => write!(f, "{aov}"),
            Pattern::Constructor(g, args) => {
                if paren && ! args.is_empty() {
                    write!(f, "(")?;
                }

                write!(f, "{g}");
                for arg in args {
                    write!(f, " ")?;
                    arg.fmt_internal(f, true)?;
                }

                if paren && ! args.is_empty() {
                    write!(f, ")")?;
                }

                Ok(())
            },
        }
    }
}

impl Display for Pattern {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.fmt_internal(f, false)
    }
}

impl Display for Head {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Head::Pattern(p) => write!(f, "{p}"),
            Head::LetEq(l, r) => {
                write!(f, "leteq ")?;
                l.fmt_internal(f, true)?;
                write!(f, " ")?;
                r.fmt_internal(f, true)
            },
            Head::Guard(op, l, r) => {
                write!(f, "if ")?;
                l.fmt_internal(f, true)?;
                write!(f, "{op:?}")?;
                r.fmt_internal(f, true)
            },
        }
    }
}