/// related e-graph
use alloc::string::String;
use alloc::vec::Vec;
use smallvec::ToSmallVec;

#[cfg(feature = "std")]
use rayon::prelude::*;

use crate::{
    common::{ColumnIndex, Map, RowIndex, Set, Value, VarId},
    rule::{Action, ActionTail, FunctionCall, Op, Query, Rule},
    table::{Row, Table},
    types::{BaseType, TableDef, Type},
    uf::UnionFind,
};

pub type TableId = usize;
pub type RuleId = usize;
pub type GroupName = String;

pub type NativeFn = fn(input: &[Value]) -> Value;

pub type RuleGroup = Set<RuleId>;

#[derive(Debug, Clone)]
pub enum RunMode {
    Saturate,
    Times(usize),
}

#[derive(Debug, Default, Clone)]
pub struct RelatedEGraph {
    union_find: UnionFind,
    tables: Vec<Table>,

    pending_unions: Vec<(Value, Value)>,

    native_functions: Vec<NativeFn>,

    ruleset: Vec<Rule>,
    rule_deps: Map<TableId, Vec<RuleId>>,
    rule_groups: Map<GroupName, RuleGroup>,
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

    /// Run all rules to fixpoint (backward compat).
    pub fn run(&mut self) {
        self.run_all(RunMode::Saturate);
    }

    /// Run all rules with the given mode.
    pub fn run_all(&mut self, mode: RunMode) {
        self.run_semi_naive(None, mode);
    }

    /// Run rules in the named group with the given mode.
    pub fn run_group(&mut self, group_name: &str, mode: RunMode) {
        if let Some(rules) = self.rule_groups.get(group_name).cloned() {
            self.run_semi_naive(Some(&rules), mode);
        }
    }

