// low level types

use alloc::boxed::Box;

use crate::{common::Name, types::BaseType};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RelationType(pub Name, Box<[BaseType]>);
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionType(pub Name, Box<[BaseType]>, Box<BaseType>);
