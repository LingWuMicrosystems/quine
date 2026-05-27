/// related e-graph
use alloc::string::String;
use alloc::vec::Vec;
use smallvec::{SmallVec, ToSmallVec};

#[cfg(feature = "std")]
use rayon::prelude::*;

use crate::{
    common::{ColumnIndex, Map, RowIndex, Set, Value, VarId},
    rule::{Action, ActionTail, Constraint, FunctionCall, Op, Query, Rule, ScanStep},
    table::{ModifyState, Row, Table},
    types::{BaseType, MergeFn, TableDef, Type},
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
    union_find: UnionFind,
    tables: Vec<Table>,

    pending_unions: Vec<(Value, Value)>,

    native_functions: Vec<NativeFn>,

    ruleset: Vec<Rule>,
    rule_deps: Map<TableId, Vec<RuleId>>,
    pub rule_groups: Map<GroupName, RuleGroup>,
}

/// Mutable state needed exclusively by action application.
/// Separated so that action can borrow ruleset independently.
struct ActionCtx<'a> {
    tables: &'a mut Vec<Table>,
    union_find: &'a mut UnionFind,
    pending_unions: &'a mut Vec<(Value, Value)>,
    native_functions: &'a [NativeFn],
}

impl ActionCtx<'_> {
    fn alloc_id(&mut self) -> Value {
        let id = Value(self.union_find.parents.len() as u64);
        self.union_find.add(id);
        id
    }

    fn insert(&mut self, table_id: usize, key: Row, value: Value) {
        let table = &mut self.tables[table_id];
        let arity = table.arity();
        debug_assert_eq!(key.0.len(), arity);

        // Snapshot pre-insert canonicals so we can detect unions.
        let col = table.column_count();
        let existing_idx = table.key_index.get(&key).copied();
        let old_canonical = existing_idx
            .map(|idx| self.union_find.find(table.rows[idx.0 * col + arity]));
        let new_canonical = self.union_find.find(value);

        match table.insert(&mut self.union_find, key, value) {
            ModifyState::NewRow(idx) => {
                let start = idx.0 * col;
                for i in 0..col {
                    let ty = &table.table_def.1[i];
                    if matches!(ty, Type::Name(_) | Type::Base(BaseType::Id)) {
                        let v = table.rows[start + i];
                        table.parents.entry(v).or_default().push(idx);
                    }
                }
            }
            ModifyState::UnionRow(_) => {
                if let Some(old) = old_canonical {
                    if old != new_canonical {
                        let pair = if old < new_canonical {
                            (old, new_canonical)
                        } else {
                            (new_canonical, old)
                        };
                        self.pending_unions.push(pair);
                    }
                }
            }
            ModifyState::MergeRow(_) | ModifyState::NoModify => {}
        }
    }

    fn union(&mut self, old: Value, new: Value) {
        if let Some(r) = self.union_find.union(old, new) {
            self.pending_unions.push(r);
        }
    }

    pub fn apply_action(&mut self, action: &Action, rows: Set<Row>) {
        for row in rows.into_iter() {
            self.apply_action_in_row(action, row);
        }
    }

    fn apply_action_in_row(&mut self, action: &Action, mut row: Row) {
        for call in &action.lets {
            let args = call.args.iter().map(|arg| arg.resolve(&row)).collect();
            let result = self.apply_function_call(call, Row(args));
            row.0.push(result);
        }
        for tail in &action.tail {
            self.apply_action_tail(tail, &row);
        }
    }

    fn apply_function_call(&mut self, function_call: &FunctionCall, args: Row) -> Value {
        if function_call.is_native {
            return self.native_functions[function_call.offset](&args.0);
        }
        let result = self.alloc_id();
        self.insert(function_call.offset, args, result);
        self.union_find.find(result)
    }

    fn apply_action_tail(&mut self, tail: &ActionTail, row: &Row) {
        match tail {
            ActionTail::Union(var0, var1) => self.union(var0.resolve(row), var1.resolve(row)),
            ActionTail::Insert(table_id, args, result) => {
                let args = Row(args.iter().map(|arg| arg.resolve(row)).collect());
                if let Some(result) = result {
                    self.insert(*table_id, args, result.resolve(row));
                } else {
                    let id = self.alloc_id();
                    self.insert(*table_id, args, id);
                }
            }
        }
    }
}

impl RelatedEGraph {
    pub fn add_table(&mut self, table_def: TableDef) {
        self.tables.push(Table::new(table_def));
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
            let pairs: Vec<(TableId, RuleId)> = (0..self.tables.len())
                .filter(|tid| self.tables[*tid].has_delta())
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
            let snapshots: Vec<usize> = self.tables.iter().map(|t| t.row_count).collect();

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
                let mut ctx = ActionCtx {
                    tables: &mut self.tables,
                    union_find: &mut self.union_find,
                    pending_unions: &mut self.pending_unions,
                    native_functions: &self.native_functions,
                };
                ctx.apply_action(action, rows);
            }

            let rebuild_affected = self.rebuild();

