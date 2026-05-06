/// related e-graph
use alloc::{boxed::Box, string::String, vec::Vec};
// use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use smallvec::{SmallVec, ToSmallVec};

use crate::{
    common::{ColumnIndex, Id, Map, RowIndex},
    table::{Row, Table},
    uf::UnionFind,
};

#[derive(Debug, Default, Clone)]
pub struct RelatedEGraph {
    union_find: UnionFind,

    tables: Vec<Table>,
    table_map: Map<String, usize>,

    next_id: Id,
    pending_unions: Vec<(Id, Id)>,
}

pub enum Subset {
    All,
    Since,
}

pub enum Constraint {
    Eq(ColumnIndex, Id),
}

impl RelatedEGraph {
    pub fn scan(&self, table_id: usize, subset: Subset, cs: &[Constraint]) -> Vec<Row> {
        let table = &self.tables[table_id];

        let start = match subset {
            Subset::All => 0,
            Subset::Since => table.last_scan_end,
        };

        table.rows[start..]
            .chunks(table.arity)
            .map(|row: &[Id]| {
                let chunk: SmallVec<_> = row.iter().map(|id| self.union_find.find(*id)).collect();
                Row(chunk)
            })
            .filter(|row| {
                cs.iter().all(|c| match c {
                    Constraint::Eq(col, id) => row.0[col.0] == *id,
                })
            })
            .collect::<Vec<_>>()
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
