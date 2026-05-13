pub mod command;
pub mod env;
pub mod error;
pub mod frontend;
pub mod interner;

use alloc::borrow::ToOwned;
use alloc::boxed::Box;

use smallvec::smallvec;

use crate::engine::env::TableEnv;
use crate::engine::interner::Interner;
use crate::regraph::common::TypeName;
use crate::regraph::types::{SumType, TableDef, TypeDef};
use crate::{
    engine::command::BackendCommand,
    engine::env::CompileEnv,
    regraph::common::Set,
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
}
