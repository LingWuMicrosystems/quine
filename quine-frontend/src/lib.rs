#![no_std]

extern crate alloc;

pub mod compile;
pub mod env;
pub mod error;
pub mod interner;
pub mod prelude;
pub mod syntax;
pub mod term;

use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use quine_core::common::{Map, RowIndex, Set, Value};
use quine_core::related_egraph::TableId;
use quine_core::related_egraph::{GroupName, NativeFn, RelatedEGraph, RunMode};
use quine_core::rule::{self, Query, VariableRecord};
use quine_core::table::Row;
use quine_core::types::*;

use crate::env::{CompileEnv, TableEnv};
use crate::interner::Interner;
use crate::syntax::{Atom, AtomOrVariable, CostDef, Expr, FunctionCall};
use crate::term::Term;

#[derive(Debug, Clone)]
pub struct NativeSignature {
    pub args: Box<[BaseType]>,
    pub ret: BaseType,
}

#[derive(Debug, Clone)]
pub enum CompiledUnit {
    TableDefs(Box<[TableDef]>),
    Rule(Option<String>, rule::Rule),
    Action(rule::Action),
    Run(Run),
    CostDef(CostDef),
    Extract(Expr),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Run(pub RunMode, pub RunBody);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RunBody {
    All,
    Group(GroupName),
    Program(Box<[Run]>),
}

#[derive(Debug, Default, Clone)]
pub struct EngineContext {
    pub data_types: CompileEnv,
    pub table_types: TableEnv,
    pub interner: Interner,
    pub regraph: RelatedEGraph,
    pub native_names: Map<String, usize>,
    pub native_signatures: Map<String, NativeSignature>,
    /// Result of the last `extract <expr>` evaluation (set by apply).
    pub last_extract: Option<Term>,
}

impl EngineContext {
    pub fn apply(&mut self, unit: CompiledUnit) {
        match unit {
            CompiledUnit::TableDefs(table_defs) => {
                for table_def in table_defs {
                    self.regraph.add_table(table_def);
                }
            }
            CompiledUnit::Rule(group_name, rule) => self.regraph.add_rule(group_name, rule),
            CompiledUnit::Action(action) => {
                self.regraph
                    .apply_action(&action, Set::from_iter([Row::default()]));
                self.regraph.rebuild();
            }
            CompiledUnit::Run(run) => {
                self.apply_run(&run);
            }
            CompiledUnit::CostDef(def) => {
                let key = format!("{}.{}", def.type_name, def.constructor);
                self.regraph.set_cost_model(key, def.cost);
            }
            CompiledUnit::Extract(expr) => {
                self.last_extract = Some(self.evaluate_and_extract(&expr));
            }
        }
    }

    pub fn apply_run(&mut self, run: &Run) -> bool {
        match &run.1 {
            RunBody::All => self.regraph.run_semi_naive(None, run.0),
            RunBody::Group(name) => {
                let rules = self.regraph.rule_groups.get(name).cloned();
                self.regraph.run_semi_naive(rules.as_ref(), run.0)
            }
            RunBody::Program(inner) => {
                let mut iteration = 0;
                loop {
                    if let RunMode::Repeat(count) = run.0
                        && iteration >= count
                    {
                        return false;
                    }
                    if self.apply_run_program(inner) {
                        return true;
                    }
                    iteration += 1;
                }
            }
        }
    }

    pub fn apply_run_program(&mut self, run_body: &[Run]) -> bool {
        let mut sat = true;
        for run in run_body {
            sat &= self.apply_run(run);
        }
        sat
    }

    pub fn query(&mut self, query: &Query, vars: &[String]) -> (VariableRecord, Set<Row>) {
        let mut result = self.regraph.run_query(query, None);
        if vars.is_empty() {
            return (query.variables.clone(), result);
        }
        let mut proj_record = VariableRecord::default();
        for name in vars {
            let offset = query.variables.get_offset(name).unwrap();
            let ty = query.variables.get_type(offset).unwrap();
            proj_record.insert_var(Some(name.clone()), ty.clone());
        }
        let offsets: Vec<_> = vars
            .iter()
            .map(|n| query.variables.get_offset(n).unwrap())
            .collect();
        result = result
            .into_iter()
            .map(|row| Row(offsets.iter().map(|&o| *row.0.get(o).unwrap()).collect()))
            .collect();
        (proj_record, result)
    }

    pub fn register_native(
        &mut self,
        name: &str,
        args: &[BaseType],
        ret: BaseType,
        func: NativeFn,
    ) {
        let offset = self.regraph.register_native_fn(func);
        self.native_names.insert(name.into(), offset);
        self.native_signatures.insert(
            name.into(),
            NativeSignature {
                args: args.into(),
                ret,
            },
        );
    }

