use core::fmt::Display;

use alloc::{boxed::Box, string::String, vec::Vec};

use crate::{
    common::{ColumnIndex, Map, Set, Value, VarId},
    related_egraph::TableId,
    table::Row,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Constraint {
    pub op: Op,
    pub column: ColumnIndex,
    pub value: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CrossConstraint {
    pub op: Op,
    pub lhs: VarId,
    pub rhs: VarId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub query: Query,
    pub action: Action,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    pub variables: VariableRecord,

    pub scan_steps: Box<[ScanStep]>,
    pub constraints: Box<[CrossConstraint]>,
}

impl Query {
    pub fn tables(&self) -> Set<TableId> {
        self.scan_steps.iter().map(|s| s.table).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanStep {
    pub table: TableId,
    pub columns: Vec<ColumnIndex>,
    pub var_binding: Vec<VarId>,
    pub constraints: Vec<Constraint>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct VariableRecord {
    variables: Vec<Type>,
    pub names_map: Map<String, usize>,
}

impl VariableRecord {
    pub fn insert_var(&mut self, name: Option<String>, ty: Type) -> VarId {
        let i = self.variables.len();
        self.variables.push(ty);
        if let Some(name) = name {
            self.names_map.insert(name, i);
        }
        VarId(i)
    }

    pub fn get_offset(&self, name: &String) -> Option<usize> {
        self.names_map.get(name).copied()
    }

    pub fn get_type(&self, offset: usize) -> Option<&Type> {
        self.variables.get(offset)
    }

    pub fn get_type_from_name(&self, name: &String) -> Option<&Type> {
        self.get_offset(name).and_then(|i| self.get_type(i))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Action {
    // pub lets_map: Map<VarName, VarId>,
    pub lets: Box<[FunctionCall]>,
    pub tail: Box<[ActionTail]>,
}

impl Action {
    pub fn tables(&self) -> Set<TableId> {
        let lets: Set<_> = self.lets.iter().map(|fc| fc.offset).collect();
        let tail: Set<_> = self
            .tail
            .iter()
            .filter_map(|t| {
                if let ActionTail::Insert(offset, _, _) = t {
                    Some(*offset)
                } else {
                    None
                }
            })
            .collect();
        lets.union(&tail).copied().collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ActionTail {
    Union(ValueOrVariable, ValueOrVariable),
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
    Variable(VarId),
}

impl ValueOrVariable {
    pub fn resolve(&self, row: &Row) -> Value {
        match self {
            ValueOrVariable::Value(id) => *id,
            ValueOrVariable::Variable(i) => i.resolve(row),
        }
    }
}

impl VarId {
    pub fn resolve(&self, row: &Row) -> Value {
        row.0[self.0]
    }
}
