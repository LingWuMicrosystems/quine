use alloc::format;
use alloc::vec;
use alloc::vec::Vec;

use crate::engine::EngineContext;
use crate::engine::error::CompileError;
use crate::engine::frontend::body2action::{bodys2action, CompileCtx};
use crate::engine::frontend::head2query::heads2query;
use crate::regraph::rule;
use crate::{
    engine::command::BackendCommand,
    engine::frontend::syntax::{self, Command},
    regraph::rule::VariableRecord,
    regraph::types::TableDef,
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
                self.table_types.insert(name, table_def.clone())?;
                Ok(BackendCommand::AddTables(vec![table_def]))
            }
            Command::Rule(rule) => Ok(BackendCommand::AddRule(self.check_and_compile_rule(&rule)?)),
            Command::Fact(fact) => {
                let mut ctx = CompileCtx {
                    table_map: &self.table_types.name_map,
                    head_variables: &VariableRecord::default(),
                    variables: VariableRecord::default(),
                    lets: Vec::new(),
                    interner: &mut self.interner,
                    native_names: &self.native_names,
                    native_signatures: &self.native_signatures,
                };
                Ok(BackendCommand::Action(bodys2action(&mut ctx, &fact)?))
            }
            Command::Query(head) => Ok(BackendCommand::Query(heads2query(
                &head,
                &self.table_types,
                &mut self.interner,
            )?)),
            Command::Run => Ok(BackendCommand::Run),
        }
    }

    fn check_and_compile_rule(&mut self, rule: &syntax::Rule) -> Result<rule::Rule, CompileError> {
        let query = heads2query(&rule.heads, &self.table_types, &mut self.interner)?;
        let mut ctx = CompileCtx {
            table_map: &self.table_types.name_map,
            head_variables: &query.variables,
            variables: VariableRecord::default(),
            lets: Vec::new(),
            interner: &mut self.interner,
            native_names: &self.native_names,
            native_signatures: &self.native_signatures,
        };
        let action = bodys2action(&mut ctx, &rule.bodys)?;
        Ok(rule::Rule { query, action })
    }
}
