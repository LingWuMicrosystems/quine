use alloc::vec::Vec;

use crate::{
    regraph::rule::{Action, Query, Rule},
    types::TableDef,
};

#[derive(Debug, Clone)]
pub enum BackendCommand {
    AddTables(Vec<TableDef>),
    AddRule(Rule),
    Action(Action),
    // repl only
    Query(Query),
}
