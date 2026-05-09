use alloc::{boxed::Box, vec::Vec};

use crate::{
    common::{Atom, Name},
    core::rule::{Action, Query, Rule},
    types::TableDef,
};

#[derive(Debug, Clone)]
pub enum BackendCommand {
    Actions(Vec<Action>),
    AddTables(Vec<TableDef>),
    AddRule(Rule),
    // repl only
    Query(Query),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InsertRow(Name, Box<[Atom]>, Option<Atom>);
