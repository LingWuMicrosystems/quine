/// related e-graph
use alloc::{boxed::Box, string::String, vec::Vec};
// use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use smallvec::{SmallVec, ToSmallVec, smallvec};

use crate::{
    common::{ColumnIndex, Id, Map, RowIndex, Set},
    rule::{Constraint, FusedScan, Op, Rule, VarColsScanRule},
    table::{Row, Table},
    uf::UnionFind,
};

pub type TableId = usize;

#[derive(Debug, Default, Clone)]
pub struct RelatedEGraph {
    union_find: UnionFind,

    tables: Vec<Table>,
    table_map: Map<String, TableId>,

    next_id: Id,
    pending_unions: Vec<(Id, Id)>,

    rules: Vec<Rule>,
}

pub enum Subset {
    All,
    Since,
}

impl RelatedEGraph {
    pub fn run(&mut self) -> bool {
        loop {
            for rule in &self.rules {
                self.run_rule(rule);
            }
            self.rebuild();
        }
    }

    pub fn run_rule(&self, rule: &Rule) {
        let matches = self.run_query(&rule.var_cols, smallvec![]);
        todo!()
    }

    pub fn run_query(
        &self,
        var_cols: &[VarColsScanRule],
        binding: SmallVec<[(ColumnIndex, Constraint); 4]>,
    ) -> Set<Row> {
        let mut cartesian_product: Set<Row> = Set::default();
        loop {
            if var_cols.is_empty() {
                return cartesian_product;
            }
            let cols = &var_cols[0];
            if cartesian_product.is_empty() {
                let set = cols
                    .iter()
                    .map(|fusion| self.fused_scan(fusion, binding.clone()))
                    .reduce(|l, r| l.intersection(&r).copied().collect())
                    .unwrap_or_default();
                cartesian_product = set.iter().map(|id| Row(smallvec![*id])).collect();
                continue;
            }
            let r = cartesian_product
                .iter()
                .map(|row| {
                    let set = cols
                        .iter()
                        .map(|fused_scan| {
                            let mut full_binding = binding.clone();
                            full_binding.extend(row_bindings(var_cols, fused_scan.table, &row));
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
                .flatten()
                .collect();
            cartesian_product = r;
        }
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
