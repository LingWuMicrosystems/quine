use alloc::string::String;
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Value(pub u32);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowIndex(pub usize);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColumnIndex(pub usize);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Variable(pub usize);

// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
// pub struct VarName(pub Name);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeName(pub Name);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TableName(pub Name);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConstructorName(pub Name);

pub type Name = String;

pub type Map<K, V> = FxHashMap<K, V>;
pub type Set<K> = FxHashSet<K>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Atom {
    Int(i64),
    Uint(u64),
    Bool(bool),
    Str(String),
}

impl Atom {
    pub fn to_value(self) -> Value {
        match self {
            Atom::Int(i) => {
                let r: i32 = i.try_into().expect("too large");
                Value(r as u32)
            }
            Atom::Uint(u) => {
                let r: u32 = u.try_into().expect("too large");
                Value(r)
            }
            Atom::Bool(b) => Value(if b { 1u32 } else { 0u32 }),
            Atom::Str(_) => unimplemented!("unimplement String Atom to value"),
        }
    }
}
