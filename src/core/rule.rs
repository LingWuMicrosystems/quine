use alloc::{boxed::Box, vec::Vec};

use crate::{
    common::{ColumnIndex, Value, Variable},
    core::{regraph::TableId, table::Row},
    frontend::syntax::VarName,
    types::Type,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Op {
    Equ,
    Neq,
    Lt,
    Gt,
    Leq,
    Geq,
    Ltu,
    Gtu,
    Lequ,
    Gequ,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Constraint {
    pub op: Op,
    pub value: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CrossConstraint {
    pub op: Op,
    pub lhs: Variable,
    pub rhs: Variable,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Rule {
    pub query: Query,
    pub action: Action,
}

/// table -> column -> constraints
pub type VarColsScanRule = Box<[FusedScan]>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Query {
    pub variables: VariableRecord,

    pub var_cols: Box<[VarColsScanRule]>,
    pub constraints: Box<[CrossConstraint]>,
}

pub type VariableRecord = Vec<(Type, Option<VarName>)>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FusedScan {
    pub table: TableId,
    pub column: ColumnIndex,
    pub column_type: Type,
    pub constraints: Box<[Constraint]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Action {
    pub lets: Box<[FunctionCall]>,
    pub tail: Box<[ActionTail]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ActionTail {
    Union(Variable, Variable),
    Insert(TableId, Box<[ValueOrVariable]>, Option<ValueOrVariable>),
    // Delete(TableId, Variable),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionCall {
    pub is_native: bool,
    pub offset: usize,
    pub args: Box<[ValueOrVariable]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValueOrVariable {
    Value(Value),
    Variable(Variable),
}

impl ValueOrVariable {
    pub fn resolve(&self, row: &Row) -> Value {
        match self {
            ValueOrVariable::Value(id) => *id,
            ValueOrVariable::Variable(i) => i.resolve(row),
        }
    }
}

impl Variable {
    pub fn resolve(&self, row: &Row) -> Value {
        row.0[self.0]
    }
}
