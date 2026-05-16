use alloc::boxed::Box;

use crate::{
    regraph::{
        common::{Atom, ColumnIndex, TableName, TypeName},
        types::Type,
    },
    syntax::VarName,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CompileError {
    DuplicateTableName(TableName),
    DuplicateTypeName(TypeName),
    UnknownTypeName(TypeName),
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
