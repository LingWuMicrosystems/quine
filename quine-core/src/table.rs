use alloc::vec;
use alloc::vec::Vec;

use smallvec::SmallVec;

use crate::{
    common::{ColumnIndex, Map, RowIndex, Value},
    rule::{Constraint, Op},
    types::{BaseType, TableDef, Type},
    uf::UnionFind,
};

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Row(pub SmallVec<[Value; 6]>);

#[derive(Debug, Clone, PartialEq)]
pub enum Column {
    Id(Vec<Value>),  // e-class IDs
    Str(Vec<Value>), // interned string IDs
    I1(Vec<bool>),   // 0 or 1 per bool, can be optimized to bitmap later
    I8(Vec<i8>),
    U8(Vec<u8>),
    I16(Vec<i16>),
    U16(Vec<u16>),
    I32(Vec<i32>),
    U32(Vec<u32>),
    I64(Vec<i64>),
    U64(Vec<u64>),

    F32(Vec<f32>),
    F64(Vec<f64>),
}

impl Column {
    pub fn from_base_type(base_type: &BaseType) -> Self {
        match base_type {
            BaseType::Id => Column::Id(Vec::new()),
            BaseType::Str => Column::Str(Vec::new()),
            BaseType::I1 => Column::I1(Vec::new()),
            BaseType::I8 => Column::I8(Vec::new()),
            BaseType::U8 => Column::U8(Vec::new()),
            BaseType::I16 => Column::I16(Vec::new()),
            BaseType::U16 => Column::U16(Vec::new()),
            BaseType::I32 => Column::I32(Vec::new()),
            BaseType::U32 => Column::U32(Vec::new()),
            BaseType::I64 => Column::I64(Vec::new()),
            BaseType::U64 => Column::U64(Vec::new()),
            BaseType::F32 => Column::F32(Vec::new()),
            BaseType::F64 => Column::F64(Vec::new()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Table {
    pub table_def: TableDef,
    pub rows: Vec<Value>,
    pub key_index: Map<Row, RowIndex>,
    pub parents: Map<Value, Vec<RowIndex>>,
    pub row_count: usize,
}

impl Table {
    pub fn new(table_def: TableDef) -> Self {
        Self {
            table_def,
            rows: vec![],
            key_index: Default::default(),
            parents: Default::default(),
            row_count: Default::default(),
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

    pub fn insert(&mut self, uf: &mut UnionFind, mut key: Row, value: Value) {
        debug_assert_eq!(key.0.len(), self.arity());
        if let Some(r) = self.key_index.get(&key) {
            // TODO: Conflict Merge!
            let column = self.column_count();
            let arity = self.arity();
            let value_ref = &mut self.rows[r.0 * column + arity];
            if let Some((_old, value)) = uf.union(*value_ref, value) {
                *value_ref = value;
            } else {
                *value_ref = value;
            };
        } else {
            // update key_index
            self.key_index.insert(key.clone(), RowIndex(self.row_count));

            // insert
            key.0.push(value);
            self.rows.extend(&key.0);
        }
        self.row_count += 1;
    }

    pub fn fused_scan(
        &self,
        uf: &UnionFind,
        find_columns: &[ColumnIndex],
        constraints: &[Constraint],
    ) -> impl Iterator<Item = Row> {
        self.rows
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

    // pub fn get_result(&self, row_index: RowIndex) -> Value {
    //     todo!()
    // }

    // pub fn get(&self, column_index: ColumnIndex, row_index: RowIndex) -> Value {
    //     match &self.columns[column_index.0] {
    //         Column::Id(values) => values.get(row_index.0).copied().unwrap(),
    //         Column::Str(values) => values.get(row_index.0).copied().unwrap(),
    //         Column::I1(items) => Value(items.get(row_index.0).copied().unwrap() as _),
    //         Column::I8(items) => Value(items.get(row_index.0).copied().unwrap() as _),
    //         Column::U8(items) => Value(items.get(row_index.0).copied().unwrap() as _),
    //         Column::I16(items) => Value(items.get(row_index.0).copied().unwrap() as _),
    //         Column::U16(items) => Value(items.get(row_index.0).copied().unwrap() as _),
    //         Column::I32(items) => Value(items.get(row_index.0).copied().unwrap() as _),
    //         Column::U32(items) => Value(items.get(row_index.0).copied().unwrap() as _),
    //         Column::I64(items) => Value(items.get(row_index.0).copied().unwrap() as _),
    //         Column::U64(items) => Value(items.get(row_index.0).copied().unwrap() as _),
    //         Column::F32(items) => Value(items.get(row_index.0).copied().unwrap() as _),
    //         Column::F64(items) => Value(items.get(row_index.0).copied().unwrap() as _),
    //     }
    // }

    // pub fn get_row_and_result(&self, idx: RowIndex) -> (Row, Value) {
    //     let row = self.get_all_row(idx);
    //     let result = row[row.len() - 1];
    //     (Row(row[..row.len() - 1].to_smallvec()), result)
    // }

    // pub fn get_row(&self, idx: RowIndex) -> Row {
    //     let row_size = self.row_size();
    //     let start = idx.0 * row_size;
    //     let row = &self.rows[start..start + row_size];
    //     Row(row.to_smallvec())
    // }
}
