use crate::common::{TableName, TypeName};

pub enum TypeCheckError {
    DuplicateTableName(TableName),
    DuplicateTypeName(TypeName),
}
