use alloc::string::String;
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(pub u32);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowIndex(pub usize);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColumnIndex(pub usize);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Variable(pub usize);

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
