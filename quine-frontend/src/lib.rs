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
use quine_core::common::{Map, Set, Value};
use quine_core::related_egraph::{NativeFn, RelatedEGraph};
use quine_core::rule::{self, Query, VariableRecord};
use quine_core::table::Row;
use quine_core::types::*;

use crate::env::{CompileEnv, TableEnv};
use crate::interner::Interner;
use crate::syntax::Atom;
use crate::term::Term;

#[derive(Debug, Clone)]
pub struct NativeSignature {
    pub args: Box<[BaseType]>,
    pub ret: BaseType,
}

#[derive(Debug, Default, Clone)]
pub struct CompiledUnit {
    pub table_defs: Vec<TableDef>,
    pub rules: Vec<rule::Rule>,
    pub actions: Vec<rule::Action>,
}

#[derive(Debug, Default, Clone)]
pub struct EngineContext {
    pub data_types: CompileEnv,
    pub table_types: TableEnv,
    pub interner: Interner,
    pub regraph: RelatedEGraph,
    pub native_names: Map<String, usize>,
    pub native_signatures: Map<String, NativeSignature>,
}

impl EngineContext {
    pub fn apply(&mut self, unit: CompiledUnit) {
        for table_def in unit.table_defs {
            self.regraph.add_table(table_def);
        }
        for rule in unit.rules {
            self.regraph.add_rule(rule);
        }
        for action in unit.actions {
            self.regraph
                .apply_action(&action, Set::from_iter([Row::default()]));
        }
    }

    pub fn run_query(&mut self, query: &Query, vars: &[String]) -> (VariableRecord, Set<Row>) {
        let mut result = self.regraph.run_query(query);
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

    pub fn run(&mut self) {
        self.regraph.set_fully_dirty();
        self.regraph.run();
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
