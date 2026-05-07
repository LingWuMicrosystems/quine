use crate::{
    common::{Map, Name},
    types::SumType,
};

pub struct TypeEnv(pub Map<Name, SumType>);
