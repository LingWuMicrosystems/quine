use alloc::boxed::Box;

use crate::{
    common::{Atom, ColumnIndex, TableName, TypeName},
    frontend::syntax::VarName,
    types::Type,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CompileError {
    DuplicateTableName(TableName),
    DuplicateTypeName(TypeName),
    InvalidTableName(TableName),
    InvalidTableColumn(TableName, ColumnIndex),
    InvalidVariableName(VarName),
    InvalidAtomType(Atom, Type),
    VariableNotDefine(VarName),
    VariableInvalidInFact(Box<[VarName]>),
    TypeCheckError(Type, Type),
    InvalidTableWidth(usize, usize),
    InvalidExpression,
}
