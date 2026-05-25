use core::fmt::Display;

use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use quine_core::{
    common::Set,
    rule::Op,
    types::{BaseType, TableDef, TypeDef},
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Command {
    TypeDef(String, TypeDef),
    TableDef(String, TableDef),
    Rule(Rule),
    Fact(Bodys),
    Query(Heads, Vec<String>),
    Run(Option<String>, Option<usize>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Rule {
    pub group: Option<String>,
    pub heads: Heads,
    pub bodys: Bodys,
}

pub type Heads = Box<[Head]>;
pub type Bodys = Box<[Body]>;

pub trait VarExtractor {
    fn extract_vars(&self) -> Set<String>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Head {
    Match(Span, ConstructorPattern),
    LetEq(Span, Pattern, Pattern),
    Guard(Span, Op, String, AtomOrVariable),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Pattern {
    Wildcard(Span),
    Atom(Span, Atom),
    Variable(Span, String),
    Constructor(Span, ConstructorPattern),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConstructorPattern {
    pub name: String,
    pub args: Box<[Pattern]>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    AtomOrVariable(AtomOrVariable),
    FunctionCall(FunctionCall),
}

impl VarExtractor for Expr {
    fn extract_vars(&self) -> Set<String> {
        match self {
            Expr::AtomOrVariable(e) => e.extract_vars(),
            Expr::FunctionCall(call) => call.extract_vars(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionCall(pub String, pub Box<[Expr]>);

impl VarExtractor for FunctionCall {
    fn extract_vars(&self) -> Set<String> {
        self.1.iter().fold(Set::default(), |acc, arg| {
            acc.union(&arg.extract_vars()).cloned().collect()
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AtomOrVariable {
    Atom(Atom),
    Variable(String),
}

impl VarExtractor for AtomOrVariable {
    fn extract_vars(&self) -> Set<String> {
        match self {
            AtomOrVariable::Atom(_) => Set::default(),
            AtomOrVariable::Variable(v) => Set::from_iter([v.clone()]),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Body {
    Let(Span, String, FunctionCall),
    Insert(Span, FunctionCall, Option<Expr>),
    Union(Span, Expr, Expr),
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
            Body::Insert(_, call, expr) => {
                write!(f, "set {call}")?;

                if let Some(expr) = expr {
                    write!(f, " = {expr}")?;
                }

                Ok(())
            }
            Body::Union(_, l, r) => {
                write!(f, "union ")?;
                l.fmt_internal(f, true)?;
                write!(f, " with ")?;
                r.fmt_internal(f, true)
            }
            Body::Let(_, name, call) => {
                write!(f, "let {name} = {call}")
            }
        }
    }
}

impl Pattern {
    fn fmt_internal(&self, f: &mut core::fmt::Formatter<'_>, paren: bool) -> core::fmt::Result {
        match self {
            Pattern::Wildcard(_) => write!(f, "_"),
            Pattern::Atom(_, a) => write!(f, "{a}"),
            Pattern::Variable(_, v) => write!(f, "{v}"),
            Pattern::Constructor(_, cp) => cp.fmt_internal(f, paren),
        }
    }
}

impl ConstructorPattern {
    fn fmt_internal(&self, f: &mut core::fmt::Formatter<'_>, paren: bool) -> core::fmt::Result {
        if paren && !self.args.is_empty() {
            write!(f, "(")?;
        }

        write!(f, "{}", self.name)?;
        for arg in self.args.iter() {
            write!(f, " ")?;
            arg.fmt_internal(f, true)?;
        }

        if paren && !self.args.is_empty() {
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

impl Display for Head {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Head::Match(_, p) => write!(f, "{p}"),
            Head::LetEq(_, l, r) => {
                write!(f, "leteq ")?;
                l.fmt_internal(f, true)?;
                write!(f, " <- ")?;
                r.fmt_internal(f, true)
            }
            Head::Guard(_, op, l, r) => {
                write!(f, "if {}", l)?;
                write!(f, " {op} {}", r)?;
                Ok(())
            }
        }
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Command::TypeDef(_, def) => write!(f, "{def}"),
            Command::TableDef(_, def) => write!(f, "{def}"),
            Command::Rule(Rule { heads, bodys, .. }) => {
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
            Command::Fact(bodies) => {
                write!(
                    f,
                    "fact {}",
                    bodies
                        .iter()
                        .map(|b| b.to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            }
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
            Command::Run(..) => write!(f, "run"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Atom {
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    U8(u8),
    U16(u16),
    U32(u32),

    I64(i64),
    U64(u64),
    F32(u32), // IEEE 754 bits
    F64(u64), // IEEE 754 bits
    Str(String),
}

impl Display for Atom {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Atom::Bool(b) => write!(f, "{b}"),
            Atom::I8(i) => write!(f, "{i}"),
            Atom::I16(i) => write!(f, "{i}"),
            Atom::I32(i) => write!(f, "{i}"),
            Atom::U8(u) => write!(f, "{u}"),
            Atom::U16(u) => write!(f, "{u}"),
            Atom::U32(u) => write!(f, "{u}"),
            Atom::I64(i) => write!(f, "{i}"),
            Atom::U64(u) => write!(f, "{u}"),
            Atom::F32(bits) => write!(f, "{}", f32::from_bits(*bits)),
            Atom::F64(bits) => write!(f, "{}", f64::from_bits(*bits)),
            Atom::Str(s) => write!(f, "{s}"),
        }
    }
}

impl Atom {
    pub fn get_type(&self) -> BaseType {
        match self {
            Atom::Bool(_) => BaseType::I1,
            Atom::I8(_) => BaseType::I8,
            Atom::I16(_) => BaseType::I16,
            Atom::I32(_) => BaseType::I32,
            Atom::U8(_) => BaseType::U8,
            Atom::U16(_) => BaseType::U16,
            Atom::U32(_) => BaseType::U32,
            Atom::I64(_) => BaseType::I64,
            Atom::U64(_) => BaseType::U64,
            Atom::F32(_) => BaseType::F32,
            Atom::F64(_) => BaseType::F64,
            Atom::Str(_) => BaseType::Str,
        }
    }
}
