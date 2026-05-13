use std::boxed::Box;

use alloc::vec::Vec;
use smallvec::SmallVec;

use crate::regraph::{
    common::{ColumnIndex, Map, RowIndex, Set, Value},
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
    // pub rows: Vec<Value>,
    pub columns: Box<[Column]>,
    pub key_index: Map<Row, RowIndex>,
    pub parents: Map<Value, Vec<RowIndex>>,
    pub row_count: usize,
}

impl Table {
    pub fn new(table_def: TableDef) -> Self {
        let mut col_types: Vec<_> = table_def.1.iter().map(Type::to_base_type).collect();
        col_types.push(
            table_def
                .2
                .as_ref()
                .map(Type::to_base_type)
                .unwrap_or(BaseType::Id),
        );
        let columns = col_types.iter().map(Column::from_base_type).collect();
        Self {
            columns,
            key_index: Default::default(),
            parents: Default::default(),
            row_count: Default::default(),
        }
    }

    pub fn arity(&self) -> usize {
        self.column_count() - 1
    }

    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    pub fn row_count(&self) -> usize {
        self.row_count
    }

    pub fn insert_row(&mut self, row: Row) {
        debug_assert_eq!(row.0.len(), self.column_count());
        self.row_count += 1;
        for (col, val) in self.columns.iter_mut().zip(row.0.iter()) {
            match col {
                Column::Id(values) => values.push(*val),
                Column::Str(values) => values.push(*val),
                Column::I1(items) => items.push(val.0 != 0),
                Column::I8(items) => items.push(val.0 as _),
                Column::U8(items) => items.push(val.0 as _),
                Column::I16(items) => items.push(val.0 as _),
                Column::U16(items) => items.push(val.0 as _),
                Column::I32(items) => items.push(val.0 as _),
                Column::U32(items) => items.push(val.0 as _),
                Column::I64(items) => items.push(val.0 as _),
                Column::U64(items) => items.push(val.0),
                Column::F32(items) => items.push(val.0 as _),
                Column::F64(items) => items.push(val.0 as _),
            }
        }
    }