            // New delta = rows added since snapshot,
            // unless rebuild already reset it to 0 for a full re-scan.
            for (tid, (table, &snapshot)) in
                self.tables.iter_mut().zip(&snapshots).enumerate()
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
        let mut ctx = ActionCtx {
            tables: &mut self.tables,
            union_find: &mut self.union_find,
            pending_unions: &mut self.pending_unions,
            native_functions: &self.native_functions,
        };
        ctx.apply_action(action, rows);
    }

    pub fn run_query(&self, query: &Query, delta_table: Option<TableId>) -> Set<Row> {
        if query.scan_steps.is_empty() {
            return Set::default();
        }

        // Step 0: initial scan.
        let mut rows: Vec<Row> = scan_table(
            &self.tables,
            &self.union_find,
            &query.scan_steps[0],
            delta_table,
            &[],
        );

        let mut result_vars: Vec<VarId> = query.scan_steps[0]
            .columns
            .iter()
            .map(|(_, v)| *v)
            .collect();

        // Subsequent steps: sideways information passing + hash join.
        for step in &query.scan_steps[1..] {
            let shared: Vec<(usize, usize)> = step
                .columns
                .iter()
                .enumerate()
                .filter_map(|(sp, (_, v))| {
                    result_vars.iter().position(|rv| rv == v).map(|rp| (sp, rp))
                })
                .collect();

            let result_vars_set: Set<VarId> = result_vars.iter().copied().collect();
            let new_cols: Vec<usize> = step
                .columns
                .iter()
                .enumerate()
                .filter(|(_, (_, v))| !result_vars_set.contains(v))
                .map(|(i, _)| i)
                .collect();

            // Scan with pushed-down constraints when a single shared
            // variable allows sideways information passing.
            let next_rows: Vec<Row> = if shared.len() == 1 {
                let (sp, rp) = shared[0];
                let distinct: Set<Value> = rows.iter().map(|r| r.0[rp]).collect();
                let col = step.columns[sp].0;
                distinct
                    .iter()
                    .flat_map(|&v| {
                        scan_table(
                            &self.tables,
                            &self.union_find,
                            step,
                            delta_table,
                            &[Constraint {
                                op: Op::Equ,
                                column: col,
                                value: v,
                            }],
                        )
                    })
                    .collect()
            } else {
                scan_table(&self.tables, &self.union_find, step, delta_table, &[])
            };

            // Hash join on shared variables.
            if shared.is_empty() {
                let mut new_rows = Vec::with_capacity(rows.len() * next_rows.len());
                for left in &rows {
                    for right in &next_rows {
                        let mut r = left.clone();
                        for &si in &new_cols {
                            r.0.push(right.0[si]);
                        }
                        new_rows.push(r);
                    }
                }
                rows = new_rows;
            } else {
                let mut hash: Map<SmallVec<[Value; 4]>, Vec<&Row>> = Map::default();
                for right in &next_rows {
                    let key: SmallVec<[Value; 4]> =
                        shared.iter().map(|(sp, _)| right.0[*sp]).collect();
                    hash.entry(key).or_default().push(right);
                }
                let mut new_rows = Vec::with_capacity(rows.len());
                for left in &rows {
                    let key: SmallVec<[Value; 4]> =
                        shared.iter().map(|(_, rp)| left.0[*rp]).collect();
                    if let Some(matches) = hash.get(&key) {
                        for right in matches {
                            let mut r = left.clone();
                            for &si in &new_cols {
                                r.0.push(right.0[si]);
                            }
                            new_rows.push(r);
                        }
                    }
                }
                rows = new_rows;
            }

            for &si in &new_cols {
                result_vars.push(step.columns[si].1);
            }
        }

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
                query.constraints.iter().all(|cs| {
                    let Some(&lhs) = row.0.get(var_to_pos[cs.lhs.0]) else {
                        return true;
                    };
                    let Some(&rhs) = row.0.get(var_to_pos[cs.rhs.0]) else {
                        return true;
                    };
                    check_cross(lhs, rhs, cs.op)
                })
            })
            .map(|row| Row(var_to_pos.iter().map(|&p| row.0[p]).collect()))
            .collect()
    }

    pub fn insert(&mut self, table_id: usize, key: Row, value: Value) {
        let table = &mut self.tables[table_id];
        let arity = table.arity();
        debug_assert_eq!(key.0.len(), arity);

        let col = table.column_count();
        let existing_idx = table.key_index.get(&key).copied();
        let old_canonical = existing_idx
            .map(|idx| self.union_find.find(table.rows[idx.0 * col + arity]));
        let new_canonical = self.union_find.find(value);

        match table.insert(&mut self.union_find, key, value) {
            ModifyState::NewRow(idx) => {
                let start = idx.0 * col;
                for i in 0..col {
                    let ty = &table.table_def.1[i];
                    if matches!(ty, Type::Name(_) | Type::Base(BaseType::Id)) {
                        let v = table.rows[start + i];
                        table.parents.entry(v).or_default().push(idx);
                    }
                }
            }
            ModifyState::UnionRow(_) => {
                if let Some(old) = old_canonical {
                    if old != new_canonical {
                        let pair = if old < new_canonical {
                            (old, new_canonical)
                        } else {
                            (new_canonical, old)
                        };
                        self.pending_unions.push(pair);
                    }
                }
            }
            ModifyState::MergeRow(_) | ModifyState::NoModify => {}
        }
    }

    pub fn union(&mut self, old: Value, new: Value) {
        if let Some(r) = self.union_find.union(old, new) {
            self.pending_unions.push(r);
        }
    }

    pub fn rebuild(&mut self) -> Set<TableId> {
        let mut affected: Map<TableId, usize> = Map::default();
        while let Some((_parent, child)) = self.pending_unions.pop() {
            for tid in 0..self.tables.len() {
                let table = &self.tables[tid];
                let Some(indices) = table.parents.get(&child) else {
                    continue;
                };

                let pairs: Vec<_> = indices
                    .iter()
                    .flat_map(|&idx| rebuild_row(table, idx, &self.union_find))
                    .collect();

                if !pairs.is_empty() {
                    // Track minimum affected row index so we only
                    // rescan from that point, not the whole table.
                    let min_idx = indices
                        .iter()
                        .map(|i| i.0)
                        .chain(pairs.iter().map(|(i, _, _)| i.0))
                        .min()
                        .unwrap_or(0);
                    let entry = affected.entry(tid).or_insert(usize::MAX);
                    *entry = (*entry).min(min_idx);

                    let merge = self.tables[tid].table_def.2;
                    let new_pairs: Vec<_> = match merge {
                        Some(merge_fn) => {
                            for (existing_idx, old, new) in &pairs {
                                let column = self.tables[tid].column_count();
                                let arity = self.tables[tid].arity();
                                let value_ref =
                                    &mut self.tables[tid].rows[existing_idx.0 * column + arity];
                                let resolved = match merge_fn {
                                    MergeFn::Min => {
                                        if *new < *old {
                                            *new
                                        } else {
                                            *old
                                        }
                                    }
                                    MergeFn::Max => {
                                        if *new > *old {
                                            *new
                                        } else {
                                            *old
                                        }
                                    }
                                };
                                *value_ref = resolved;
                            }
                            Vec::new()
                        }
                        None => pairs
                            .into_iter()
                            .flat_map(|(_, old, new)| self.union_find.union(old, new))
                            .collect(),
                    };
                    self.pending_unions.extend(new_pairs);
                }
            }
        }
        for (tid, min_idx) in &affected {
            let table = &mut self.tables[*tid];
            table.delta_start_row = table.delta_start_row.min(*min_idx);
        }
        affected.into_keys().collect()
    }

    pub fn register_native_fn(&mut self, func: NativeFn) -> usize {
        let offset = self.native_functions.len();
        self.native_functions.push(func);
        offset
    }

    pub fn find_defining_row(&self, id: Value) -> Option<(TableId, RowIndex)> {
        let id = self.union_find.find(id);
        for (tid, table) in self.tables.iter().enumerate() {
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
        self.union_find.find(id)
    }

    pub fn table_count(&self) -> usize {
        self.tables.len()
    }

    pub fn get_table(&self, tid: TableId) -> &Table {
        &self.tables[tid]
    }
}

