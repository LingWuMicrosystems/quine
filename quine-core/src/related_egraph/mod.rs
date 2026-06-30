/// related e-graph
use alloc::string::String;
use alloc::vec::Vec;
use smallvec::SmallVec;

#[cfg(feature = "std")]
use rayon::prelude::*;

mod action;
mod cost;
mod reverse_index;

use crate::{
    common::{ColumnIndex, Map, RowIndex, Set, Value, VarId},
    related_egraph::action::ActionCtx,
    rule::{
        Action, Constraint, CrossConstraint, Op, Query, Rule, ScanStep,
    },
    table::{ Row, Table},
    types::{BaseType, TableDef, Type},
    uf::UnionFind,
};

pub type TableId = usize;
pub type RuleId = usize;
pub type GroupName = String;

pub type NativeFn = fn(input: &[Value]) -> Value;

pub type RuleGroup = Set<RuleId>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RunMode {
    Saturate,
    Repeat(usize),
}

#[derive(Debug, Default, Clone)]
pub struct RelatedEGraph {
    action: ActionCtx,
    ruleset: Vec<Rule>,
    rule_deps: Map<TableId, Vec<RuleId>>,
    pub rule_groups: Map<GroupName, RuleGroup>,
}

impl RelatedEGraph {
    pub fn add_table(&mut self, table_def: TableDef) {
        self.action.tables.push(Table::new(table_def));
    }

    pub fn add_rule(&mut self, group_name: Option<GroupName>, rule: Rule) {
        let rule_id = self.ruleset.len();
        for t in rule.query.tables().iter() {
            self.rule_deps.entry(*t).or_default().push(rule_id);
        }
        self.ruleset.push(rule);
        if let Some(group_name) = group_name {
            self.rule_groups
                .entry(group_name)
                .or_default()
                .insert(rule_id);
        }
    }

    /// Semi-naive fixpoint iteration.
    ///
    /// Repeatedly fires rules in `rule_filter` (all rules if `None`)
    /// until no table has delta rows (fixpoint), or until `Repeat(n)`
    /// iteration limit is reached.
    ///
    /// Returns `true` if fixpoint was reached, `false` if truncated by Repeat.
    pub fn run_semi_naive(&mut self, rule_filter: Option<&RuleGroup>, mode: RunMode) -> bool {
        let mut iteration = 0;
        loop {
            // Collect (driver_table, rule) pairs for tables that have delta rows
            let pairs: Vec<(TableId, RuleId)> = (0..self.action.tables.len())
                .filter(|tid| self.action.tables[*tid].has_delta())
                .flat_map(|tid| {
                    self.rule_deps
                        .get(&tid)
                        .into_iter()
                        .flatten()
                        .filter(|rid| rule_filter.is_none_or(|r| r.contains(rid)))
                        .map(move |rid| (tid, *rid))
                })
                .collect::<Set<(TableId, RuleId)>>()
                .into_iter()
                .collect();

            if pairs.is_empty() {
                return true;
            }

            // Snapshot current row counts so new delta only includes rows added this round
            let snapshots: Vec<usize> = self.action.tables.iter().map(|t| t.row_count).collect();

            // Phase 1: semi-naive queries (parallel with std feature)
            #[cfg(feature = "std")]
            let results: Vec<Set<Row>> = pairs
                .par_iter()
                .map(|(driver_table, rule_id)| {
                    let query = &self.ruleset[*rule_id].query;
                    self.run_query(query, Some(*driver_table))
                })
                .collect();

            #[cfg(not(feature = "std"))]
            let results: Vec<Set<Row>> = pairs
                .iter()
                .map(|(driver_table, rule_id)| {
                    let query = &self.ruleset[*rule_id].query;
                    self.run_query(query, Some(*driver_table))
                })
                .collect();

            // Phase 2: apply actions (always serial)
            for ((_driver_table, rule_id), rows) in pairs.iter().zip(results) {
                let action = &self.ruleset.get(*rule_id).unwrap().action;

                self.action.apply_action(action, rows);
            }

            let rebuild_affected = self.rebuild();

            // New delta = rows added since snapshot,
            // unless rebuild already reset it to 0 for a full re-scan.
            for (tid, (table, &snapshot)) in
                self.action.tables.iter_mut().zip(&snapshots).enumerate()
            {
                if !rebuild_affected.contains(&tid) {
                    table.delta_start_row = snapshot;
                }
            }

            iteration += 1;
            if matches!(mode, RunMode::Repeat(n) if iteration >= n) {
                return false;
            }
        }
    }