    pub fn native_offset(&self, name: &str) -> Option<usize> {
        self.native_names.get(name).copied()
    }

    /// Look up the cost of a constructor. Returns 0 if not defined.
    pub fn constructor_cost(&self, type_name: &str, constructor: &str) -> u64 {
        let key = format!("{}.{}", type_name, constructor);
        self.regraph.get_constructor_cost(&key)
    }

    /// Get the current minimum cost of an eclass. Returns u64::MAX if unknown.
    pub fn eclass_cost(&self, eclass: Value) -> u64 {
        self.regraph.eclass_cost(eclass)
    }

    /// Get the cheapest enode for an eclass, if any.
    pub fn cost_select(&self, eclass: Value) -> Option<(TableId, RowIndex)> {
        self.regraph.cost_select(eclass)
    }

    pub fn extract(&self, id: Value, ty: &Type) -> Term {
        let base = ty.to_base_type();
        if matches!(base, BaseType::Id) {
            let id = self.regraph.find(id);
            let mut visited = Set::default();
            self.extract_inner(id, &mut visited)
        } else {
            Term::Literal(self.atom_from_value(id, &base))
        }
    }

    fn extract_inner(&self, id: Value, visited: &mut Set<Value>) -> Term {
        if !visited.insert(id) {
            return Term::Cyclic;
        }

        let Some((tid, row_idx)) = self.regraph.find_defining_row(id) else {
            return Term::Literal(Atom::U64(id.0));
        };

        let table = self.regraph.get_table(tid);
        let row = table.get_all_row(row_idx);
        let mut children = Vec::new();

        for (i, v) in row.0[..table.arity()].iter().enumerate() {
            let ty = &table.table_def.1[i];
            let child = if matches!(ty, Type::Name(_) | Type::Base(BaseType::Id)) {
                self.extract_inner(self.regraph.find(*v), visited)
            } else {
                let base = ty.to_base_type();
                Term::Literal(self.atom_from_value(*v, &base))
            };
            children.push(child);
        }

        Term::App(table.table_def.0.clone(), children)
    }

    /// Evaluate an extract expression and return the cheapest equivalent Term.
    /// Chains evaluate_expr → materialize_cheapest. Atoms short-circuit directly.
    pub fn evaluate_and_extract(&self, expr: &Expr) -> Term {
        // Bare atoms are literals, not eclass references
        if let Expr::AtomOrVariable(AtomOrVariable::Atom(atom)) = expr {
            return Term::Literal(atom.clone());
        }
        match self.evaluate_expr(expr) {
            Ok(eclass) => self.materialize_cheapest(eclass),
            Err(msg) => Term::Literal(Atom::Str(alloc::format!("<error: {msg}>"))),
        }
    }

    /// Resolve an `Expr` to a canonical eclass Value by looking up
    /// constructors in the e-graph tables.
    pub fn evaluate_expr(&self, expr: &Expr) -> Result<Value, String> {
        match expr {
            Expr::AtomOrVariable(AtomOrVariable::Atom(atom)) => {
                // Atoms are literal values — don't canonicalize via union_find
                Ok(self.value_from_atom(atom))
            }
            Expr::AtomOrVariable(AtomOrVariable::Variable(name)) => {
                Err(alloc::format!("variable '{name}' not valid in extract expression"))
            }
            Expr::FunctionCall(FunctionCall(name, args)) => {
                // Resolve constructor name to TableId
                let tid = self.resolve_constructor_to_table(name)?;

                // Recursively evaluate each argument
                let mut key_values: Vec<Value> = Vec::new();
                for arg in args.iter() {
                    key_values.push(self.evaluate_expr(arg)?);
                }

                let table = self.regraph.get_table(tid);
                let arity = table.arity();

                if key_values.len() != arity {
                    return Err(alloc::format!(
                        "constructor '{name}' expects {arity} args, got {}",
                        key_values.len()
                    ));
                }

                let key = Row(key_values.into());
                match table.get_by_key(&key) {
                    Some(row_idx) => {
                        let row = table.get_all_row(row_idx);
                        let value = row.0[arity];
                        Ok(self.regraph.find(value))
                    }
                    None => Err(alloc::format!(
                        "no matching row for '{name}' with given arguments in e-graph"
                    )),
                }
            }
        }
    }

