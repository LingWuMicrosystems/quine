use alloc::boxed::Box;

use crate::{
    common::{ColumnIndex, TableName, TypeName},
    frontend::{head2flat_clause::NameOrVariable, syntax::VarName},
    types::Type,
};

pub enum CompileError {
    DuplicateTableName(TableName),
    DuplicateTypeName(TypeName),
    InvalidTableName(TableName),
    InvalidTableColumn(TableName, ColumnIndex),
    InvalidVariableName(NameOrVariable),
    VariableInvalidInFact(Box<[VarName]>),
    TypeUnificationError(NameOrVariable, Type, Type),
}