    pub fn apply_action(&mut self, action: &Action, rows: Set<Row>) {
        self.action.apply_action(action, rows);
    }

    pub fn run_query(&self, query: &Query, delta_table: Option<TableId>) -> Set<Row> {
        if query.scan_steps.is_empty() {
            return Set::default();
        }

        // Stage 1: initial scan.
        let step0 = &query.scan_steps[0];
        let mut rows: Vec<Row> = self.action.tables[step0.table]
            .fused_scan(
                &self.action.union_find,
                &step0.columns,
                &step0.constraints,
                delta_table == Some(step0.table),
            )
            .collect();
        let mut result_vars = step0.var_binding.clone();

        // Stage 2: for each subsequent step — scan, join, extend.
        for step in &query.scan_steps[1..] {
            let shared: Vec<(usize, usize)> = step
                .var_binding
                .iter()
                .enumerate()
                .filter_map(|(sp, v)| result_vars.iter().position(|rv| rv == v).map(|rp| (sp, rp)))
                .collect();
            let result_vars_set: Set<VarId> = result_vars.iter().copied().collect();
            let new_cols: Vec<usize> = step
                .var_binding
                .iter()
                .enumerate()
                .filter(|(_, v)| !result_vars_set.contains(v))
                .map(|(i, _)| i)
                .collect();

            // Sideways information passing: push-down constraints.
            let next_rows = if shared.len() == 1 {
                let (sp, rp) = shared[0];
                let distinct: Set<Value> = rows.iter().map(|r| r.0[rp]).collect();
                self.scan_step_table(step, delta_table, Some((step.columns[sp], &distinct)))
            } else {
                self.scan_step_table(step, delta_table, None)
            };

            rows = Self::join_step_rows(rows, next_rows, &shared, &new_cols);

            for &si in &new_cols {
                result_vars.push(step.var_binding[si]);
            }
        }

        // Stage 3: filter and permute.
        Self::filter_and_permute(rows, &query.constraints, &result_vars)
    }

    /// Scan a single query step's table, optionally with pushed-down
    /// equality constraints for sideways information passing.
    fn scan_step_table(
        &self,
        step: &ScanStep,
        delta_table: Option<TableId>,
        filter: Option<(ColumnIndex, &Set<Value>)>,
    ) -> Vec<Row> {
        let table = &self.action.tables[step.table];
        if let Some((col, distinct)) = filter {
            distinct
                .iter()
                .flat_map(|&v| {
                    let mut constraints = step.constraints.clone();
                    constraints.push(Constraint {
                        op: Op::Equ,
                        column: col,
                        value: v,
                    });
                    table
                        .fused_scan(
                            &self.action.union_find,
                            &step.columns,
                            &constraints,
                            delta_table == Some(step.table),
                        )
                        .collect::<Vec<_>>()
                })
                .collect()
        } else {
            table
                .fused_scan(
                    &self.action.union_find,
                    &step.columns,
                    &step.constraints,
                    delta_table == Some(step.table),
                )
                .collect()
        }
    }

