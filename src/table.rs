use alloc::vec::Vec;
use core::hash::Hash;
use smallvec::{SmallVec, ToSmallVec};

use crate::{
    common::{ColumnIndex, Id, Map, RowIndex, Set},
    rule::{Constraint, Op},
    uf::UnionFind,
};

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Row(pub SmallVec<[Id; 4]>);

#[derive(Debug, Clone)]
pub struct Table {
    pub arity: usize,
    pub rows: Vec<Id>,
    pub key_index: Map<Row, RowIndex>,
    pub parents: Map<Id, Vec<RowIndex>>,
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

    pub fn fused_scan(
        &self,
        uf: &UnionFind,
        find_column: ColumnIndex,
        cs: &[(ColumnIndex, Constraint)],
    ) -> Set<Id> {
        self.rows
            .chunks(self.arity)
            .map(|row: &[Id]| Row(row.iter().map(|id| uf.find(*id)).collect()))
            .filter(|row| {
                cs.iter().all(|(column, constraint)| {
                    let value = row.0[column.0];
                    match constraint.op {
                        Op::Equ => value == constraint.id,
                        Op::Neq => value != constraint.id,
                        Op::Lt => (value.0 as i32) < (constraint.id.0 as i32),
                        Op::Gt => (value.0 as i32) > (constraint.id.0 as i32),
                        Op::Leq => (value.0 as i32) <= (constraint.id.0 as i32),
                        Op::Geq => (value.0 as i32) >= (constraint.id.0 as i32),
                        Op::Ltu => value < constraint.id,
                        Op::Gtu => value > constraint.id,
                        Op::Lequ => value <= constraint.id,
                        Op::Gequ => value >= constraint.id,
                    }
                })
            })
            .map(|row| row.0[find_column.0])
            .collect()
    }

    #[inline]
    pub fn get_all_row(&self, idx: RowIndex) -> &[Id] {
        let start = idx.0 * self.arity;
        &self.rows[start..start + self.arity]
    }

    pub fn get_row_and_result(&self, idx: RowIndex) -> (Row, Id) {
        let row = self.get_all_row(idx);
        let result = row[row.len() - 1];
        (Row(row[..row.len() - 1].to_smallvec()), result)
    }

    pub fn get_row(&self, idx: RowIndex) -> Row {
        let start = idx.0 * self.arity;
        let row = &self.rows[start..start + self.arity - 1];
        Row(row.to_smallvec())
    }

    pub fn get_result(&self, idx: RowIndex) -> Id {
        let start = idx.0 * self.arity;
        self.rows[start + self.arity - 1]
    }
}
