use alloc::boxed::Box;

use crate::{
    engine::frontend::syntax::VarName,
    regraph::common::{Atom, ColumnIndex, TableName, TypeName},
    regraph::types::Type,
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
