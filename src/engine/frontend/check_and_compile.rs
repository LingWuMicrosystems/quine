use alloc::format;
use alloc::vec;

use crate::engine::EngineContext;
use crate::engine::error::CompileError;
use crate::engine::frontend::body2action::bodys2action;
use crate::engine::frontend::head2query::heads2query;
use crate::regraph::rule;
use crate::{
    engine::command::BackendCommand,
    engine::frontend::syntax::{self, Command},
    regraph::rule::VariableRecord,
    regraph::types::{TableDef, Type},
};

impl EngineContext {
    pub fn check_and_compile_command(
        &mut self,
        command: Command,
    ) -> Result<BackendCommand, CompileError> {
        match command {
            Command::TypeDef(name, type_def) => {
                let mut r = vec![];
                for cons in &type_def.1.0 {
                    let table_name = format!("{}.{}", name.0, cons.0);
                    let table_def = TableDef(table_name.clone(), cons.1.clone(), None);
                    self.table_types.insert(table_name, table_def.clone())?;
                    r.push(table_def);
                }
                self.data_types.insert(name.clone(), type_def)?;
                Ok(BackendCommand::AddTables(r))
            }
            Command::TableDef(name, table_def) => {
                let table_def = TableDef(
                    table_def.0,
                    table_def.1.iter().map(Type::to_base_type).collect(),
                    table_def.2.as_ref().map(Type::to_base_type),
                );
                self.table_types.insert(name, table_def.clone())?;
                Ok(BackendCommand::AddTables(vec![table_def]))
            }
            Command::Rule(rule) => Ok(BackendCommand::AddRule(self.check_and_compile_rule(&rule)?)),
            Command::Fact(fact) => Ok(BackendCommand::Action(bodys2action(
                &fact,
                &self.table_types.name_map,
                &VariableRecord::default(),
            )?)),
            Command::Query(head) => Ok(BackendCommand::Query(heads2query(
                &head,
                &self.table_types,
            )?)),
        }
    }

    fn check_and_compile_rule(&self, rule: &syntax::Rule) -> Result<rule::Rule, CompileError> {
        let query = heads2query(&rule.heads, &self.table_types)?;
        let action = bodys2action(&rule.bodys, &self.table_types.name_map, &query.variables)?;
        Ok(rule::Rule { query, action })
    }
}
