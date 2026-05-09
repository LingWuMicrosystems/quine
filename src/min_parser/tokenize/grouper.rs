use alloc::vec::Vec;
use nom::{IResult, Parser, branch::alt, multi::many0};

use crate::min_parser::tokenize::{
    error::{ParseError, tokenkind2groupkind},
    token::{Token, TokenKind, TokenSlice},
    token_tree::{GroupKind, TokenTree},
};

#[inline]
pub fn parse_brackets<'a>(tokens: &'a [Token<'a>]) -> Result<Vec<TokenTree<'a>>, ParseError> {
    let input = TokenSlice(tokens);
    parse_token_groups(None)(input)
        .and_then(|(remaining, result)| {
            if remaining.0.is_empty() {
                Ok(result)
            } else {
                Err(nom::Err::Incomplete(nom::Needed::Unknown))
            }
        })
        .map_err(|err| match err {
            nom::Err::Error(e) | nom::Err::Failure(e) => e,
            nom::Err::Incomplete(_) => unreachable!(),
        })
}

#[inline]
pub fn parse_token_groups(
    allowed_close: Option<TokenKind>,
) -> impl Fn(TokenSlice) -> IResult<TokenSlice, Vec<TokenTree>, ParseError> {
    move |input| {
        many0(alt((parse_paren, parse_brack, parse_brace, |input| {
            take_token_without_bracket(allowed_close)(input)
                .map(|(rest, r)| (rest, TokenTree::Token(r)))
        })))
        .parse(input)
    }
}

#[inline]
fn parse_paren(input: TokenSlice) -> IResult<TokenSlice, TokenTree, ParseError> {
    let (input, open_paren) = match_bracket(TokenKind::LParen)(input)?;
    let (input, body) = parse_token_groups(Some(TokenKind::RParen))(input)?;
    let (input, close_paren) = match_bracket(TokenKind::RParen)(input)?;

    Ok((
        input,
        TokenTree::Group {
            kind: GroupKind::Paren,
            body,
            start: open_paren.start,
            end: close_paren.end,
        },
    ))
}

#[inline]
fn parse_brace(input: TokenSlice) -> IResult<TokenSlice, TokenTree, ParseError> {
    let (input, open_brace) = match_bracket(TokenKind::LBrace)(input)?;
    let (input, body) = parse_token_groups(Some(TokenKind::RBrace))(input)?;
    let (input, close_brace) = match_bracket(TokenKind::RBrace)(input)?;

    Ok((
        input,
        TokenTree::Group {
            kind: GroupKind::Brace,
            body,
            start: open_brace.start,
            end: close_brace.end,
        },
    ))
}

#[inline]
fn parse_brack(input: TokenSlice) -> IResult<TokenSlice, TokenTree, ParseError> {
    let (input, open_brack) = match_bracket(TokenKind::LBrack)(input)?;
    let (input, body) = parse_token_groups(Some(TokenKind::RBrack))(input)?;
    let (input, close_brack) = (match_bracket(TokenKind::RBrack))(input)?;

    Ok((
        input,
        TokenTree::Group {
            kind: GroupKind::Brack,
            body,
            start: open_brack.start,
            end: close_brack.end,
        },
    ))
}

#[inline]
fn match_bracket<'a>(
    tok: TokenKind,
) -> impl Fn(TokenSlice<'a>) -> IResult<TokenSlice<'a>, Token<'a>, ParseError> {
    move |input: TokenSlice<'a>| {
        if let Some((first, rest)) = input.0.split_first() {
            if first.kind == tok {
                Ok((TokenSlice(rest), first.clone()))
            } else {
                Err(nom::Err::Error(ParseError::MismatchedBracket {
                    expected: tokenkind2groupkind(tok),
                    actual: first.kind,
                    position: first.start,
                }))
            }
        } else {
            match tok {
                TokenKind::RParen => Err(nom::Err::Failure(ParseError::Unclosed(GroupKind::Paren))),
                TokenKind::RBrace => Err(nom::Err::Failure(ParseError::Unclosed(GroupKind::Brace))),
                TokenKind::RBrack => Err(nom::Err::Failure(ParseError::Unclosed(GroupKind::Brack))),
                _ => Err(nom::Err::Error(ParseError::UnexpectedEof)),
            }
        }
    }
}

#[inline]
fn take_token_without_bracket<'a>(
    allowed_close: Option<TokenKind>,
) -> impl Fn(TokenSlice<'a>) -> IResult<TokenSlice<'a>, Token<'a>, ParseError> {
    move |input| {
        if let Some((tok, rest)) = input.0.split_first() {
            if let Some(allowed_close) = allowed_close
                && tok.kind == allowed_close
            {
                // return any error, because top level(many0) is processed
                // tokenkind2groupkind must input Closed bracket
                return Err(nom::Err::Error(ParseError::UnexpectedClose(
                    tokenkind2groupkind(allowed_close),
                    tok.start,
                )));
            }
            match tok.kind {
                TokenKind::RParen => Err(nom::Err::Failure(ParseError::UnexpectedClose(
                    GroupKind::Paren,
                    tok.start,
                ))),
                TokenKind::RBrace => Err(nom::Err::Failure(ParseError::UnexpectedClose(
                    GroupKind::Brace,
                    tok.start,
                ))),
                TokenKind::RBrack => Err(nom::Err::Failure(ParseError::UnexpectedClose(
                    GroupKind::Brack,
                    tok.start,
                ))),
                TokenKind::LParen => Err(nom::Err::Error(ParseError::Unclosed(GroupKind::Paren))),
                TokenKind::LBrace => Err(nom::Err::Error(ParseError::Unclosed(GroupKind::Brace))),
                TokenKind::LBrack => Err(nom::Err::Error(ParseError::Unclosed(GroupKind::Brack))),
                _ => Ok((TokenSlice(rest), tok.clone())),
            }
        } else {
            Err(nom::Err::Error(ParseError::UnexpectedEof))
        }
    }
}
