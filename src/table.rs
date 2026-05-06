use alloc::vec::Vec;
use core::hash::Hash;
use smallvec::{SmallVec, ToSmallVec};

use crate::common::{Id, Map, RowIndex};

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Row(pub SmallVec<[Id; 4]>);

#[derive(Debug, Clone)]
pub struct Table {
    pub arity: usize,
    pub rows: Vec<Id>,
    pub last_scan_end: usize,
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
            last_scan_end: Default::default(),
        }
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
