/// related e-graph
use alloc::vec::Vec;
// use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use smallvec::{ToSmallVec, smallvec};

use crate::regraph::{
    common::{Map, RowIndex, Set, Value},
    rule::{Action, ActionTail, FunctionCall, FusedScan, Op, Query, Rule},
    table::{Column, Row, Table},
    types::{BaseType, TableDef, Type},
    uf::UnionFind,
};

pub type TableId = usize;
pub type RuleId = usize;

#[derive(Debug, Default, Clone)]
pub struct RelatedEGraph {
    union_find: UnionFind,
    tables: Vec<Table>,

    pending_unions: Vec<(Value, Value)>,
    dirty_tables: Vec<TableId>,

    ruleset: Vec<Rule>,
    rule_deps: Map<TableId, Vec<RuleId>>,
}

impl RelatedEGraph {
    pub fn add_table(&mut self, table_def: TableDef) {
        self.tables.push(Table::new(table_def));
    }

    pub fn add_rule(&mut self, rule: Rule) {
        let rule_id = self.ruleset.len();
        for t in rule.query.tables().iter() {
            self.rule_deps.entry(*t).or_default().push(rule_id);
        }
        self.ruleset.push(rule);
    }

    pub fn set_dirty(&mut self, table_id: TableId) {
        if !self.dirty_tables.contains(&table_id) {
            self.dirty_tables.push(table_id);
        }
    }

    pub fn set_fully_dirty(&mut self) {
        self.dirty_tables = (0..self.tables.len()).collect();
    }

    pub fn run(&mut self) {
        loop {
            let app_rules: Vec<_> = self
                .dirty_tables
                .iter()
                .flat_map(|table| self.rule_deps.get(table).cloned())
                .flatten()
                .collect::<Set<RuleId>>()
                .into_iter()
                .collect();

            if app_rules.is_empty() {
                return;
            }
            self.dirty_tables.clear();

            // batched query
            let rules_rows: Vec<Set<Row>> = app_rules
                .iter()
                .map(|rule_id| {
                    let query = &self.ruleset[*rule_id].query;
                    self.run_query(query)
                })
                .collect();

            // apply actions
            for (rule_id, rows) in app_rules.iter().zip(rules_rows.into_iter()) {
                let action = &self.ruleset[*rule_id].action.clone();
                self.apply_action(action, rows);
            }

            self.rebuild();
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
            unimplemented!();
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
                    let candidates: Set<Value> = cols
                        .iter()
                        .map(|fs| self.fused_scan(fs).into_iter().collect::<Set<_>>())
                        .reduce(|l, r| l.intersection(&r).copied().collect())
                        .unwrap_or_default();
                    candidates
                        .into_iter()
                        .map(|id| {
                            let mut r = row.clone();
                            r.0.push(id);
                            r
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
                        let ty = query.variables.get_type(cs.lhs.0).unwrap();
                        check_cross(*lhs, *rhs, cs.op, ty)
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

    pub fn insert(&mut self, table_id: usize, mut key: Row, value: Value) {
        let table = &mut self.tables[table_id];

        // canonical value
        // let value = self.union_find.find_compress(value);

        debug_assert_eq!(key.0.len(), table.arity());
        if let Some(row_idx) = table.key_index.get(&key) {
            if let Some(r) = self.union_find.union(table.get_result(*row_idx), value) {
                self.pending_unions.push(r);
                self.set_dirty(table_id);
            }
            return;
        }

        let row_idx = RowIndex(table.row_count());

        // insert forward find table
        table.key_index.insert(key.clone(), row_idx);
        // insert backward find table — only Id columns can be unioned
        for (i, v) in key.0.iter().enumerate() {
            if matches!(table.columns[i], Column::Id(_)) {
                table.parents.entry(*v).or_default().push(row_idx);
            }
        }
        if matches!(table.columns[table.arity()], Column::Id(_)) {
            table.parents.entry(value).or_default().push(row_idx);
        }

        key.0.push(value);
        // insert row & result
        table.insert_row(key);
        self.set_dirty(table_id);
    }

    pub fn union(&mut self, old: Value, new: Value) {
        let (old, new) = self.union_find.union(old, new).unwrap();
        self.pending_unions.push((old, new));
    }

    pub fn rebuild(&mut self) {
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
                    self.set_dirty(tid);
                    let new_pairs: Vec<_> = pairs
                        .into_iter()
                        .flat_map(|(old, new)| self.union_find.union(old, new))
                        .collect();
                    self.pending_unions.extend(new_pairs);
                }
            }
        }
    }

    pub fn alloc_id(&mut self) -> Value {
        let id = Value(self.union_find.parents.len() as u64);
        self.union_find.add(id);
        id
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

    pub fn find_defining_row(&self, id: Value) -> Option<(TableId, RowIndex)> {
        let id = self.union_find.find(id);
        for (tid, table) in self.tables.iter().enumerate() {
            if let Column::Id(vals) = &table.columns[table.arity()] {
                for (i, &v) in vals.iter().enumerate() {
                    if self.union_find.find(v) == id {
                        return Some((tid, RowIndex(i)));
                    }
                }
            }
        }
        None
    }
}

fn rebuild_row(table: &Table, idx: RowIndex, uf: &UnionFind) -> Option<(Value, Value)> {
    let row = table.get_all_row(idx);
    let canonical: Vec<Value> = row
        .0
        .iter()
        .enumerate()
        .map(|(i, v)| match &table.columns[i] {
            Column::Id(_) => uf.find(*v),
            _ => *v,
        })
        .collect();

    let key = Row(canonical[..table.arity()].to_smallvec());
    let existing = table.key_index.get(&key)?;
    if *existing == idx {
        return None;
    }
    let old = table.get_result(*existing);
    let new = canonical[table.arity()];
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
