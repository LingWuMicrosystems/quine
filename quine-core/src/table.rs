use alloc::vec;
use alloc::vec::Vec;

use smallvec::SmallVec;

use crate::{
    common::{ColumnIndex, Map, RowIndex, Value},
    rule::Constraint,
    types::{MergeFn, TableDef, Type},
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModifyState {
    NoModify,
    UnionRow(RowIndex),
    MergeRow(RowIndex),
    NewRow(RowIndex),
}

impl Into<Option<RowIndex>> for ModifyState {
    fn into(self) -> Option<RowIndex> {
        match self {
            ModifyState::NoModify => None,
            ModifyState::MergeRow(row_index)
            | ModifyState::UnionRow(row_index)
            | ModifyState::NewRow(row_index) => Some(row_index),
        }
    }
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

    pub fn is_fn_table(&self) -> bool {
        self.table_def.2.is_some()
    }

    /// Look up a row index by its key columns. Returns None if not found.
    pub fn get_by_key(&self, key: &Row) -> Option<RowIndex> {
        self.key_index.get(key).copied()
    }

    pub fn canonicalize_row(&self, uf: &UnionFind, row: &[Value]) -> Row {
        debug_assert!(row.len() <= self.column_count());
        let mut result_vec = SmallVec::new();
        for idx in 0..row.len() {
            let ty = &self.table_def.1[idx];
            if ty.is_id_type() {
                result_vec.push(uf.find(row[idx]));
            } else {
                result_vec.push(row[idx]);
            }
        }
        Row(result_vec)
    }


    /// Inserts a row. Returns `Some(row_index)` if a new row was added,
    /// `None` if an existing row was updated.
    pub fn insert(&mut self, uf: &mut UnionFind, mut key: Row, value: Value) -> ModifyState {
        debug_assert_eq!(key.0.len(), self.arity());
        let Some(r) = self.key_index.get(&key) else {
            let idx = RowIndex(self.row_count);
            self.key_index.insert(key.clone(), idx);
            key.0.push(value);
            self.rows.extend(&key.0);
            self.row_count += 1;
            return ModifyState::NewRow(idx);
        };

        let column = self.column_count();
        let arity = self.arity();
        let value_idx = r.0 * column + arity;
        let value_ref = &mut self.rows[value_idx];
        if value_ref == &value {
            return ModifyState::NoModify;
        }

        let Some(merge_fn) = &self.table_def.2 else {
            let rhs = if let Some((parent, _child)) = uf.union(*value_ref, value) {
                parent
            } else {
                value
            };
            *value_ref = rhs;
            self.delta_start_row = self.delta_start_row.min(r.0);
            return ModifyState::UnionRow(*r);
        };

        *value_ref = merge_fn.interp(value_ref,&value);

        self.delta_start_row = self.delta_start_row.min(r.0);
        ModifyState::MergeRow(*r)
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
            .map(|row| self.canonicalize_row(uf, row))
            .filter(|row| {
                constraints
                    .iter()
                    .all(|c| c.op.interp(&row.0[c.column.0], &c.value))
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

    #[inline]
    pub fn get_row_key(&self, row_index: RowIndex) -> Row {
        let start = row_index.0 * self.column_count();
        let end = start + self.arity();
        Row(self.rows[start..end].into())
    }

    #[inline]
    pub fn get_row_value(&self, row_index: RowIndex) -> Value {
        let idx = row_index.0 * self.column_count() + self.arity();
        self.rows[idx]
    }

    pub fn get_row_value_mut(&mut self, row_index: RowIndex) -> &mut Value {
        let idx = row_index.0 * self.column_count() + self.arity();
        &mut self.rows[idx]
    }

    #[inline]
    pub fn get_value_type(&self) -> &Type {
        &self.table_def.1[self.arity()]
    }

    #[inline]
    pub fn get_canonicalized_row_value(&self, uf: &UnionFind, row_index: RowIndex) -> Value {
        let v = self.get_row_value(row_index);
        if self.get_value_type().is_id_type() {
            uf.find(v)
        } else {
            v
        }
    }

}
