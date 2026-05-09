use alloc::{format, vec::Vec};

use crate::{
    common::{ConstructorName, Map, TableName, TypeName},
    frontend::error::CompileError,
    types::{TableDef, TypeDef},
};

#[derive(Debug, Default, Clone)]
pub struct TableEnv {
    pub tables: Vec<TableDef>,
    pub name_map: Map<TableName, usize>,
}

impl TableEnv {
    pub fn insert(&mut self, name: TableName, def: TableDef) -> Result<(), CompileError> {
        let offset = self.tables.len();
        if self.name_map.contains_key(&name) {
            return Err(CompileError::DuplicateTableName(name));
        }
        self.tables.push(def.clone());
        self.name_map.insert(name, offset);
        Ok(())
    }

    pub fn get_offset(&self, name: &TableName) -> Option<usize> {
        self.name_map.get(name).copied()
    }

    pub fn get(&self, offset: usize) -> Option<&TableDef> {
        self.tables.get(offset)
    }

    pub fn get_from_name(&self, name: &TableName) -> Option<&TableDef> {
        let offset = self.name_map.get(name)?;
        self.tables.get(*offset)
    }
}

#[derive(Debug, Default, Clone)]
pub struct DataTypeEnv {
    pub type_list: Vec<TypeDef>,
    pub name2type_map: Map<TypeName, usize>,
    // pub type2name_map: Map<TypeDef, usize>,
    // constructor name
    pub cons2type_map: Map<ConstructorName, usize>,
}

impl DataTypeEnv {
    pub fn insert(&mut self, name: TypeName, type_def: TypeDef) -> Result<(), CompileError> {
        if self.name2type_map.contains_key(&name) {
            return Err(CompileError::DuplicateTypeName(name));
        }
        let offset = self.type_list.len();
        self.type_list.push(type_def.clone());

        for cons in type_def.1.0 {
            self.cons2type_map.insert(
                ConstructorName(format!("{}.{}", type_def.0, cons.0.clone())),
                offset,
            );
        }
        self.name2type_map.insert(name, offset);
        // self.type2name_map.insert(type_def, offset);
        Ok(())
    }
}
