use alloc::boxed::Box;

use crate::common::Name;

// high level types

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeDef(pub Name, pub SumType);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SumType(pub Box<[TypeConstructor]>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeConstructor(pub Name, pub Box<[Type]>);

/// if TableDef.2 is Some, then it is a function type, otherwise it is a relation type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TableDef(pub Name, pub Box<[Type]>, pub Option<Type>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Base(BaseType),
    Name(Name),
}

impl Type {
    pub fn to_base_type(&self) -> Type {
        match self {
            Type::Base(base) => Type::Base(base.clone()),
            Type::Name(_) => Type::Base(BaseType::Id),
        }
    }
}

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
