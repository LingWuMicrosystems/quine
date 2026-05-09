/// related e-graph
use alloc::{boxed::Box, vec::Vec};
// use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use smallvec::{ToSmallVec, smallvec};

use crate::{
    common::{RowIndex, Set, Value},
    core::{
        rule::{Action, ActionTail, FunctionCall, FusedScan, Op, Query, Rule},
        table::{Row, Table},
    },
    uf::UnionFind,
};

pub type TableId = usize;

#[derive(Debug, Default, Clone)]
pub struct RelatedEGraph {
    union_find: UnionFind,

    tables: Vec<Table>,

    next_id: Value,
    pending_unions: Vec<(Value, Value)>,
}

pub enum Subset {
    All,
    Since,
}

impl RelatedEGraph {
    pub fn run(&mut self, rules: &[Rule]) {
        // TODO: need scheduler
        let mut dirty = false;
        loop {
            for rule in rules {
                let result = self.run_rule(rule);
                dirty |= result;
            }
            if !dirty {
                return;
            }
            self.rebuild();
        }
    }

    pub fn run_rule(&mut self, rule: &Rule) -> bool {
        let rows = self.run_query(&rule.query);
        if rows.is_empty() {
            return false;
        }
        self.apply_action(&rule.action, rows);
        true
    }

    pub fn apply_action(&mut self, actions: &Action, rows: Set<Row>) {
        for row in rows.into_iter() {
            self.apply_action_in_row(actions, row);
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
            unimplemented!();
        }
        let result = self.alloc_id();
        self.insert(function_call.offset, args, result);
        result
    }

    fn apply_action_tail(&mut self, tail: &ActionTail, row: &Row) {
        match tail {
            ActionTail::Union(var0, var1) => self.union(var0.resolve(&row), var1.resolve(&row)),
            ActionTail::Insert(table_id, args, result) => {
                let args = Row(args.iter().map(|arg| arg.resolve(&row)).collect());
                if let Some(result) = result {
                    self.insert(*table_id, args, result.resolve(row));
                } else {
                    let id = self.alloc_id();
                    self.insert(*table_id, args, id);
                }
            }
        }
    }

    pub fn run_query(&self, query: &Query) -> Set<Row> {
        let Some(cols) = query.var_cols.first() else {
            return Set::default();
        };

        let mut rows: Set<Row> = cols
            .iter()
            .map(|fusion| self.fused_scan(fusion))
            .reduce(|l, r| l.intersection(&r).copied().collect())
            .unwrap_or_default()
            .into_iter()
            .map(|id| Row(smallvec![id]))
            .collect();

        let rest = &query.var_cols[1..];

        for cols in rest {
            rows = rows
                .iter()
                .flat_map(|row| {
                    cols.iter()
                        .map(|fused_scan| self.fused_scan(fused_scan))
                        .reduce(|l, r| l.intersection(&r).copied().collect())
                        .unwrap_or_default()
                        .iter()
                        .map(|id| {
                            let mut row = row.clone();
                            row.0.push(*id);
                            row
                        })
                        .collect::<Set<Row>>()
                })
                .filter(|row| {
                    query.constraints.iter().all(|cs| {
                        let Some(lhs) = row.0.get(cs.lhs.0) else {
                            return true;
                        };
                        let Some(rhs) = row.0.get(cs.rhs.0) else {
                            return true;
                        };
                        match cs.op {
                            Op::Equ => lhs == rhs,
                            Op::Neq => lhs != rhs,
                            Op::Lt => (lhs.0 as i32) < (rhs.0 as i32),
                            Op::Gt => (lhs.0 as i32) > (rhs.0 as i32),
                            Op::Leq => (lhs.0 as i32) <= (rhs.0 as i32),
                            Op::Geq => (lhs.0 as i32) >= (rhs.0 as i32),
                            Op::Ltu => lhs < rhs,
                            Op::Gtu => lhs > rhs,
                            Op::Lequ => lhs <= rhs,
                            Op::Gequ => lhs >= rhs,
                        }
                    })
                })
                .collect();
        }
        rows
    }

    fn fused_scan(&self, fused_scan: &FusedScan) -> Set<Value> {
        let table = &self.tables[fused_scan.table];
        table.fused_scan(&self.union_find, fused_scan.column, &fused_scan.constraints)
    }

    pub fn insert(&mut self, table_id: usize, key: Row, value: Value) {
        let table = &mut self.tables[table_id];

        // canonical key
        let mut key = Row(key
            .0
            .into_iter()
            .map(|v| self.union_find.find_compress(v))
            .collect());
        let value = self.union_find.find_compress(value);

        debug_assert_eq!(key.0.len(), table.arity - 1);
        if let Some(row_idx) = table.key_index.get(&key) {
            if let Some(r) = self.union_find.union(table.get_result(*row_idx), value) {
                self.pending_unions.push(r);
            }
            return;
        }

        let row_idx = RowIndex(table.rows.len() / table.arity);

        // insert forward find table
        table.key_index.insert(key.clone(), row_idx);
        // insert backward find table
        for v in &key.0 {
            table.parents.entry(*v).or_default().push(row_idx);
        }
        table.parents.entry(value).or_default().push(row_idx);

        key.0.push(value);
        // insert row & result
        table.rows.extend(key.0.iter());
    }

    pub fn union(&mut self, old: Value, new: Value) {
        let (old, new) = self.union_find.union(old, new).unwrap();
        self.pending_unions.push((old, new));
    }

    pub fn rebuild(&mut self) {
        while let Some((_parent, child)) = self.pending_unions.pop() {
            // all table
            let ununioned: Vec<_> = self
                .tables
                .iter()
                .flat_map(|table| rebuild_table(table, child, &self.union_find))
                .collect();
            let pending = ununioned
                .into_iter()
                .flat_map(|(old, new)| self.union_find.union(old, new))
                .collect::<Vec<_>>();
            self.pending_unions.extend(pending);
        }
    }

    pub fn alloc_id(&mut self) -> Value {
        let id = self.next_id;
        self.next_id = Value(id.0 + 1);
        id
    }
}

fn rebuild_table(table: &Table, child: Value, uf: &UnionFind) -> Vec<(Value, Value)> {
    let Some(indexs) = table.parents.get(&child) else {
        return Vec::new();
    };
    // all row
    indexs
        .iter()
        .flat_map(|idx| rebuild_row(table, *idx, uf))
        .collect()
}

fn rebuild_row(table: &Table, idx: RowIndex, uf: &UnionFind) -> Option<(Value, Value)> {
    let row = table.get_all_row(idx);
    // canonicalize
    let canonical: Box<[Value]> = row.iter().map(|v| uf.find(*v)).collect();
    let key = Row(canonical[..table.arity - 1].to_smallvec());

    let existing = table.key_index.get(&key)?;
    if *existing == idx {
        return None;
    }

    let old = table.get_result(*existing);
    let new = *canonical.last().unwrap();
    Some((old, new))
}
