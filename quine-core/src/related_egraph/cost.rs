use alloc::string::String;

use crate::{
    common::{Map, RowIndex, Value},
    table::Table,
    types::{BaseType, Type},
    uf::UnionFind,
};

/// Cost tracking for e-graph nodes.
///
/// Maintains:
/// - Constructor cost models ("TypeName.ConsName" -> u64)
/// - Per-eclass minimum cost (eclass_cost)
/// - Which enode achieves that minimum (cost_select)
///
/// ## Cost lattice
///
/// (u64, ⊑, ⊥, ⊤, ⊔)
///
/// | Concept | Value | Meaning |
/// |---------|-------|---------|
/// | Partial order (⊑) | a >= b | Cheaper = more precise |
/// | Join (⊔) | min(a, b) | Cheaper wins |
/// | Bottom (⊥) | u64::MAX | Unknown cost (identity: min(MAX, x) = x) |
/// | Top (⊤) | 0 | Fully known, cheapest possible |
/// | Addition (+) | saturating_add | MAX + anything = MAX (unknown propagates) |
///
/// Fixed-point: costs monotonically move from ⊥ toward ⊤ (decrease numerically).
/// Initialized lazily — absent key means ⊥ (u64::MAX).
#[derive(Debug, Clone)]
pub struct CostTracker {
    /// Constructor cost models: "TypeName.ConsName" -> u64
    pub cost_models: Map<String, u64>,
    /// Per-eclass minimum cost. Absent = u64::MAX (⊥). Monotonically decreases.
    eclass_cost: Map<Value, u64>,
    /// Which enode achieves eclass_cost[eclass]. None if eclass has no enodes.
    cost_select: Map<Value, (usize, RowIndex)>,
}

impl Default for CostTracker {
    fn default() -> Self {
        Self {
            cost_models: Map::default(),
            eclass_cost: Map::default(),
            cost_select: Map::default(),
        }
    }
}

impl CostTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a cost model entry: "TypeName.ConsName" -> cost
    pub fn set_cost_model(&mut self, name: String, cost: u64) {
        self.cost_models.insert(name, cost);
    }

    /// Look up the cost of a constructor. Returns 0 if not defined.
    pub fn get_constructor_cost(&self, table_name: &str) -> u64 {
        self.cost_models.get(table_name).copied().unwrap_or(0)
    }

    /// Get the current minimum cost of an eclass. Returns u64::MAX (⊥) if unknown.
    pub fn eclass_cost(&self, union_find: &UnionFind, eclass: Value) -> u64 {
        let canonical = union_find.find(eclass);
        self.eclass_cost.get(&canonical).copied().unwrap_or(u64::MAX)
    }

    /// Get the cheapest enode for an eclass, if any.
    pub fn cost_select(
        &self,
        union_find: &UnionFind,
        eclass: Value,
    ) -> Option<(usize, RowIndex)> {
        let canonical = union_find.find(eclass);
        self.cost_select.get(&canonical).copied()
    }

    /// Compute the cost of the enode at (table_id, row_idx) and update
    /// eclass_cost / cost_select if it's cheaper than the current minimum.
    pub fn compute_and_update_eclass_cost(
        &mut self,
        tables: &[Table],
        union_find: &UnionFind,
        table_id: usize,
        row_idx: RowIndex,
    ) {
        let table = &tables[table_id];
        let col = table.column_count();
        let arity = table.arity();
        let start = row_idx.0 * col;
        let value = table.rows[start + arity];
        let canonical = union_find.find(value);

        let constructor_cost = self
            .cost_models
            .get(&table.table_def.0)
            .copied()
            .unwrap_or(0);

        let mut enode_cost = constructor_cost;
        for i in 0..arity {
            let child_ty = &table.table_def.1[i];
            if matches!(child_ty, Type::Name(_) | Type::Base(BaseType::Id)) {
                let child_canon = union_find.find(table.rows[start + i]);
                enode_cost = enode_cost.saturating_add(
                    self.eclass_cost.get(&child_canon).copied().unwrap_or(u64::MAX),
                );
            }
        }

        let old = self.eclass_cost.get(&canonical).copied().unwrap_or(u64::MAX);
        if enode_cost < old {
            self.eclass_cost.insert(canonical, enode_cost);
            self.cost_select.insert(canonical, (table_id, row_idx));
        }
    }

    /// Merge child eclass cost into parent: take min, keep cheaper cost_select.
    pub fn merge_eclass_cost(&mut self, parent: Value, child: Value) {
        let child_cost = self.eclass_cost.remove(&child).unwrap_or(u64::MAX);
        let parent_entry = self.eclass_cost.entry(parent).or_insert(u64::MAX);
        if child_cost < *parent_entry {
            *parent_entry = child_cost;
            if let Some(select) = self.cost_select.remove(&child) {
                self.cost_select.insert(parent, select);
            }
        } else {
            self.cost_select.remove(&child);
        }
    }

    /// Redirect cost_select from one enode to another (rebuild D1).
    pub fn cost_select_redirect(
        &mut self,
        canonical: Value,
        from: (usize, RowIndex),
        to: (usize, RowIndex),
    ) {
        if self.cost_select.get(&canonical) == Some(&from) {
            self.cost_select.insert(canonical, to);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn lattice_join_bottom_identity() {
        // min(u64::MAX, x) == x  (⊥ ⊔ x = x)
        assert_eq!(core::cmp::min(u64::MAX, 42u64), 42);
        assert_eq!(core::cmp::min(u64::MAX, 0u64), 0);
    }

    #[test]
    fn lattice_join_bottom_bottom() {
        // min(u64::MAX, u64::MAX) == u64::MAX  (⊥ ⊔ ⊥ = ⊥)
        assert_eq!(core::cmp::min(u64::MAX, u64::MAX), u64::MAX);
    }

    #[test]
    fn lattice_join_cheaper_wins() {
        // min = cheaper (lower numeric cost)
        assert_eq!(core::cmp::min(5u64, 3u64), 3);
        assert_eq!(core::cmp::min(10u64, 10u64), 10);
    }

    #[test]
    fn saturating_add_propagates_unknown() {
        // u64::MAX.saturating_add(x) == u64::MAX (⊥ propagates)
        assert_eq!(u64::MAX.saturating_add(5), u64::MAX);
        assert_eq!(u64::MAX.saturating_add(0), u64::MAX);
    }

    #[test]
    fn saturating_add_normal() {
        // Normal cost propagation works
        assert_eq!(5u64.saturating_add(10), 15);
        assert_eq!(0u64.saturating_add(0), 0);
    }
}
