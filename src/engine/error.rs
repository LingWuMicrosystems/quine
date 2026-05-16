use quine_core::{
    common::{Atom, ColumnIndex},
    types::Type,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CompileError {
    DuplicateTableName(String),
    DuplicateTypeName(String),
    UnknownTypeName(String),
    InvalidTableName(String),
    InvalidTableColumn(String, ColumnIndex),
    InvalidVariableName(String),
    InvalidAtomType(Atom, Type),
    VariableNotDefine(String),
    VariableInvalidInFact(Box<[String]>),
    TypeCheckError(Type, Type),
    InvalidTableWidth(usize, usize),
    InvalidExpression,
}