    pub fn fused_scan(
        &self,
        uf: &UnionFind,
        find_column: ColumnIndex,
        cs: &[Constraint],
    ) -> Set<Value> {
        match &self.columns[find_column.0] {
            Column::Id(values) => values
                .iter()
                .filter(|v| {
                    cs.iter().all(|c| {
                        let v = uf.find(**v);
                        let c_v = c.value;
                        match c.op {
                            Op::Equ => v == c_v,
                            Op::Neq => v != c_v,
                            Op::Lt => unreachable!(""),
                            Op::Gt => unreachable!(""),
                            Op::Leq => unreachable!(""),
                            Op::Geq => unreachable!(""),
                        }
                    })
                })
                .cloned()
                .collect(),
            Column::Str(values) => values
                .iter()
                .filter(|v| {
                    cs.iter().all(|c| {
                        let v = **v;
                        let c_v = c.value;
                        match c.op {
                            Op::Equ => v == c_v,
                            Op::Neq => v != c_v,
                            Op::Lt => unreachable!(""),
                            Op::Gt => unreachable!(""),
                            Op::Leq => unreachable!(""),
                            Op::Geq => unreachable!(""),
                        }
                    })
                })
                .cloned()
                .collect(),
            Column::I1(items) => items
                .iter()
                .filter(|v| {
                    cs.iter().all(|c| {
                        let v = **v;
                        let c_v = c.value.0 != 0;
                        match c.op {
                            Op::Equ => v == c_v,
                            Op::Neq => v != c_v,
                            Op::Lt => v < c_v,
                            Op::Gt => v > c_v,
                            Op::Leq => v <= c_v,
                            Op::Geq => v >= c_v,
                        }
                    })
                })
                .map(|x| Value((*x) as u64))
                .collect(),
            Column::I8(items) => items
                .iter()
                .filter(|v| {
                    cs.iter().all(|c| {
                        let v = **v;
                        let c_v = c.value.0 as i8;
                        match c.op {
                            Op::Equ => v == c_v,
                            Op::Neq => v != c_v,
                            Op::Lt => v < c_v,
                            Op::Gt => v > c_v,
                            Op::Leq => v <= c_v,
                            Op::Geq => v >= c_v,
                        }
                    })
                })
                .map(|x| Value((*x) as u64))
                .collect(),
            Column::U8(items) => items
                .iter()
                .filter(|v| {
                    cs.iter().all(|c| {
                        let v = **v;
                        let c_v = c.value.0 as u8;
                        match c.op {
                            Op::Equ => v == c_v,
                            Op::Neq => v != c_v,
                            Op::Lt => v < c_v,
                            Op::Gt => v > c_v,
                            Op::Leq => v <= c_v,
                            Op::Geq => v >= c_v,
                        }
                    })
                })
                .map(|x| Value((*x) as u64))
                .collect(),
            Column::I16(items) => items
                .iter()
                .filter(|v| {
                    cs.iter().all(|c| {
                        let v = **v;
                        let c_v = c.value.0 as i16;
                        match c.op {
                            Op::Equ => v == c_v,
                            Op::Neq => v != c_v,
                            Op::Lt => v < c_v,
                            Op::Gt => v > c_v,
                            Op::Leq => v <= c_v,
                            Op::Geq => v >= c_v,
                        }
                    })
                })
                .map(|x| Value((*x) as u64))
                .collect(),
            Column::U16(items) => items
                .iter()
                .filter(|v| {
                    cs.iter().all(|c| {
                        let v = **v;
                        let c_v = c.value.0 as u16;
                        match c.op {
                            Op::Equ => v == c_v,
                            Op::Neq => v != c_v,
                            Op::Lt => v < c_v,
                            Op::Gt => v > c_v,
                            Op::Leq => v <= c_v,
                            Op::Geq => v >= c_v,
                        }
                    })
                })
                .map(|x| Value((*x) as u64))
                .collect(),
            Column::I32(items) => items
                .iter()
                .filter(|v| {
                    cs.iter().all(|c| {
                        let v = **v;
                        let c_v = c.value.0 as i32;
                        match c.op {
                            Op::Equ => v == c_v,
                            Op::Neq => v != c_v,
                            Op::Lt => v < c_v,
                            Op::Gt => v > c_v,
                            Op::Leq => v <= c_v,
                            Op::Geq => v >= c_v,
                        }
                    })
                })
                .map(|x| Value((*x) as u64))
                .collect(),
            Column::U32(items) => items
                .iter()
                .filter(|v| {
                    cs.iter().all(|c| {
                        let v = **v;
                        let c_v = c.value.0 as u32;
                        match c.op {
                            Op::Equ => v == c_v,
                            Op::Neq => v != c_v,
                            Op::Lt => v < c_v,
                            Op::Gt => v > c_v,
                            Op::Leq => v <= c_v,
                            Op::Geq => v >= c_v,
                        }
                    })
                })
                .map(|x| Value((*x) as u64))
                .collect(),
            Column::I64(items) => items
                .iter()
                .filter(|v| {
                    cs.iter().all(|c| {
                        let v = **v;
                        let c_v = c.value.0 as i64;
                        match c.op {
                            Op::Equ => v == c_v,
                            Op::Neq => v != c_v,
                            Op::Lt => v < c_v,
                            Op::Gt => v > c_v,
                            Op::Leq => v <= c_v,
                            Op::Geq => v >= c_v,
                        }
                    })
                })
                .map(|x| Value((*x) as u64))
                .collect(),
            Column::U64(items) => items
                .iter()
                .filter(|v| {
                    cs.iter().all(|c| {
                        let v = **v;
                        let c_v = c.value.0 as u64;
                        match c.op {
                            Op::Equ => v == c_v,
                            Op::Neq => v != c_v,
                            Op::Lt => v < c_v,
                            Op::Gt => v > c_v,
                            Op::Leq => v <= c_v,
                            Op::Geq => v >= c_v,
                        }
                    })
                })
                .map(|x| Value((*x) as u64))
                .collect(),
            Column::F32(items) => items
                .iter()
                .filter(|v| {
                    cs.iter().all(|c| {
                        let v = **v;
                        let c_v = c.value.0 as f32;
                        match c.op {
                            Op::Equ => v == c_v,
                            Op::Neq => v != c_v,
                            Op::Lt => v < c_v,
                            Op::Gt => v > c_v,
                            Op::Leq => v <= c_v,
                            Op::Geq => v >= c_v,
                        }
                    })
                })
                .map(|x| Value((*x) as u64))
                .collect(),
            Column::F64(items) => items
                .iter()
                .filter(|v| {
                    cs.iter().all(|c| {
                        let v = **v;
                        let c_v = c.value.0 as f64;
                        match c.op {
                            Op::Equ => v == c_v,
                            Op::Neq => v != c_v,
                            Op::Lt => v < c_v,
                            Op::Gt => v > c_v,
                            Op::Leq => v <= c_v,
                            Op::Geq => v >= c_v,
                        }
                    })
                })
                .map(|x| Value((*x) as u64))
                .collect(),
        }
    }

