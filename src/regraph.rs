/// related e-graph
use alloc::{boxed::Box, vec::Vec};
// use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use smallvec::{SmallVec, ToSmallVec, smallvec};

use crate::{
    common::{ColumnIndex, Id, RowIndex, Set},
    rule::{Action, Constraint, CrossConstraint, FusedScan, Op, Rule, VarColsScanRule},
    table::{Row, Table},
    uf::UnionFind,
};

pub type TableId = usize;

#[derive(Debug, Default, Clone)]
pub struct RelatedEGraph {
    union_find: UnionFind,

    tables: Vec<Table>,

    next_id: Id,
    pending_unions: Vec<(Id, Id)>,
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
        let rows = self.run_query(&rule.var_cols, &rule.constraints, smallvec![]);
        if rows.is_empty() {
            return false;
        }
        self.apply_actions(&rule.actions, &rows);
        true
    }

    pub fn apply_actions(&mut self, actions: &[Action], rows: &Set<Row>) {
        for action in actions {
            match action {
                Action::Union(lhs, rhs) => {
                    for row in rows.iter() {
                        let lhs = lhs.resolve(row);
                        let rhs = rhs.resolve(row);
                        self.pending_unions.push((lhs, rhs));
                    }
                }
                Action::Insert(table_id, action_row) => {
                    for row in rows.iter() {
                        let row = action_row.iter().map(|a| a.resolve(row)).collect();
                        let new_id = self.alloc_id();
                        self.insert(*table_id, Row(row), new_id);
                    }
                } // Action::Delete(table_id, _) => self.delete(*table_id),
            }
        }
    }

    pub fn run_query(
        &self,
        var_cols: &[VarColsScanRule],
        cross_rules: &[CrossConstraint],
        binding: SmallVec<[(ColumnIndex, Constraint); 4]>,
    ) -> Set<Row> {
        let Some(cols) = var_cols.first() else {
            return Set::default();
        };

        let mut rows: Set<Row> = cols
            .iter()
            .map(|fusion| self.fused_scan(fusion, binding.clone()))
            .reduce(|l, r| l.intersection(&r).copied().collect())
            .unwrap_or_default()
            .into_iter()
            .map(|id| Row(smallvec![id]))
            .collect();

        let rest = &var_cols[1..];

        for cols in rest {
            rows = rows
                .iter()
                .flat_map(|row| {
                    let set = cols
                        .iter()
                        .map(|fused_scan| {
                            let mut full_binding = binding.clone();
                            full_binding.extend(row_bindings(var_cols, fused_scan.table, row));
                            self.fused_scan(fused_scan, full_binding)
                        })
                        .reduce(|l, r| l.intersection(&r).copied().collect())
                        .unwrap_or_default();
                    set.iter()
                        .map(|id| {
                            let mut row = row.clone();
                            row.0.push(*id);
                            row
                        })
                        .collect::<Set<Row>>()
                })
                .filter(|row| {
                    cross_rules.iter().all(|cs| {
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

    fn fused_scan(
        &self,
        fused_scan: &FusedScan,
        mut binding: SmallVec<[(ColumnIndex, Constraint); 4]>,
    ) -> Set<Id> {
        let table = &self.tables[fused_scan.table];
        if let Some(cs) = fused_scan.constraints {
            binding.push((fused_scan.column, cs));
        }
        table.fused_scan(&self.union_find, fused_scan.column, &binding)
    }

    pub fn insert(&mut self, table_id: usize, key: Row, value: Id) {
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

    pub fn alloc_id(&mut self) -> Id {
        let id = self.next_id;
        self.next_id = Id(id.0 + 1);
        id
    }
}

fn row_bindings(
    var_cols: &[VarColsScanRule],
    table: TableId,
    row: &Row,
) -> SmallVec<[(ColumnIndex, Constraint); 4]> {
    row.0
        .iter()
        .enumerate()
        .filter_map(|(i, val)| {
            var_cols[i].iter().find(|s| s.table == table).map(|s| {
                (
                    s.column,
                    Constraint {
                        op: Op::Equ,
                        id: *val,
                    },
                )
            })
        })
        .collect()
}

fn rebuild_table(table: &Table, child: Id, uf: &UnionFind) -> Vec<(Id, Id)> {
    let Some(indexs) = table.parents.get(&child) else {
        return Vec::new();
    };
    // all row
    indexs
        .iter()
        .flat_map(|idx| rebuild_row(table, *idx, uf))
        .collect()
}

fn rebuild_row(table: &Table, idx: RowIndex, uf: &UnionFind) -> Option<(Id, Id)> {
    let row = table.get_all_row(idx);
    // canonicalize
    let canonical: Box<[Id]> = row.iter().map(|v| uf.find(*v)).collect();
    let key = Row(canonical[..table.arity - 1].to_smallvec());

    let existing = table.key_index.get(&key)?;
    if *existing == idx {
        return None;
    }

    let old = table.get_result(*existing);
    let new = *canonical.last().unwrap();
    Some((old, new))
}
