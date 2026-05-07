use core::fmt::Display;

use alloc::boxed::Box;

use crate::{
    common::{Atom, Name, TableName, TypeName},
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