    /// Cost-aware recursive materialization from an eclass.
    /// Uses cost_select to pick the cheapest enode at each level.
    pub fn materialize_cheapest(&self, eclass: Value) -> Term {
        let canonical = self.regraph.find(eclass);
        let mut visited = Set::default();
        self.materialize_cheapest_inner(canonical, &mut visited)
    }

    fn materialize_cheapest_inner(&self, eclass: Value, visited: &mut Set<Value>) -> Term {
        if !visited.insert(eclass) {
            return Term::Cyclic;
        }

        // Use cost_select to find the cheapest enode for this eclass
        let (tid, row_idx) = match self.regraph.cost_select(eclass) {
            Some(entry) => entry,
            None => {
                // No cost info — fall back to extract_inner (scan-based)
                return self.extract_inner(eclass, visited);
            }
        };

        let table = self.regraph.get_table(tid);
        let row = table.get_all_row(row_idx);
        let arity = table.arity();
        let mut children = Vec::new();

        for i in 0..arity {
            let child_ty = &table.table_def.1[i];
            let child_val = row.0[i];
            let child = if matches!(child_ty, Type::Name(_) | Type::Base(BaseType::Id)) {
                let child_canon = self.regraph.find(child_val);
                // Recursively materialize child using cost_select for cheapest
                self.materialize_cheapest_inner(child_canon, visited)
            } else {
                let base = child_ty.to_base_type();
                Term::Literal(self.atom_from_value(child_val, &base))
            };
            children.push(child);
        }

        Term::App(table.table_def.0.clone(), children)
    }

    /// Resolve a constructor name to a TableId.
    /// Tries direct table lookup first, then cons2type_map for short names.
    fn resolve_constructor_to_table(&self, name: &str) -> Result<TableId, String> {
        // Direct lookup
        if let Some(&tid) = self.table_types.name_map.get(name) {
            return Ok(tid);
        }
        // Try cons2type_map — search for keys ending with ".name"
        let dotted = alloc::format!(".{name}");
        for key in self.data_types.cons2type_map.keys() {
            if key.ends_with(&dotted) {
                // Found the full qualified name, look it up in table_types
                if let Some(&tid) = self.table_types.name_map.get(key) {
                    return Ok(tid);
                }
            }
        }
        Err(alloc::format!("unknown constructor or table: '{name}'"))
    }

    /// Convert an Atom to a Value (reverse of atom_from_value).
    /// Strings are expected to already be interned.
    fn value_from_atom(&self, atom: &Atom) -> Value {
        match atom {
            Atom::Str(s) => {
                // Search the interner for this string
                for id in 0..self.interner.max_id() {
                    if let Some(interned) = self.interner.lookup(id) {
                        if interned == s {
                            return Value(id as u64);
                        }
                    }
                }
                Value(u64::MAX)
            }
            Atom::I8(i) => Value::encode_i8(*i),
            Atom::I16(i) => Value::encode_i16(*i),
            Atom::I32(i) => Value::encode_i32(*i),
            Atom::I64(i) => Value::encode_i64(*i),
            Atom::U8(u) => Value(*u as u64),
            Atom::U16(u) => Value(*u as u64),
            Atom::U32(u) => Value(*u as u64),
            Atom::U64(u) => Value(*u),
            Atom::Bool(b) => Value(if *b { 1u64 } else { 0u64 }),
            Atom::F32(bits) => Value::encode_f32(f32::from_bits(*bits)),
            Atom::F64(bits) => Value::encode_f64(f64::from_bits(*bits)),
        }
    }

    fn atom_from_value(&self, v: Value, base: &BaseType) -> Atom {
        match base {
            BaseType::Id => Atom::U64(v.0),
            BaseType::Str => {
                let id = v.0 as u32;
                match self.interner.lookup(id) {
                    Some(s) => Atom::Str(s.to_owned()),
                    None => Atom::Str(format!("#str{}", id)),
                }
            }
            BaseType::I1 => Atom::Bool(v.0 != 0),
            BaseType::I8 => Atom::I8(v.decode_i8()),
            BaseType::U8 => Atom::U8(v.0 as u8),
            BaseType::I16 => Atom::I16(v.decode_i16()),
            BaseType::U16 => Atom::U16(v.0 as u16),
            BaseType::I32 => Atom::I32(v.decode_i32()),
            BaseType::U32 => Atom::U32(v.0 as u32),
            BaseType::I64 => Atom::I64(v.decode_i64()),
            BaseType::U64 => Atom::U64(v.0),
            BaseType::F32 => Atom::F32(v.decode_f32().to_bits()),
            BaseType::F64 => Atom::F64(v.decode_f64().to_bits()),
        }
    }
}
