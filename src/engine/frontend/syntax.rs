use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::fmt::Display;

use crate::regraph::{
    common::{Atom, Name, Set, TableName, TypeName},
    rule::Op,
    types::{BaseType, TableDef, Type, TypeConstructor, TypeDef},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Command {
    TypeDef(TypeName, TypeDef),
    TableDef(TableName, TableDef),
    Rule(Rule),
    Fact(Bodys),
    // repl only
    Query(Heads, Vec<VarName>),
    Run,
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
    Match(ConstructorPattern),
    LetEq(Pattern, Pattern),
    Guard(Op, VarName, AtomOrVariable),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Pattern {
    Wildcard,
    Atom(Atom),
    Variable(VarName),
    Constructor(ConstructorPattern),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConstructorPattern(pub Name, pub Box<[Pattern]>);

// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// pub enum Op {
//     Equ,
//     Neq,
//     Lt,
//     Gt,
//     Leq,
//     Geq,
// }

// impl Op {
//     pub fn to_constraint_op(&self, is_sign: bool) -> rule::Op {
//         match self {
//             Op::Equ => rule::Op::Equ,
//             Op::Neq => rule::Op::Neq,
//             Op::Lt => {
//                 if is_sign {
//                     rule::Op::Lt
//                 } else {
//                     rule::Op::Ltu
//                 }
//             }
//             Op::Gt => {
//                 if is_sign {
//                     rule::Op::Gt
//                 } else {
//                     rule::Op::Gtu
//                 }
//             }
//             Op::Leq => {
//                 if is_sign {
//                     rule::Op::Leq
//                 } else {
//                     rule::Op::Lequ
//                 }
//             }
//             Op::Geq => {
//                 if is_sign {
//                     rule::Op::Geq
//                 } else {
//                     rule::Op::Gequ
//                 }
//             }
//         }
//     }
// }

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

        if paren && !args.is_empty() {
            write!(f, "(")?;
        }

        write!(f, "{g}")?;
        for arg in args {
            write!(f, " ")?;
            arg.fmt_internal(f, true)?;
        }

        if paren && !args.is_empty() {
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
                write!(f, "set {call}")?;

                if let Some(expr) = expr {
                    write!(f, " = {expr}")?;
                }

                Ok(())
            }
            Body::Union(l, r) => {
                write!(f, "union ")?;
                l.fmt_internal(f, true)?;
                write!(f, " with ")?;
                r.fmt_internal(f, true)
            }
            Body::Let(name, call) => {
                write!(f, "let {name} = {call}")
            }
        }
    }
}

impl Pattern {
    fn fmt_internal(&self, f: &mut core::fmt::Formatter<'_>, paren: bool) -> core::fmt::Result {
        match self {
            Pattern::Wildcard => write!(f, "_"),
            Pattern::Atom(a) => write!(f, "{a}"),
            Pattern::Variable(v) => write!(f, "{v}"),
            Pattern::Constructor(ConstructorPattern(g, args)) => {
                if paren && !args.is_empty() {
                    write!(f, "(")?;
                }

                write!(f, "{g}")?;
                for arg in args {
                    write!(f, " ")?;
                    arg.fmt_internal(f, true)?;
                }

                if paren && !args.is_empty() {
                    write!(f, ")")?;
                }

                Ok(())
            }
        }
    }
}

impl ConstructorPattern {
    fn fmt_internal(&self, f: &mut core::fmt::Formatter<'_>, paren: bool) -> core::fmt::Result {
        let g = &self.0;
        let args = &self.1;

        if paren && !args.is_empty() {
            write!(f, "(")?;
        }

        write!(f, "{g}")?;
        for arg in args {
            write!(f, " ")?;
            arg.fmt_internal(f, true)?;
        }

        if paren && !args.is_empty() {
            write!(f, ")")?;
        }

        Ok(())
    }
}

impl Display for ConstructorPattern {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.fmt_internal(f, false)
    }
}

impl Display for Pattern {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.fmt_internal(f, false)
    }
}

impl Display for Op {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let display = match self {
            Op::Equ => "=",
            Op::Neq => "!=",
            Op::Lt => "<",
            Op::Gt => ">",
            Op::Leq => "<=",
            Op::Geq => ">=",
        };

        write!(f, "{}", display)
    }
}

impl Display for Head {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Head::Match(p) => write!(f, "{p}"),
            Head::LetEq(l, r) => {
                write!(f, "leteq ")?;
                l.fmt_internal(f, true)?;
                write!(f, " <- ")?;
                r.fmt_internal(f, true)
            }
            Head::Guard(op, l, r) => {
                write!(f, "if {}", l)?;
                write!(f, " {op} {}", r)?;
                Ok(())
            }
        }
    }
}

impl Display for BaseType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = match self {
            BaseType::Id => "ID",
            BaseType::I1 => "i1",
            BaseType::I8 => "i8",
            BaseType::U8 => "u8",
            BaseType::I16 => "i16",
            BaseType::U16 => "u16",
            BaseType::I32 => "i32",
            BaseType::U32 => "u32",
            BaseType::F32 => "f32",
            BaseType::I64 => "i64",
            BaseType::U64 => "u64",
            BaseType::F64 => "f64",
            BaseType::Str => "str",
        };

        write!(f, "{s}")
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Type::Base(bt) => write!(f, "{bt:?}"),
            Type::Name(name) => write!(f, "{name}"),
        }
    }
}

impl Display for TypeConstructor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}(", self.0)?;
        for (idx, t) in self.1.iter().enumerate() {
            if idx != 0 {
                write!(f, ", ")?;
            }

            write!(f, "{t}")?;
        }

        write!(f, ")")
    }
}

impl Display for TypeDef {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "type {}", self.0)?;
        for con in &self.1.0 {
            writeln!(f, "| {con}")?;
        }

        Ok(())
    }
}

impl Display for TableDef {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "table {}",
            TypeConstructor(self.0.clone(), self.1.clone())
        )
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Command::TypeDef(_, def) => write!(f, "{def}"),
            Command::Rule(Rule { heads, bodys }) => {
                write!(f, "rule ")?;
                for (idx, head) in heads.iter().enumerate() {
                    if idx != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{head}")?;
                }
                write!(f, " {{ ")?;
                for (idx, body) in bodys.iter().enumerate() {
                    if idx != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{body}")?;
                }
                write!(f, " }}")
            }
            Command::Fact(fact) => {
                write!(
                    f,
                    "fact {}",
                    fact.iter()
                        .map(|b| b.to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            }
            Command::TableDef(_, def) => write!(f, "{def}"),
            Command::Query(heads, vars) => {
                write!(f, "query ")?;
                for (idx, head) in heads.iter().enumerate() {
                    if idx != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{head}")?;
                }
                write!(f, " print(")?;
                for (idx, v) in vars.iter().enumerate() {
                    if idx != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{v}")?;
                }
                write!(f, ")")
            }
            Command::Run => write!(f, "run"),
        }
    }
}
