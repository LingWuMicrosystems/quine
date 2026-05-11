use alloc::vec::Vec;
use alloc::{boxed::Box, vec};

use crate::frontend::body2action::{bodys2action, function_call_transform};
use crate::frontend::head2query::heads2query;
use crate::regraph::rule::Action;
use crate::{
    common::Map,
    engine::command::BackendCommand,
    frontend::{
        env::{DataTypeEnv, TableEnv},
        error::CompileError,
        syntax::{self, Command},
    },
    regraph::rule::{self, VariableRecord},
    types::{TableDef, Type},
};

#[derive(Debug, Default, Clone)]
pub struct CompileEnv {
    pub data_types: DataTypeEnv,
    pub table_types: TableEnv,
}

impl CompileEnv {
    pub fn check_and_compile_command(
        &mut self,
        command: Command,
    ) -> Result<BackendCommand, CompileError> {
        match command {
            Command::TypeDef(name, type_def) => {
                self.data_types.insert(name, type_def)?;
                // todo
                Ok(BackendCommand::AddTables(vec![]))
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
            Command::Fact(fact) => {
                let mut lets = Vec::default();
                let _call = function_call_transform(
                    &fact,
                    &self.table_types.name_map,
                    &VariableRecord::default(),
                    &mut lets,
                    &Map::default(),
                )?;
                let lets = lets.into_boxed_slice();
                Ok(BackendCommand::Action(Action {
                    lets,
                    tail: Box::new([]),
                }))
            }
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
