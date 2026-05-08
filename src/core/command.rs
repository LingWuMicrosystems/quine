use alloc::boxed::Box;

use crate::{
    common::Name,
    core::rule::{Atom, Rule},
    types::TableDef,
};

pub enum BackendCommand {
    InsertRow(InsertRow),
    AddTable(TableDef),
    AddRule(Rule),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InsertRow(Name, Box<[Atom]>, Option<Atom>);
