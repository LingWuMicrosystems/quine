pub mod body2action;
pub mod head2query;

use alloc::format;
use alloc::vec;
use alloc::vec::Vec;

use crate::engine::EngineContext;
use crate::engine::command::BackendCommand;
use crate::engine::compile::body2action::{CompileCtx, bodys2action};
use crate::engine::compile::head2query::heads2query;
use crate::engine::error::CompileError;
use crate::engine::interner::Interner;
use crate::regraph::common::TypeName;
use crate::regraph::common::{Atom, Value};
use crate::regraph::rule;
use crate::regraph::rule::VariableRecord;
use crate::regraph::types::BaseType;
use crate::regraph::types::{TableDef, Type};
use crate::syntax::{self, Command};

pub fn atom_to_value(atom: &Atom, interner: &mut Interner) -> Value {
    match atom {
        Atom::Str(s) => Value(interner.intern(s.clone()) as u64),
        other => other.to_value(),
    }
}

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
                    let mut cols: Vec<Type> = cons.1.to_vec();
                    cols.push(Type::Base(BaseType::Id));
                    let table_def = TableDef(table_name.clone(), cols.into());
                    self.table_types.insert(table_name, table_def.clone())?;
                    r.push(table_def);
                }
                self.data_types.insert(name.clone(), type_def)?;
                Ok(BackendCommand::AddTables(r))
            }
            Command::TableDef(name, table_def) => {
                for ty in table_def.1.iter() {
                    self.check_type_defined(ty)?;
                }
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
            Command::Query(heads, vars) => Ok(BackendCommand::Query(
                heads2query(
                    &heads,
                    &self.table_types,
                    &self.data_types,
                    &mut self.interner,
                )?,
                vars,
            )),
            Command::Run => Ok(BackendCommand::Run),
        }
    }

    fn check_type_defined(&self, ty: &Type) -> Result<(), CompileError> {
        match ty {
            Type::Name(name)
                if !self
                    .data_types
                    .name2type_map
                    .contains_key(&TypeName(name.clone())) =>
            {
                Err(CompileError::UnknownTypeName(TypeName(name.clone())))
            }
            _ => Ok(()),
        }
    }

    fn check_and_compile_rule(&mut self, rule: &syntax::Rule) -> Result<rule::Rule, CompileError> {
        let query = heads2query(
            &rule.heads,
            &self.table_types,
            &self.data_types,
            &mut self.interner,
        )?;
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
