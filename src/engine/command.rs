use quine_core::{
    rule::{Action, Query, Rule},
    types::TableDef,
};

#[derive(Debug, Clone)]
pub enum BackendCommand {
    AddTables(Vec<TableDef>),
    AddRule(Rule),
    Action(Action),
    Run,
    Query(Query, Vec<String>),
}
