use alloc::vec::Vec;
use core::hash::Hash;
use smallvec::{SmallVec, ToSmallVec};

use crate::{
    common::{ColumnIndex, Map, RowIndex, Set, Value},
    regraph::rule::{Constraint, Op},
};

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Row(pub SmallVec<[Value; 4]>);

#[derive(Debug, Clone)]
pub struct Table {
    pub arity: usize,
    pub rows: Vec<Value>,
    pub key_index: Map<Row, RowIndex>,
    pub parents: Map<Value, Vec<RowIndex>>,
}

impl Table {
    pub fn new(arity: usize) -> Self {
        Self {
            arity,
            rows: Default::default(),
            key_index: Default::default(),
            parents: Default::default(),
        }
    }

    pub fn insert_row(&mut self, row: Row) {
        debug_assert_eq!(row.0.len(), self.arity);
        self.rows.extend(row.0);
    }

    pub fn fused_scan(&self, find_column: ColumnIndex, cs: &[Constraint]) -> Set<Value> {
        self.rows
            .chunks(self.arity)
            .filter_map(|row| {
                if cs.iter().all(|constraint| {
                    let value = row[find_column.0];
                    match constraint.op {
                        Op::Equ => value == constraint.value,
                        Op::Neq => value != constraint.value,
                        Op::Lt => (value.0 as i32) < (constraint.value.0 as i32),
                        Op::Gt => (value.0 as i32) > (constraint.value.0 as i32),
                        Op::Leq => (value.0 as i32) <= (constraint.value.0 as i32),
                        Op::Geq => (value.0 as i32) >= (constraint.value.0 as i32),
                        Op::Ltu => value < constraint.value,
                        Op::Gtu => value > constraint.value,
                        Op::Lequ => value <= constraint.value,
                        Op::Gequ => value >= constraint.value,
                    }
                }) {
                    Some(row[find_column.0])
                } else {
                    None
                }
            })
            .collect()
    }

    #[inline]
    pub fn get_all_row(&self, idx: RowIndex) -> &[Value] {
        let start = idx.0 * self.arity;
        &self.rows[start..start + self.arity]
    }

    pub fn get_row_and_result(&self, idx: RowIndex) -> (Row, Value) {
        let row = self.get_all_row(idx);
        let result = row[row.len() - 1];
        (Row(row[..row.len() - 1].to_smallvec()), result)
    }

    pub fn get_row(&self, idx: RowIndex) -> Row {
        let start = idx.0 * self.arity;
        let row = &self.rows[start..start + self.arity - 1];
        Row(row.to_smallvec())
    }

    pub fn get_result(&self, idx: RowIndex) -> Value {
        let start = idx.0 * self.arity;
        self.rows[start + self.arity - 1]
    }
}