    fn run_semi_naive(&mut self, rule_filter: Option<&RuleGroup>, mode: RunMode) {
        let mut iteration = 0;
        loop {
            // Collect (driver_table, rule) pairs, optionally filtered by group
            let pairs: Vec<(TableId, RuleId)> = (0..self.tables.len())
                .filter(|tid| self.tables[*tid].has_delta())
                .flat_map(|tid| {
                    self.rule_deps
                        .get(&tid)
                        .into_iter()
                        .flatten()
                        .filter(|rid| rule_filter.map_or(true, |r| r.contains(rid)))
                        .map(move |rid| (tid, *rid))
                })
                .collect::<Set<(TableId, RuleId)>>()
                .into_iter()
                .collect();

            if pairs.is_empty() {
                return;
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
                let action = &self.ruleset[*rule_id].action.clone();
                self.apply_action(action, rows);
            }

            let rebuild_affected = self.rebuild();

            // New delta = rows added since snapshot,
            // unless rebuild already reset it to 0 for a full re-scan.
            for tid in 0..self.tables.len() {
                if !rebuild_affected.contains(&tid) {
                    self.tables[tid].delta_start_row = snapshots[tid];
                }
            }

            iteration += 1;
            if matches!(mode, RunMode::Times(n) if iteration >= n) {
                return;
            }
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
        result
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

    pub fn run_query(&self, query: &Query, delta_table: Option<TableId>) -> Set<Row> {
        let step_results: Vec<Vec<Row>> = query
            .scan_steps
            .iter()
            .map(|step| {
                let table = &self.tables[step.table];
                let col_indices: Vec<ColumnIndex> = step.columns.iter().map(|(c, _)| *c).collect();
                let use_delta = delta_table == Some(step.table) && table.has_delta();
                table
                    .fused_scan(&self.union_find, &col_indices, &step.constraints, use_delta)
                    .collect()
            })
            .collect();

        if step_results.is_empty() {
            return Set::default();
        }

        let mut rows: Vec<Row> = step_results[0].clone();
        let mut result_vars: Vec<VarId> = query.scan_steps[0]
            .columns
            .iter()
            .map(|(_, v)| *v)
            .collect();

        for (step_idx, next_rows) in step_results.iter().enumerate().skip(1) {
            let step = &query.scan_steps[step_idx];

            let shared: Vec<(usize, usize)> = step
                .columns
                .iter()
                .enumerate()
                .filter_map(|(sp, (_, v))| {
                    result_vars.iter().position(|rv| rv == v).map(|rp| (sp, rp))
                })
                .collect();

            let new_cols: Vec<usize> = step
                .columns
                .iter()
                .enumerate()
                .filter(|(_, (_, v))| !result_vars.contains(v))
                .map(|(i, _)| i)
                .collect();

            rows = rows
                .into_iter()
                .flat_map(|left| {
                    next_rows
                        .iter()
                        .filter(|right| shared.iter().all(|(sp, rp)| right.0[*sp] == left.0[*rp]))
                        .map(|right| {
                            let mut r = left.clone();
                            for &si in &new_cols {
                                r.0.push(right.0[si]);
                            }
                            r
                        })
                        .collect::<Vec<_>>()
                })
                .collect();

            for &si in &new_cols {
                result_vars.push(step.columns[si].1);
            }
        }

        rows.into_iter()
            .filter(|row| {
                query.constraints.iter().all(|cs| {
                    let Some(lhs) = row.0.get(cs.lhs.0) else {
                        return true;
                    };
                    let Some(rhs) = row.0.get(cs.rhs.0) else {
                        return true;
                    };
                    let ty = query.variables.get_type(cs.lhs.0).unwrap();
                    check_cross(*lhs, *rhs, cs.op, ty)
                })
            })
            .collect()
    }

    pub fn insert(&mut self, table_id: usize, key: Row, value: Value) {
        let table = &mut self.tables[table_id];
        debug_assert_eq!(key.0.len(), table.arity());
        table.insert(&mut self.union_find, key, value);
    }

    pub fn union(&mut self, old: Value, new: Value) {
        if let Some(r) = self.union_find.union(old, new) {
            self.pending_unions.push(r);
        }
    }

    pub fn rebuild(&mut self) -> Set<TableId> {
        let mut affected = Set::default();
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
                    // Rebuild changed equivalence classes for this table.
                    // Reset delta to force a full re-scan next iteration.
                    self.tables[tid].delta_start_row = 0;
                    affected.insert(tid);
                    let new_pairs: Vec<_> = pairs
                        .into_iter()
                        .flat_map(|(old, new)| self.union_find.union(old, new))
                        .collect();
                    self.pending_unions.extend(new_pairs);
                }
            }
        }
        affected
    }

    pub fn register_native_fn(&mut self, func: NativeFn) -> usize {
        let offset = self.native_functions.len();
        self.native_functions.push(func);
        offset
    }

    pub fn alloc_id(&mut self) -> Value {
        let id = Value(self.union_find.parents.len() as u64);
        self.union_find.add(id);
        id
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

fn rebuild_row(table: &Table, idx: RowIndex, uf: &UnionFind) -> Option<(Value, Value)> {
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
    Some((old, new))
}

fn check_cross(lhs: Value, rhs: Value, op: Op, ty: &Type) -> bool {
    let base = match ty {
        Type::Base(b) => b,
        Type::Name(_) => {
            return match op {
                Op::Equ => lhs == rhs,
                Op::Neq => lhs != rhs,
                _ => false,
            };
        }
    };
    match base {
        BaseType::Id | BaseType::Str => match op {
            Op::Equ => lhs == rhs,
            Op::Neq => lhs != rhs,
            _ => false,
        },
        BaseType::I1 => {
            let l = lhs.0 != 0;
            let r = rhs.0 != 0;
            cmp_cross_inner(l, r, op)
        }
        BaseType::I8 => cmp_cross_inner(lhs.0 as i8, rhs.0 as i8, op),
        BaseType::U8 => cmp_cross_inner(lhs.0 as u8, rhs.0 as u8, op),
        BaseType::I16 => cmp_cross_inner(lhs.0 as i16, rhs.0 as i16, op),
        BaseType::U16 => cmp_cross_inner(lhs.0 as u16, rhs.0 as u16, op),
        BaseType::I32 => cmp_cross_inner(lhs.0 as i32, rhs.0 as i32, op),
        BaseType::U32 => cmp_cross_inner(lhs.0 as u32, rhs.0 as u32, op),
        BaseType::I64 => cmp_cross_inner(lhs.0 as i64, rhs.0 as i64, op),
        BaseType::U64 => cmp_cross_inner(lhs.0, rhs.0, op),
        BaseType::F32 => {
            let l = f32::from_bits(lhs.0 as u32);
            let r = f32::from_bits(rhs.0 as u32);
            cmp_cross_inner(l, r, op)
        }
        BaseType::F64 => {
            let l = f64::from_bits(lhs.0);
            let r = f64::from_bits(rhs.0);
            cmp_cross_inner(l, r, op)
        }
    }
}

fn cmp_cross_inner<T: PartialEq + PartialOrd>(l: T, r: T, op: Op) -> bool {
    match op {
        Op::Equ => l == r,
        Op::Neq => l != r,
        Op::Lt => l < r,
        Op::Gt => l > r,
        Op::Leq => l <= r,
        Op::Geq => l >= r,
    }
}
