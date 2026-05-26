use alloc::vec;
use alloc::vec::Vec;

use smallvec::SmallVec;

use crate::{
    common::{ColumnIndex, Map, RowIndex, Value},
    rule::{Constraint, Op},
    types::{BaseType, MergeFn, TableDef, Type},
    uf::UnionFind,
};

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Row(pub SmallVec<[Value; 7]>);

#[derive(Debug, Clone)]
pub struct Table {
    pub table_def: TableDef,
    pub rows: Vec<Value>,
    pub key_index: Map<Row, RowIndex>,
    pub parents: Map<Value, Vec<RowIndex>>,
    pub row_count: usize,
    pub delta_start_row: usize,
}

impl Table {
    pub fn new(table_def: TableDef) -> Self {
        Self {
            table_def,
            rows: vec![],
            key_index: Default::default(),
            parents: Default::default(),
            row_count: Default::default(),
            delta_start_row: 0,
        }
    }

    pub fn arity(&self) -> usize {
        self.table_def.1.len() - 1
    }

    pub fn column_count(&self) -> usize {
        self.table_def.1.len()
    }

    pub fn row_count(&self) -> usize {
        self.row_count
    }

    pub fn reset_delta(&mut self) {
        self.delta_start_row = self.row_count;
    }

    pub fn has_delta(&self) -> bool {
        self.delta_start_row < self.row_count
    }

    /// Inserts a row. Returns `Some(row_index)` if a new row was added,
    /// `None` if an existing row was updated.
    pub fn insert(&mut self, uf: &mut UnionFind, mut key: Row, value: Value) -> Option<RowIndex> {
        debug_assert_eq!(key.0.len(), self.arity());
        if let Some(r) = self.key_index.get(&key) {
            let column = self.column_count();
            let arity = self.arity();
            let value_idx = r.0 * column + arity;
            let value_ref = &mut self.rows[value_idx];
            match &self.table_def.2 {
                Some(MergeFn::Min) => {
                    if value < *value_ref {
                        *value_ref = value;
                        self.delta_start_row = 0;
                    }
                }
                Some(MergeFn::Max) => {
                    if value > *value_ref {
                        *value_ref = value;
                        self.delta_start_row = 0;
                    }
                }
                None => {
                    if let Some((_old, value)) = uf.union(*value_ref, value) {
                        *value_ref = value;
                    } else {
                        *value_ref = value;
                    };
                }
            }
            None
        } else {
            let idx = RowIndex(self.row_count);
            self.key_index.insert(key.clone(), idx);
            key.0.push(value);
            self.rows.extend(&key.0);
            self.row_count += 1;
            Some(idx)
        }
    }

    pub fn fused_scan(
        &self,
        uf: &UnionFind,
        find_columns: &[ColumnIndex],
        constraints: &[Constraint],
        use_delta: bool,
    ) -> impl Iterator<Item = Row> {
        let start = if use_delta {
            self.delta_start_row * self.column_count()
        } else {
            0
        };
        self.rows[start..]
            .chunks(self.column_count())
            .map(|row| {
                let row = row
                    .iter()
                    .zip(self.table_def.1.iter())
                    .map(|(v, ty)| {
                        if matches!(ty, Type::Name(_) | Type::Base(BaseType::Id)) {
                            uf.find(*v)
                        } else {
                            *v
                        }
                    })
                    .collect();
                Row(row)
            })
            .filter(|row| {
                constraints.iter().all(|c| match c.op {
                    Op::Equ => row.0[c.column.0] == c.value,
                    Op::Neq => row.0[c.column.0] != c.value,
                    Op::Lt => row.0[c.column.0] < c.value,
                    Op::Gt => row.0[c.column.0] > c.value,
                    Op::Leq => row.0[c.column.0] <= c.value,
                    Op::Geq => row.0[c.column.0] >= c.value,
                })
            })
            .map(|row| {
                let row = find_columns.iter().map(|c| row.0[c.0]).collect();
                Row(row)
            })
    }

    #[inline]
    pub fn get_all_row(&self, row_index: RowIndex) -> Row {
        let start = row_index.0 * self.column_count();
        let end = start + self.column_count();
        Row(self.rows[start..end].into())
    }
}
