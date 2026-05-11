use alloc::string::String;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::regraph::types::BaseType;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Value(pub u32);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowIndex(pub usize);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColumnIndex(pub usize);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VarId(pub usize);

// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
// pub struct VarName(pub Name);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeName(pub Name);

pub type TableName = Name;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConstructorName(pub Name);

pub type Name = String;

pub type Map<K, V> = FxHashMap<K, V>;
pub type Set<K> = FxHashSet<K>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Atom {
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    U8(u8),
    U16(u16),
    U32(u32),

    I64(i64),
    U64(u64),
    // F32(f32),
    // F64(f64),
    Str(String),
}

impl Atom {
    pub fn get_type(&self) -> BaseType {
        match self {
            Atom::Bool(_) => BaseType::I1,
            Atom::I8(_) => BaseType::I8,
            Atom::I16(_) => BaseType::I16,
            Atom::I32(_) => BaseType::I32,
            Atom::U8(_) => BaseType::U8,
            Atom::U16(_) => BaseType::U16,
            Atom::U32(_) => BaseType::U32,
            Atom::I64(_) => BaseType::I64,
            Atom::U64(_) => BaseType::U64,
            Atom::Str(_) => BaseType::Str,
        }
    }

    pub fn to_value(self) -> Value {
        match self {
            Atom::I8(i) => Value(i as u32),
            Atom::I16(i) => Value(i as u32),
            Atom::U8(u) => Value(u as u32),
            Atom::U16(u) => Value(u as u32),
            Atom::I32(i) => Value(i as u32),
            Atom::U32(u) => Value(u),
            Atom::I64(_) => unimplemented!("unimplement I64 Atom to value"),
            Atom::U64(_) => unimplemented!("unimplement U64 Atom to value"),
            Atom::Bool(b) => Value(if b { 1u32 } else { 0u32 }),
            Atom::Str(_) => unimplemented!("unimplement String Atom to value"),
        }
    }
}