fn scan_table(
    tables: &[Table],
    uf: &UnionFind,
    step: &ScanStep,
    delta_table: Option<TableId>,
    extra_constraints: &[Constraint],
) -> Vec<Row> {
    let table = &tables[step.table];
    let col_indices: Vec<ColumnIndex> = step.columns.iter().map(|(c, _)| *c).collect();
    let use_delta = delta_table == Some(step.table) && table.has_delta();
    let mut constraints = step.constraints.clone();
    constraints.extend_from_slice(extra_constraints);
    table
        .fused_scan(uf, &col_indices, &constraints, use_delta)
        .collect()
}

fn rebuild_row(table: &Table, idx: RowIndex, uf: &UnionFind) -> Option<(RowIndex, Value, Value)> {
    let row = table.get_all_row(idx);
    let canonical: Vec<Value> = row
        .0
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let ty = &table.table_def.1[i];
            if matches!(ty, Type::Name(_) | Type::Base(BaseType::Id)) {
                uf.find(*v)
            } else {
                *v
            }
        })
        .collect();

    let arity = table.arity();
    let key = Row(canonical[..arity].to_smallvec());
    let existing = table.key_index.get(&key)?;
    if *existing == idx {
        return None;
    }
    let old = table.get_all_row(*existing).0[arity];
    let new = canonical[arity];
    Some((*existing, old, new))
}

#[inline]
fn check_cross(lhs: Value, rhs: Value, op: Op) -> bool {
    match op {
        Op::Equ => lhs.0 == rhs.0,
        Op::Neq => lhs.0 != rhs.0,
        Op::Lt => lhs.0 < rhs.0,
        Op::Gt => lhs.0 > rhs.0,
        Op::Leq => lhs.0 <= rhs.0,
        Op::Geq => lhs.0 >= rhs.0,
    }
}
