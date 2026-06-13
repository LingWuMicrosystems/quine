use crate::{
    common::{Map, RowIndex, Set, Value},
    related_egraph::TableId,
};

/// Tracks eclass → enode reference mappings.
///
/// Maps each canonical eclass value to the set of `(table_id, row_index)`
/// enodes that reference it. Maintained through insert, union, and rebuild.
///
/// ## Type guard
///
/// Only eclass-typed value columns (`Type::Name(_)` or `Type::Base(BaseType::Id)`)
/// are tracked. Literal-typed columns are excluded (Decision #1).
/// The type check remains at call sites — ReverseIndex does not inspect types.
#[derive(Debug, Default, Clone)]
pub struct ReverseIndex {
    index: Map<Value, Set<(TableId, RowIndex)>>,
}

impl ReverseIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Track an enode reference for a canonical eclass value.
    ///
    /// Called after insert when a new row's value column is eclass-typed.
    pub fn insert(&mut self, canonical: Value, table_id: TableId, row_idx: RowIndex) {
        self.index
            .entry(canonical)
            .or_default()
            .insert((table_id, row_idx));
    }

    /// Merge child eclass enode references into parent.
    ///
    /// Called on union: child's entries move to parent, child key is removed.
    pub fn merge(&mut self, parent: Value, child: Value) {
        if let Some(child_entries) = self.index.remove(&child) {
            self.index
                .entry(parent)
                .or_default()
                .extend(child_entries);
        }
    }

    /// Remove a specific enode reference.
    ///
    /// Called during rebuild D1 when an absorbed row's reference is removed.
    pub fn remove(&mut self, canonical: Value, table_id: TableId, row_idx: RowIndex) {
        self.index
            .entry(canonical)
            .and_modify(|s| {
                s.remove(&(table_id, row_idx));
            });
    }

    /// Get all enode references for a canonical eclass.
    ///
    /// Returns an empty set if the eclass has no tracked references.
    pub fn get(&self, canonical: Value) -> Set<(TableId, RowIndex)> {
        self.index.get(&canonical).cloned().unwrap_or_default()
    }
}
