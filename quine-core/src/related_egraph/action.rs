
use super::cost::CostTracker;
use super::reverse_index::ReverseIndex;
use super::NativeFn;
use crate::uf::UnionFind;
use crate::common::{Value,Set};
use crate::table::{Table,ModifyState,Row};
use crate::rule::{Action,ActionTail, FunctionCall};
use crate::types::{Type,BaseType};
use alloc::{vec::Vec};


/// Mutable state needed exclusively by action application.
/// Separated so that action can borrow ruleset independently.
#[derive(Debug, Default, Clone)]
pub(crate) struct ActionCtx{
    pub(crate) tables: Vec<Table>,
    pub(crate) union_find: UnionFind,
    pub(crate) pending_unions: Vec<(Value, Value)>,
    pub(crate) native_functions: Vec<NativeFn>,

    // any canonical eclass -> all enode references of it
    pub(crate) reverse_index: ReverseIndex,

    // Cost tracking (eager incremental maintenance)
    pub(crate) cost_tracker:  CostTracker,
}

impl ActionCtx{
    pub(crate) fn alloc_id(&mut self) -> Value {
        let id = Value(self.union_find.parents.len() as u64);
        self.union_find.add(id);
        id
    }

    pub(crate) fn insert(&mut self, table_id: usize, key: Row, value: Value) {
        let table = &mut self.tables[table_id];
        let arity = table.arity();
        debug_assert_eq!(key.0.len(), arity);

        // Snapshot pre-insert canonicals so we can detect unions.
        let col = table.column_count();
        let existing_idx = table.key_index.get(&key).copied();
        let old_canonical =
            existing_idx.map(|idx| self.union_find.find(table.rows[idx.0 * col + arity]));
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
                // Track enode reference: value column -> reverse_index
                let value_type = &table.table_def.1[arity];
                if matches!(value_type, Type::Name(_) | Type::Base(BaseType::Id)) {
                    let canonical = self.union_find.find(value);
                    self.reverse_index.insert(canonical, table_id, idx);

                    // compute enode cost and update eclass minimum
                    self.cost_tracker.compute_and_update_eclass_cost(
                        &self.tables,
                        &self.union_find,
                        table_id,
                        idx,
                    );
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

    pub(crate) fn union(&mut self, old: Value, new: Value) {
        if let Some((parent, child)) = self.union_find.union(old, new) {
            self.reverse_index.merge(parent, child);

            // Merge eclass costs eagerly
            self.cost_tracker.merge_eclass_cost(parent, child);

            self.pending_unions.push((parent, child));
        }
    }

    pub(crate) fn apply_action(&mut self, action: &Action, rows: Set<Row>) {
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