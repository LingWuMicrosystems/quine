use alloc::{vec, vec::Vec};

use crate::{
    common::{Atom, TableName},
    core::{command::BackendCommand, rule},
    env::{DataTypeEnv, TableEnv},
    error::TypeCheckError,
    syntax::{Command, Op, VarName},
    types::{TableDef, Type},
};

#[derive(Debug, Default, Clone)]
pub struct CompileEnv {
    pub data_types: DataTypeEnv,
    pub table_types: TableEnv,
}

impl CompileEnv {
    pub fn check_and_compile(
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
                let rule = self.check_and_compile_rule(&rule)?;
                Ok(vec![BackendCommand::AddRule(rule)])
            }
            Command::Fact(fact) => todo!(),
            Command::Query(rule) => todo!(),
        }
    }

    pub fn check_and_compile_rule(
        &mut self,
        rule: &crate::syntax::Rule,
    ) -> Result<rule::Rule, TypeCheckError> {
        // let mut variables = vec![];
        todo!()
    }
}

#[derive(Debug, Clone)]
pub enum FlatClause {
    Lookup(VarName, TableName, Vec<VarName>),
    Eq(VarName, VarName),
    Constraint(VarName, Op, Atom),
    Guard(VarName, Op, VarName),
}
