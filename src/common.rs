use rustc_hash::FxHashMap;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(pub u32);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowIndex(pub usize);

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct ColumnIndex(pub usize);

pub type Map<K, V> = FxHashMap<K, V>;
