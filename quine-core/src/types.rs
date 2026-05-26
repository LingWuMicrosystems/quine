use core::fmt::Display;

use alloc::{boxed::Box, string::String};

// high level types

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeDef(pub String, pub SumType);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SumType(pub Box<[TypeConstructor]>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeConstructor(pub String, pub Box<[Type]>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MergeFn {
    Min,
    Max,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TableDef(pub String, pub Box<[Type]>, pub Option<MergeFn>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Base(BaseType),
    Name(String),
}

impl Type {
    pub fn is_sign(&self) -> bool {
        let Type::Base(base) = self else { return false };
        base.is_sign()
    }
}

impl Type {
    pub fn to_base_type(&self) -> BaseType {
        match self {
            Type::Base(base) => base.clone(),
            Type::Name(_) => BaseType::Id,
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

impl BaseType {
    pub fn is_sign(&self) -> bool {
        matches!(
            self,
            BaseType::I8 | BaseType::I16 | BaseType::I32 | BaseType::I64
        )
    }

    pub fn is_numeric(&self) -> bool {
        !matches!(self, BaseType::Id | BaseType::Str)
    }
}

impl Display for BaseType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = match self {
            BaseType::Id => "ID",
            BaseType::I1 => "i1",
            BaseType::I8 => "i8",
            BaseType::U8 => "u8",
            BaseType::I16 => "i16",
            BaseType::U16 => "u16",
            BaseType::I32 => "i32",
            BaseType::U32 => "u32",
            BaseType::F32 => "f32",
            BaseType::I64 => "i64",
            BaseType::U64 => "u64",
            BaseType::F64 => "f64",
            BaseType::Str => "str",
        };

        write!(f, "{s}")
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Type::Base(bt) => write!(f, "{bt:?}"),
            Type::Name(name) => write!(f, "{name}"),
        }
    }
}

impl Display for TypeConstructor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}(", self.0)?;
        for (idx, t) in self.1.iter().enumerate() {
            if idx != 0 {
                write!(f, ", ")?;
            }

            write!(f, "{t}")?;
        }

        write!(f, ")")
    }
}

impl Display for TypeDef {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "type {}", self.0)?;
        for con in &self.1.0 {
            writeln!(f, "| {con}")?;
        }

        Ok(())
    }
}

impl Display for TableDef {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "table {}",
            TypeConstructor(self.0.clone(), self.1.clone())
        )?;
        if let Some(merge) = &self.2 {
            match merge {
                MergeFn::Min => write!(f, " merge min"),
                MergeFn::Max => write!(f, " merge max"),
            }
        } else {
            Ok(())
        }
    }
}
