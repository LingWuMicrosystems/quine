use core::fmt;

use crate::min_parser::{
    tokenize::token::{Position, TokenKind, TokenSlice},
    tokenize::token_tree::GroupKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    UnexpectedEof,
    Unclosed(GroupKind),
    UnexpectedClose(GroupKind, Position),
    MismatchedBracket {
        expected: GroupKind,
        actual: TokenKind,
        position: Position,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unclosed(kind) => {
                write!(f, "Unclosed {kind:?} bracket")
            }
            Self::UnexpectedClose(kind, position) => {
                write!(
                    f,
                    "Unexpected closing {kind:?} bracket at line {}:{}",
                    position.line, position.column
                )
            }
            Self::MismatchedBracket {
                expected,
                actual,
                position,
            } => {
                write!(
                    f,
                    "Mismatched bracket: expected {:?} but found {:?} at line {}:{}",
                    expected, actual, position.line, position.column
                )
            }
            Self::UnexpectedEof => write!(f, "Unexpected EOF",),
        }
    }
}

impl core::error::Error for ParseError {}

impl<'a> nom::error::ParseError<TokenSlice<'a>> for ParseError {
    fn from_error_kind(input: TokenSlice<'a>, _kind: nom::error::ErrorKind) -> Self {
        if let Some((tok, _)) = input.0.split_first() {
            match tok.kind {
                TokenKind::LParen | TokenKind::LBrace | TokenKind::LBrack => {
                    Self::Unclosed(tokenkind2groupkind(tok.kind))
                }
                TokenKind::RParen | TokenKind::RBrace | TokenKind::RBrack => {
                    Self::UnexpectedClose(tokenkind2groupkind(tok.kind), tok.start)
                }
                _ => unreachable!(),
            }
        } else {
            Self::UnexpectedEof
        }
    }

    fn append(_input: TokenSlice<'a>, _kind: nom::error::ErrorKind, other: Self) -> Self {
        other
    }
}

#[must_use]
pub fn tokenkind2groupkind(tok: TokenKind) -> GroupKind {
    match tok {
        TokenKind::LParen | TokenKind::RParen => GroupKind::Paren,
        TokenKind::LBrace | TokenKind::RBrace => GroupKind::Brace,
        TokenKind::LBrack | TokenKind::RBrack => GroupKind::Brack,
        _ => unreachable!(),
    }
}
