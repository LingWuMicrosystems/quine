use alloc::{format, vec, vec::Vec};

use crate::{
    common::{Map, Name},
    core::{command::BackendCommand, rule},
    error::TypeCheckError,
    syntax::Command,
    types::{TableDef, Type, TypeDef},
};

#[derive(Debug, Default, Clone)]
pub struct Env {
    pub data_types: DataTypeEnv,
    pub table_types: TableEnv,
}

impl Env {
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
        todo!()
    }
}

#[derive(Debug, Default, Clone)]
pub struct TableEnv {
    pub tables: Vec<TableDef>,
    pub name_map: Map<Name, usize>,
}

impl TableEnv {
    pub fn insert(&mut self, name: Name, def: TableDef) -> Result<(), TypeCheckError> {
        let offset = self.tables.len();
        if self.name_map.contains_key(&name) {
            return Err(TypeCheckError::DuplicateName(name));
        }
        self.tables.push(def.clone());
        self.name_map.insert(name, offset);
        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct DataTypeEnv {
    pub type_list: Vec<TypeDef>,
    pub name2type_map: Map<Name, usize>,
    // pub type2name_map: Map<TypeDef, usize>,
    // constructor name
    pub cons2type_map: Map<Name, usize>,
}

impl DataTypeEnv {
    pub fn insert(&mut self, name: Name, type_def: TypeDef) -> Result<(), TypeCheckError> {
        if self.name2type_map.contains_key(&name) {
            return Err(TypeCheckError::DuplicateName(name));
        }
        let offset = self.type_list.len();
        self.type_list.push(type_def.clone());

        for cons in type_def.1.0 {
            self.cons2type_map
                .insert(format!("{}.{}", type_def.0, cons.0.clone()), offset);
        }
        self.name2type_map.insert(name, offset);
        // self.type2name_map.insert(type_def, offset);
        Ok(())
    }
}
