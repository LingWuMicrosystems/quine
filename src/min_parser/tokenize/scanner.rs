use alloc::{format, string::String, vec::Vec};
use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{is_not, tag, take_while, take_while1},
    character::{
        complete::{char, multispace0, satisfy},
        one_of,
    },
    combinator::{complete, map, not, opt, peek, recognize},
    multi::many0,
    sequence::{delimited, pair, preceded},
};
use nom_locate::LocatedSpan;

use crate::min_parser::tokenize::token::{Position, Token, TokenKind};

fn make_pos(span: &Span) -> Position {
    Position {
        offset: span.location_offset(),
        line: span.location_line(),
        column: span
            .get_column()
            .try_into()
            .expect("column size overflow u32"),
    }
}

fn make_token<'a>(kind: TokenKind, text: &'a str, start: &Span, end: &Span) -> Token<'a> {
    Token {
        kind,
        text,
        start: make_pos(start),
        end: make_pos(end),
    }
}

fn is_ident_start(c: char) -> bool {
    is_ident_continue(c) && !c.is_ascii_digit()
}

fn is_ident_continue(c: char) -> bool {
    !is_whitespace(c) && !is_paren(c) && !is_symbol_char(c)
}

fn is_symbol_char(c: char) -> bool {
    r".:->=|#<+*^%&!~/\,;@?$".contains(c)
}

fn is_whitespace(c: char) -> bool {
    " \t\n\r".contains(c)
}

fn is_paren(c: char) -> bool {
    "(){}[]".contains(c)
}

type Span<'a> = LocatedSpan<&'a str>;

fn ident(start: Span) -> IResult<Span, Token> {
    let (rest, s) =
        recognize(pair(satisfy(is_ident_start), take_while(is_ident_continue))).parse(start)?;
    Ok((
        rest,
        make_token(TokenKind::Ident, s.into_fragment(), &start, &rest),
    ))
}

fn symbol(start: Span) -> IResult<Span, Token> {
    let (rest, s) = take_while1(is_symbol_char)(start)?;
    Ok((
        rest,
        make_token(TokenKind::Symbol, s.fragment(), &start, &rest),
    ))
}

fn is_digit(c: char) -> bool {
    "0123456789".contains(c)
}

const fn is_hex_digit(c: char) -> bool {
    c.is_ascii_hexdigit()
}

const fn is_bin_digit(c: char) -> bool {
    c == '0' || c == '1' || c == '_'
}

fn dec_int(start: Span) -> IResult<Span, Token> {
    let (rest, s) = recognize(pair(
        satisfy(is_digit),
        pair(
            take_while(|c| is_digit(c) || c == '_'),
            not(peek(complete(satisfy(is_ident_start)))),
        ),
    ))
    .parse(start)?;
    Ok((
        rest,
        make_token(TokenKind::IntDec, s.fragment(), &start, &rest),
    ))
}

fn hex_int(start: Span) -> IResult<Span, Token> {
    let (rest, s) = recognize(preceded(
        tag("0x"),
        pair(
            satisfy(is_hex_digit),
            pair(
                take_while(|c: char| is_hex_digit(c) || c == '_'),
                not(peek(complete(satisfy(is_ident_start)))),
            ),
        ),
    ))
    .parse(start)?;
    Ok((
        rest,
        make_token(TokenKind::IntHex, s.fragment(), &start, &rest),
    ))
}

fn bin_int(start: Span) -> IResult<Span, Token> {
    let (rest, s) = recognize(preceded(
        tag("0b"),
        pair(
            take_while1(is_bin_digit),
            not(peek(complete(satisfy(is_ident_continue)))),
        ),
    ))
    .parse(start)?;
    Ok((
        rest,
        make_token(TokenKind::IntBin, s.fragment(), &start, &rest),
    ))
}

/// EXPONENT  ::= [eE] [+-]? DIGIT+
fn exponent(start: Span) -> IResult<Span, Span> {
    let (rest, s) = recognize(pair(
        one_of("eE"),
        pair(opt(one_of("+-")), take_while1(is_digit)),
    ))
    .parse(start)?;
    Ok((rest, s))
}

/// FLOAT     ::= DIGIT+ ("." DIGIT+ (EXPONENT)? | EXPONENT)
/// float must not be int.
fn float(start: Span) -> IResult<Span, Token> {
    let (rest, s) = recognize(preceded(
        // Peek to ensure there is a dot or exponent after digits
        peek(complete(pair(take_while1(is_digit), one_of(".eE")))),
        pair(
            take_while1(is_digit),
            pair(
                alt((
                    // Case 1: "." DIGIT* (EXPONENT)?
                    recognize(pair(char('.'), pair(take_while1(is_digit), opt(exponent)))),
                    // Case 2: EXPONENT (must have exponent if no dot)
                    complete(exponent),
                )),
                not(peek(complete(satisfy(is_ident_start)))),
            ),
        ),
    ))
    .parse(start)?;
    Ok((
        rest,
        make_token(TokenKind::Float, s.fragment(), &start, &rest),
    ))
}
fn number(start: Span) -> IResult<Span, Token> {
    alt((hex_int, bin_int, float, dec_int)).parse(start)
}

fn string(start: Span) -> IResult<Span, Token> {
    let (rest, s) = delimited(char('"'), is_not("\""), char('"')).parse(start)?;
    Ok((
        rest,
        make_token(TokenKind::String, s.fragment(), &start, &rest),
    ))
}

fn paren(start: Span) -> IResult<Span, Token> {
    let (rest, kind) = alt((
        map(char('('), |_| TokenKind::LParen),
        map(char(')'), |_| TokenKind::RParen),
        map(char('{'), |_| TokenKind::LBrace),
        map(char('}'), |_| TokenKind::RBrace),
        map(char('['), |_| TokenKind::LBrack),
        map(char(']'), |_| TokenKind::RBrack),
    ))
    .parse(start)?;
    Ok((
        rest,
        make_token(kind, &start.fragment()[..1], &start, &rest),
    ))
}

fn space(start: Span) -> IResult<Span, Token> {
    let (rest, s) = take_while1(|c| c == ' ').parse(start)?;
    Ok((
        rest,
        make_token(TokenKind::Space, s.fragment(), &start, &rest),
    ))
}

fn tab(start: Span) -> IResult<Span, Token> {
    let (rest, s) = take_while1(|c| c == '\t').parse(start)?;
    Ok((
        rest,
        make_token(TokenKind::Space, s.fragment(), &start, &rest),
    ))
}

fn newline(start: Span) -> IResult<Span, Token> {
    let (rest, s) = take_while1(|c| c == '\n' || c == '\r').parse(start)?;
    Ok((
        rest,
        make_token(TokenKind::Space, s.fragment(), &start, &rest),
    ))
}

fn whitespace(start: Span) -> IResult<Span, Token> {
    alt((space, newline, tab)).parse(start)
}

fn token(start: Span) -> IResult<Span, Token> {
    delimited(
        multispace0,
        alt((whitespace, paren, string, symbol, number, ident)),
        multispace0,
    )
    .parse(start)
}

pub fn scan(start: Span) -> IResult<Span, Vec<Token>> {
    many0(token).parse(start)
}

pub fn scan_tokens(input: &str) -> Result<Vec<Token<'_>>, String> {
    let span = LocatedSpan::new(input);
    let (remaining, tokens) = scan(span).map_err(|e| format!("Scan error: {e:?}"))?;
    if !remaining.fragment().is_empty() {
        return Err(format!(
            "Unparsed trailing characters: {:?}",
            remaining.fragment()
        ));
    }
    Ok(tokens)
}
