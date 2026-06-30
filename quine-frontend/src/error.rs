use alloc::{boxed::Box, string::String};
use quine_core::{common::ColumnIndex, types::Type};

use quine_core::atom::Atom;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CompileError {
    DuplicateTableName(String),
    DuplicateTypeName(String),
    UnknownTypeName(String),
    UnknownTypeNames(Box<[String]>),
    InvalidTableName(String),
    InvalidTableColumn(String, ColumnIndex),
    InvalidVariableName(String),
    InvalidAtomType(Atom, Type),
    VariableNotDefine(String),
    VariableInvalidInFact(Box<[String]>),
    TypeCheckError(Type, Type),
    InvalidTableWidth(usize, usize),
    InvalidExpression,
    MergeOnNonNumeric(String),
    MergeRequired(String),
    UnknownConstructor(String, String),
    VariableInExtract(String),
}
