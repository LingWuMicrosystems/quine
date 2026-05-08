use alloc::vec;
use alloc::vec::Vec;

use crate::{
    core::command::BackendCommand,
    frontend::{
        env::{DataTypeEnv, TableEnv},
        error::TypeCheckError,
        syntax::Command,
        syntax2flat_clause::{
            AnonymousVarCounter, function_call2flat_clause, syntax_rule2flat_clause,
        },
    },
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
    ) -> Result<Vec<BackendCommand>, TypeCheckError> {
        match command {
            Command::TypeDef(name, type_def) => {
                self.data_types.insert(name, type_def)?;
                // todo
                Ok(vec![])
            }
            Command::TableDef(name, table_def) => {
                let table_def = TableDef(
                    table_def.0,
                    table_def.1.iter().map(Type::to_base_type).collect(),
                    table_def.2.as_ref().map(Type::to_base_type),
                );
                self.table_types.insert(name, table_def.clone())?;
                Ok(vec![BackendCommand::AddTable(table_def)])
            }
            Command::Rule(rule) => {
                // let rule = syntax_rule2flat_clause(&rule)?;
                // Ok(vec![BackendCommand::AddRule(rule)])
                todo!()
            }
            Command::Fact(fact) => {
                let mut vars = vec![];
                let mut clauses = vec![];
                let (_, r) = function_call2flat_clause(
                    &fact,
                    &mut vars,
                    &mut clauses,
                    AnonymousVarCounter::default(),
                );
                Ok(vec![])
            }
            Command::Query(rule) => todo!(),
        }
    }
}
