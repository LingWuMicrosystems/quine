use alloc::boxed::Box;

use crate::common::Name;

// high level types

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeDef {
    Relation(RelationType),
    Function(FunctionType),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RelationType(pub Name, Box<[Type]>);
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionType(pub Name, Box<[Type]>, Box<Type>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Base(BaseType),
    Name(Option<Name>),
    SumType(SumType),
    Constructor(TypeConstructor),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeConstructor(pub Name, Box<[Type]>);

pub type SumType = Box<[TypeConstructor]>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BaseType {
    Id,
    I1,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    F32,

    // outer arena types
    I64,
    U64,
    F64,
    Str,
}
