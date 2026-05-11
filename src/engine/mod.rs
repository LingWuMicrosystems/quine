use std::dbg;

use alloc::vec::Vec;

use smallvec::smallvec;

use crate::{
    common::Set,
    engine::command::BackendCommand,
    frontend::{check_and_compile::CompileEnv, error::CompileError},
    regraph::{
        regraph::RelatedEGraph,
        rule::{Rule, VariableRecord},
        table::Row,
    },
};

pub mod command;

#[derive(Debug, Default, Clone)]
pub struct EngineContext {
    pub compile_env: CompileEnv,
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
                for table_def in table_defs {
                    self.regraph.add_table(&table_def);
                }
                Ok(None)
            }
            BackendCommand::AddRule(rule) => {
                self.ruleset.push(rule);
                Ok(None)
            }
            BackendCommand::Action(action) => {
                dbg!(&action);
                self.regraph
                    .apply_action(&action, Set::from_iter([Row(smallvec![])].into_iter()));
                Ok(None)
            }
            BackendCommand::Query(query) => {
                let result = self.regraph.run_query(&query);
                Ok(Some((query.variables.clone(), result)))
            }
        }
    }
}