    #[inline]
    pub fn get_all_row(&self, row_index: RowIndex) -> Row {
        Row(self
            .columns
            .iter()
            .map(|col| match col {
                Column::Id(values) => values.get(row_index.0).copied().unwrap(),
                Column::Str(values) => values.get(row_index.0).copied().unwrap(),
                Column::I1(items) => Value(items.get(row_index.0).copied().unwrap() as _),
                Column::I8(items) => Value(items.get(row_index.0).copied().unwrap() as _),
                Column::U8(items) => Value(items.get(row_index.0).copied().unwrap() as _),
                Column::I16(items) => Value(items.get(row_index.0).copied().unwrap() as _),
                Column::U16(items) => Value(items.get(row_index.0).copied().unwrap() as _),
                Column::I32(items) => Value(items.get(row_index.0).copied().unwrap() as _),
                Column::U32(items) => Value(items.get(row_index.0).copied().unwrap() as _),
                Column::I64(items) => Value(items.get(row_index.0).copied().unwrap() as _),
                Column::U64(items) => Value(items.get(row_index.0).copied().unwrap() as _),
                Column::F32(items) => Value(items.get(row_index.0).copied().unwrap() as _),
                Column::F64(items) => Value(items.get(row_index.0).copied().unwrap() as _),
            })
            .collect())
    }

    pub fn get_result(&self, row_index: RowIndex) -> Value {
        let column_index = ColumnIndex(self.columns.len() - 1);
        self.get(column_index, row_index)
    }

    pub fn get(&self, column_index: ColumnIndex, row_index: RowIndex) -> Value {
        match &self.columns[column_index.0] {
            Column::Id(values) => values.get(row_index.0).copied().unwrap(),
            Column::Str(values) => values.get(row_index.0).copied().unwrap(),
            Column::I1(items) => Value(items.get(row_index.0).copied().unwrap() as _),
            Column::I8(items) => Value(items.get(row_index.0).copied().unwrap() as _),
            Column::U8(items) => Value(items.get(row_index.0).copied().unwrap() as _),
            Column::I16(items) => Value(items.get(row_index.0).copied().unwrap() as _),
            Column::U16(items) => Value(items.get(row_index.0).copied().unwrap() as _),
            Column::I32(items) => Value(items.get(row_index.0).copied().unwrap() as _),
            Column::U32(items) => Value(items.get(row_index.0).copied().unwrap() as _),
            Column::I64(items) => Value(items.get(row_index.0).copied().unwrap() as _),
            Column::U64(items) => Value(items.get(row_index.0).copied().unwrap() as _),
            Column::F32(items) => Value(items.get(row_index.0).copied().unwrap() as _),
            Column::F64(items) => Value(items.get(row_index.0).copied().unwrap() as _),
        }
    }

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
