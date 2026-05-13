pub mod command;
pub mod env;
pub mod error;
pub mod frontend;
pub mod interner;
pub mod term;

use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::format;
use alloc::vec::Vec;

use smallvec::smallvec;

use crate::engine::env::TableEnv;
use crate::engine::interner::Interner;
use crate::engine::term::Term;
use crate::regraph::common::{Atom, Set, TypeName, Value};
use crate::regraph::table::Column;
use crate::regraph::types::{BaseType, SumType, TableDef, Type, TypeDef};
use crate::{
    engine::command::BackendCommand,
    engine::env::CompileEnv,
    regraph::{related_egraph::RelatedEGraph, rule::VariableRecord, table::Row},
};

#[derive(Debug, Clone)]
pub struct EngineContext {
    pub data_types: CompileEnv,
    pub table_types: TableEnv,
    pub interner: Interner,
    pub regraph: RelatedEGraph,
}

impl Default for EngineContext {
    fn default() -> Self {
        let unit_type = TypeDef("Unit".to_owned(), SumType(Box::new([])));
        let mut data_types = CompileEnv::default();
        let _ = data_types.insert(TypeName("Unit".to_owned()), unit_type.clone());

        let unit_table = TableDef("Unit".to_owned(), Box::new([]), None);
        let mut table_types = TableEnv::default();
        let mut regraph = RelatedEGraph::default();
        let _ = table_types.insert("Unit".to_owned(), unit_table.clone());
        regraph.add_table(unit_table);
        let new_id = regraph.alloc_id();
        regraph.insert(0, Row(smallvec![]), new_id);

        Self {
            data_types,
            table_types,
            interner: Interner::default(),
            regraph,
        }
    }
}

impl EngineContext {
    pub fn run_command(
        &mut self,
        cmd: command::BackendCommand,
    ) -> Option<(VariableRecord, Set<Row>)> {
        match cmd {
            BackendCommand::AddTables(table_defs) => {
                for table_def in table_defs {
                    self.regraph.add_table(table_def);
                }
                None
            }
            BackendCommand::AddRule(rule) => {
                self.regraph.add_rule(rule);
                None
            }
            BackendCommand::Action(action) => {
                self.regraph
                    .apply_action(&action, Set::from_iter([Row(smallvec![])]));
                None
            }
            BackendCommand::Query(query) => {
                let result = self.regraph.run_query(&query);
                Some((query.variables.clone(), result))
            }
            BackendCommand::Run => {
                self.regraph.set_fully_dirty();
                self.regraph.run();
                None
            }
        }
    }

    pub fn extract(&self, id: Value, ty: &Type) -> Term {
        let id = self.regraph.find(id);
        let base = ty.to_base_type();
        if matches!(base, BaseType::Id) {
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
            let child = match &table.columns[i] {
                Column::Id(_) => self.extract_inner(self.regraph.find(*v), visited),
                col => {
                    let base = column_to_base_type(col);
                    Term::Literal(self.atom_from_value(*v, &base))
                }
            };
            children.push(child);
        }

        Term::App(table.name.clone(), children)
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
            BaseType::I8 => Atom::I8(v.0 as i8),
            BaseType::U8 => Atom::U8(v.0 as u8),
            BaseType::I16 => Atom::I16(v.0 as i16),
            BaseType::U16 => Atom::U16(v.0 as u16),
            BaseType::I32 => Atom::I32(v.0 as i32),
            BaseType::U32 => Atom::U32(v.0 as u32),
            BaseType::I64 => Atom::I64(v.0 as i64),
            BaseType::U64 => Atom::U64(v.0),
            BaseType::F32 => Atom::F32(v.0 as u32),
            BaseType::F64 => Atom::F64(v.0),
        }
    }
}

fn column_to_base_type(col: &Column) -> BaseType {
    match col {
        Column::Id(_) => BaseType::Id,
        Column::Str(_) => BaseType::Str,
        Column::I1(_) => BaseType::I1,
        Column::I8(_) => BaseType::I8,
        Column::U8(_) => BaseType::U8,
        Column::I16(_) => BaseType::I16,
        Column::U16(_) => BaseType::U16,
        Column::I32(_) => BaseType::I32,
        Column::U32(_) => BaseType::U32,
        Column::I64(_) => BaseType::I64,
        Column::U64(_) => BaseType::U64,
        Column::F32(_) => BaseType::F32,
        Column::F64(_) => BaseType::F64,
    }
}
