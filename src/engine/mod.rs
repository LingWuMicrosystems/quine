pub mod command;
pub mod env;
pub mod error;
pub mod frontend;

use std::dbg;

use alloc::vec::Vec;

use smallvec::smallvec;

use crate::engine::env::TableEnv;
use crate::{
    engine::command::BackendCommand,
    engine::env::CompileEnv,
    engine::error::CompileError,
    regraph::common::Set,
    regraph::{
        related_egraph::RelatedEGraph,
        rule::{Rule, VariableRecord},
        table::Row,
    },
};

#[derive(Debug, Default, Clone)]
pub struct EngineContext {
    pub data_types: CompileEnv,
    pub table_types: TableEnv,
    pub regraph: RelatedEGraph,
    pub ruleset: Vec<Rule>,
}

impl EngineContext {
    pub fn run_command(
        &mut self,
        cmd: command::BackendCommand,
    ) -> Result<Option<(VariableRecord, Set<Row>)>, CompileError> {
        match cmd {
            BackendCommand::AddTables(table_defs) => {
                dbg!(&table_defs);
                for table_def in table_defs {
                    self.regraph.add_table(table_def);
                }
                Ok(None)
            }
            BackendCommand::AddRule(rule) => {
                self.ruleset.push(rule);
                Ok(None)
            }
            BackendCommand::Action(action) => {
                self.regraph
                    .apply_action(&action, Set::from_iter([Row(smallvec![])]));
                Ok(None)
            }
            BackendCommand::Query(query) => {
                let result = self.regraph.run_query(&query);
                Ok(Some((query.variables.clone(), result)))
            }
        }
    }
}
