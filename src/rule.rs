use alloc::{boxed::Box, vec::Vec};
use smallvec::SmallVec;

use crate::{
    common::{ColumnIndex, Id, Variable},
    regraph::TableId,
    table::Row,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Op {
    Equ,
    Neq,
    Lt,
    Gt,
    Leq,
    Geq,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Constraint {
    pub op: Op,
    pub id: Id,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CrossConstraint {
    pub op: Op,
    pub lhs: Variable,
    pub rhs: Variable,
}

/// table -> column -> constraints
pub type VarColsScanRule = Box<[FusedScan]>;

#[derive(Debug, Clone)]
pub struct Rule {
    pub head_var_order: SmallVec<[Variable; 4]>,
    pub var_cols: Box<[VarColsScanRule]>,
    pub constraints: SmallVec<[CrossConstraint; 2]>,
    pub body_var_order: SmallVec<[Variable; 4]>,
    pub actions: SmallVec<[Action; 2]>,
}

#[derive(Debug, Clone)]
pub struct FusedScan {
    pub table: TableId,
    pub column: ColumnIndex,
    pub constraints: Option<Constraint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Action {
    Union(Variable, Variable),
    Insert(TableId, Vec<Atom>),
    // Delete(TableId, Variable),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Atom {
    Const(Id),
    Var(Variable),
}

impl Atom {
    pub fn resolve(&self, row: &Row) -> Id {
        match self {
            Atom::Const(id) => *id,
            Atom::Var(i) => i.resolve(row),
        }
    }
}

impl Variable {
    pub fn resolve(&self, row: &Row) -> Id {
        row.0[self.0]
    }
}