    /// Join two row sets on shared variables: cross-product when no
    /// variables are shared, hash join otherwise.
    fn join_step_rows(
        rows: Vec<Row>,
        next_rows: Vec<Row>,
        shared: &[(usize, usize)],
        new_cols: &[usize],
    ) -> Vec<Row> {
        if shared.is_empty() {
            let mut new_rows = Vec::with_capacity(rows.len() * next_rows.len());
            for left in &rows {
                for right in &next_rows {
                    let mut r = left.clone();
                    for &si in new_cols {
                        r.0.push(right.0[si]);
                    }
                    new_rows.push(r);
                }
            }
            new_rows
        } else {
            let mut hash: Map<SmallVec<[Value; 4]>, Vec<&Row>> = Map::default();
            for right in &next_rows {
                let key: SmallVec<[Value; 4]> = shared.iter().map(|(sp, _)| right.0[*sp]).collect();
                hash.entry(key).or_default().push(right);
            }
            let mut new_rows = Vec::with_capacity(rows.len());
            for left in &rows {
                let key: SmallVec<[Value; 4]> = shared.iter().map(|(_, rp)| left.0[*rp]).collect();
                if let Some(matches) = hash.get(&key) {
                    for right in matches {
                        let mut r = left.clone();
                        for &si in new_cols {
                            r.0.push(right.0[si]);
                        }
                        new_rows.push(r);
                    }
                }
            }
            new_rows
        }
    }

    /// Filter rows by cross-constraints and permute columns into VarId
    /// order, producing the final result set.
    fn filter_and_permute(
        rows: Vec<Row>,
        constraints: &[CrossConstraint],
        result_vars: &[VarId],
    ) -> Set<Row> {
        // Build permutation: VarId -> current column position.
        // After join, columns are in discovery order, but VarId::resolve
        // indexes by VarId directly. Reorder so position i = VarId(i).
        let var_count = result_vars.iter().map(|v| v.0).max().map_or(0, |m| m + 1);
        let mut var_to_pos: SmallVec<[usize; 8]> = (0..var_count).collect();
        for (pos, var) in result_vars.iter().enumerate() {
            var_to_pos[var.0] = pos;
        }

        rows.into_iter()
            .filter(|row| {
                constraints.iter().all(|cs| {
                    let Some(lhs) = row.0.get(var_to_pos[cs.lhs.0]) else {
                        return true;
                    };
                    let Some(rhs) = row.0.get(var_to_pos[cs.rhs.0]) else {
                        return true;
                    };
                    cs.op.interp(lhs, rhs)
                })
            })
            .map(|row| Row(var_to_pos.iter().map(|&p| row.0[p]).collect()))
            .collect()
    }

    pub fn insert(&mut self, table_id: usize, key: Row, value: Value) {
        self.action.insert(table_id, key, value);
    }

    pub fn union(&mut self, old: Value, new: Value) {
        self.action.union(old, new);
    }

    pub fn rebuild(&mut self) -> Set<TableId> {
        let mut affected: Map<TableId, usize> = Map::default();
        while let Some((_parent, child)) = self.action.pending_unions.pop() {
            for tid in 0..self.action.tables.len() {
                let table = &self.action.tables[tid];
                let Some(indices) = table.parents.get(&child) else {
                    continue;
                };

                // Preserve scanned_idx so we can remove merged rows from reverse_index.
                let pairs: Vec<_> = indices
                    .iter()
                    .filter_map(|&idx| {
                        rebuild_row(table, idx, &self.action.union_find).map(|r| (idx, r))
                    })
                    .collect();

                if !pairs.is_empty() {
                    // Remove absorbed rows from reverse_index before unions change canonicals.
                    {
                        let arity = self.action.tables[tid].arity();
                        let value_type = &self.action.tables[tid].table_def.1[arity];
                        if matches!(value_type, Type::Name(_) | Type::Base(BaseType::Id)) {
                            for (scanned_idx, (_existing_idx, _old, new)) in &pairs {
                                let canonical = self.action.union_find.find(*new);
                                self.action.reverse_index.remove(canonical, tid, *scanned_idx);
                                // D1: Redirect cost_select from absorbed -> surviving enode
                                self.action.cost_tracker.cost_select_redirect(
                                    canonical,
                                    (tid, *scanned_idx),
                                    (tid, *_existing_idx),
                                );
                            }
                        }
                    }

                    // Track minimum affected row index so we only
                    // rescan from that point, not the whole table.
                    let min_idx = indices
                        .iter()
                        .map(|i| i.0)
                        .chain(pairs.iter().map(|(_, (i, _, _))| i.0))
                        .min()
                        .unwrap_or(0);
                    let entry = affected.entry(tid).or_insert(usize::MAX);
                    *entry = (*entry).min(min_idx);

                    let merge = self.action.tables[tid].table_def.2;
                    let new_pairs: Vec<_> = match merge {
                        Some(merge_fn) => {
                            for (_scanned_idx, (existing_idx, old, new)) in &pairs {
                                let column = self.action.tables[tid].column_count();
                                let arity = self.action.tables[tid].arity();
                                let value_ref =
                                    &mut self.action.tables[tid].rows[existing_idx.0 * column + arity];
                                let resolved = merge_fn.interp(new, old);
                                *value_ref = resolved;
                            }
                            Vec::new()
                        }
                        None => pairs
                            .into_iter()
                            .flat_map(|(_scanned_idx, (_existing_idx, old, new))| {
                                if let Some((parent, child)) =
                                    self.action.union_find.union(old, new)
                                {
                                    self.action.reverse_index.merge(parent, child);
                                    // D2: Merge eclass costs eagerly
                                    self.action.cost_tracker.merge_eclass_cost(parent, child);
                                    Some((parent, child))
                                } else {
                                    None
                                }
                            })
                            .collect(),
                    };
                    self.action.pending_unions.extend(new_pairs);
                }
            }
        }
        for (tid, min_idx) in &affected {
            let table = &mut self.action.tables[*tid];
            table.delta_start_row = table.delta_start_row.min(*min_idx);
        }
        affected.into_keys().collect()
    }

