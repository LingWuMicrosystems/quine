use alloc::{format, vec::Vec};

use crate::{
    common::{ConstructorName, Map, TableName, TypeName},
    error::TypeCheckError,
    types::{TableDef, TypeDef},
};

#[derive(Debug, Default, Clone)]
pub struct TableEnv {
    pub tables: Vec<TableDef>,
    pub name_map: Map<TableName, usize>,
}

impl TableEnv {
    pub fn insert(&mut self, name: TableName, def: TableDef) -> Result<(), TypeCheckError> {
        let offset = self.tables.len();
        if self.name_map.contains_key(&name) {
            return Err(TypeCheckError::DuplicateTableName(name));
        }
        self.tables.push(def.clone());
        self.name_map.insert(name, offset);
        Ok(())
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
    pub fn insert(&mut self, name: TypeName, type_def: TypeDef) -> Result<(), TypeCheckError> {
        if self.name2type_map.contains_key(&name) {
            return Err(TypeCheckError::DuplicateTypeName(name));
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
