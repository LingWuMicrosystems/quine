use alloc::vec::Vec;

use crate::{
    engine::frontend::syntax::VarName,
    regraph::{
        rule::{Action, Query, Rule},
        types::TableDef,
    },
};

#[derive(Debug, Clone)]
pub enum BackendCommand {
    AddTables(Vec<TableDef>),
    AddRule(Rule),
    Action(Action),
    Query(Query, Vec<VarName>),
    Run,
}
