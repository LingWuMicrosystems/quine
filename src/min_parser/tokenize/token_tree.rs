use alloc::vec::Vec;

use crate::min_parser::tokenize::token::{Position, Token};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupKind {
    Paren,
    Brace,
    Brack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenTree<'a> {
    Token(Token<'a>),
    Group {
        kind: GroupKind,
        body: Vec<Self>,
        start: Position,
        end: Position,
    },
}
