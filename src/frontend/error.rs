use alloc::boxed::Box;

use crate::{
    common::{ColumnIndex, TableName, TypeName},
    frontend::syntax::VarName,
};

pub enum CompileError {
    DuplicateTableName(TableName),
    DuplicateTypeName(TypeName),
    InvalidTableName(TableName),
    InvalidTableColumn(TableName, ColumnIndex),
    VariableInvalidInFact(Box<[VarName]>),
}
