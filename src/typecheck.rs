use alloc::vec::Vec;

use crate::{
    common::{Map, Name},
    syntax::ReplCommand,
    types::TypeDef,
};

#[derive(Debug, Default, Clone)]
pub struct TypeEnv {
    pub type_list: Vec<TypeDef>,
    pub name2type_map: Map<Name, usize>,
    pub type2name_map: Map<TypeDef, usize>,
    pub constructor2type_map: Map<Name, Name>,
}

impl TypeEnv {
    pub fn check(&mut self, command: ReplCommand) -> Result<(), ()> {
        match command {
            ReplCommand::TypeDef(name, type_def) => {
                self.insert(name, type_def);
                Ok(())
            }
            ReplCommand::Rule(rule) => todo!(),
            ReplCommand::Fact(fact) => todo!(),
            ReplCommand::Query(rule) => todo!(),
        }
    }

    pub fn insert(&mut self, name: Name, type_def: TypeDef) {
        self.0.insert(name, type_def);
    }
}