    pub fn register_native_fn(&mut self, func: NativeFn) -> usize {
        let offset = self.action.native_functions.len();
        self.action.native_functions.push(func);
        offset
    }

    pub fn find_defining_row(&self, id: Value) -> Option<(TableId, RowIndex)> {
        let id = self.action.union_find.find(id);
        for (tid, table) in self.action.tables.iter().enumerate() {
            let arity = table.arity();
            let col_count = table.column_count();
            for (i, chunk) in table.rows.chunks(col_count).enumerate() {
                if chunk[arity] == id {
                    return Some((tid, RowIndex(i)));
                }
            }
        }
        None
    }

    pub fn find(&self, id: Value) -> Value {
        self.action.union_find.find(id)
    }

    pub fn table_count(&self) -> usize {
        self.action.tables.len()
    }

    pub fn get_table(&self, tid: TableId) -> &Table {
        &self.action.tables[tid]
    }

    /// Allocate a fresh eclass ID registered with the union-find.
    pub fn fresh_id(&mut self) -> Value {
        self.action.alloc_id()
    }

    // canonicalize the eclass value
    // and then returns all the enodes of it
    pub fn eclass_enodes(&self, eclass: Value) -> Set<(TableId, RowIndex)> {
        let canonical = self.action.union_find.find(eclass);
        self.action.reverse_index.get(canonical)
    }

    /// Insert a cost model entry: "TypeName.ConsName" -> cost
    pub fn set_cost_model(&mut self, name: String, cost: u64) {
        self.action.cost_tracker.set_cost_model(name, cost);
    }

    /// Look up the cost of a constructor. Returns 0 if not defined.
    pub fn get_constructor_cost(&self, table_name: &str) -> u64 {
        self.action.cost_tracker.get_constructor_cost(table_name)
    }

    /// Get the current minimum cost of an eclass. Returns u64::MAX (⊥) if unknown.
    pub fn eclass_cost(&self, eclass: Value) -> u64 {
        self.action.cost_tracker.eclass_cost(&self.action.union_find, eclass)
    }

    /// Get the cheapest enode for an eclass, if any.
    pub fn cost_select(&self, eclass: Value) -> Option<(TableId, RowIndex)> {
        self.action.cost_tracker.cost_select(&self.action.union_find, eclass)
    }
}

fn rebuild_row(table: &Table, idx: RowIndex, uf: &UnionFind) -> Option<(RowIndex, Value, Value)> {
    let raw_key = table.get_row_key(idx);
    let canonical_key = table.canonicalize_row(uf, &raw_key.0);
    let existing = table.key_index.get(&canonical_key)?;
    if *existing != idx {
        let old = table.get_row_value(*existing);
        let new = table.get_canonicalized_row_value(uf, idx);
        Some((*existing, old, new))
    } else {
        None
    }
}
