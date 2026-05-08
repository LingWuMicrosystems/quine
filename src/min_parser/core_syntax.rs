use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use alloc::format;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use crate::common::Atom;
use crate::core::rule::Op;
use crate::min_parser::parser::category::{
    Assoc, Category, ParserContext, ParserEnv, ParserFn, ParserResult, TrailingParserFn, parse_inside_group
};
use crate::min_parser::tokenize::grouper::parse_brackets;
use crate::min_parser::tokenize::scanner::scan_tokens;
use crate::min_parser::tokenize::token::TokenKind;
use crate::min_parser::tokenize::{
    token_tree::{GroupKind, TokenTree},
};
use crate::syntax::{AtomOrVariable, Body, Command, Expr, FunctionCall, Head, Pattern, Rule};
use crate::types::{BaseType, SumType, TableDef, Type, TypeConstructor, TypeDef};


// Maybe pest, our `min_parser` is hard to do mutual recursion.
// kws = _{ "if" | "leteq" | "union" | "insert" | "true" | "false" | "rule" | "fact" }
// WHITESPACE = _{ " " }
// ident_start = _{ 'a'..'z' | 'A'..'Z' }
// ident_continue = _{ 'a'..'z' | 'A'..'Z' | "_" | '0'..'9' }
//
// ident = @{ !kws ~ ident_start ~ ident_continue* }
//
// digit = _{ '0'..'9' }
// digits = @{ digit+ }
// atom = { "true" | "false" | "-"? ~ digits | ident }
// atom_expr = { atom | "(" ~ expr ~ ")" }
// expr = { atom_expr ~ expr* }
// 
// atom_pattern = { atom | "(" ~ pattern ~ ")" | "_" }
// pattern = { atom_pattern ~ pattern* }
//
// op = { "=" | ">" | "<" }
// head = {
//     | pattern
//     | "leteq" ~ atom_expr ~ atom_expr
//     | "if" ~ expr ~ op ~ expr
// }
// heads = { head ~ ("," ~ head)* }
//
// body = {
//   | "union" ~ atom_expr ~ atom_expr
//   | "insert" ~ atom_expr
// }
// bodies = { body ~ (";" ~ body)* }
//
// rule = { "rule" ~ heads ~ "=>" ~ bodies }

fn make_app<'a>(func: Expr, arg: Expr) -> Result<Expr, String> {
    match func {
        Expr::AtomOrVariable(AtomOrVariable::Variable(v)) =>
            Ok(Expr::FunctionCall(FunctionCall(v, Box::new([arg])))),
        Expr::FunctionCall(call) => {
            let mut args = call.1.to_vec();
            args.push(arg);
            Ok(Expr::FunctionCall(FunctionCall(call.0, args.into())))
        },
        Expr::AtomOrVariable(AtomOrVariable::Atom(_)) => Err("Cannot apply to atom".into()),
    }
}

fn initialize_category<T>() -> Category<T> {
    let mut c = Category::<T>::default();
    let reg_keyword = |name: &str| {
        let name = name.to_string(); //trick
        c.leading.insert(
            name.clone(),
            Rc::new(move |_ctx, _env| {
                Err(format!(
                    "'{name}' is a keyword and cannot be used as an expression"
                ))
            }),
        );
    };

    KEYWORDS.iter().map(|x| *x).for_each(reg_keyword);
    c
}

const KEYWORDS: [ &str; 24 ] = [ 
    "if", "leteq", "union", "insert", "rule", "fact", "type", "table", "query",
    "true", "false",
    "ID", "i1", "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64", "str",
];

const ATOM_LEADINGS: [ &str; 6 ] = [ "@ident", "-", "@int", "@str", "true", "false" ];
pub fn build_atom_parser_env<'a>() -> ParserEnv<AtomOrVariable> {
    let mut cat = initialize_category::<AtomOrVariable>();

    cat.leading.insert(
        "@ident".to_string(),
        Rc::new(|ctx, _env| {
            let (name, ctx) = ctx.expect_ident()?;
            Ok((AtomOrVariable::Variable(name), ctx))
        }),
    );

    let parse_int: fn(bool) -> Rc<ParserFn<AtomOrVariable>> = |is_neg| Rc::new(move |ctx, _env| {
        let (token, ctx) = ctx.next_token()?;
        let TokenTree::Token(t) = token else {
            return Err("Expected integer token".into());
        };

        let text = t.text;
        let num = match t.kind {
            TokenKind::IntDec => text.replace('_', "").parse::<u64>(),
            TokenKind::IntHex => u64::from_str_radix(&text.replace('_', "")[2..], 16),
            TokenKind::IntBin => u64::from_str_radix(&text.replace('_', "")[2..], 2),
            _ => return Err("Expected integer token".into()),
        }.map_err(|_| "Invalid integer")?;

        if is_neg {
            let num = i64::try_from(num)
                .map_err(|_| "Invalid integer")?;
            let num = - num;    // never overflow, since num is always positive
            Ok((AtomOrVariable::Atom(Atom::Int(num)), ctx))
        } else {
            Ok((AtomOrVariable::Atom(Atom::Uint(num)), ctx))
        }
    });

    cat.leading.insert("@int".to_string(), parse_int(false));
    let neg_parse_int = parse_int(true).clone();
    cat.leading.insert("-".into(), Rc::new(move |ctx, env| {
        let (_, ctx) = ctx.next_token()?;
        (*neg_parse_int)(ctx, env)
    }));

    let expect_string = "Expected string token";
    cat.leading.insert(
        "@str".to_string(),
        Rc::new(|ctx, _env| {
            let (token, ctx) = ctx.next_token()?;
            let TokenTree::Token(t) = token else {
                return Err(expect_string.into());
            };

            let text = t.text;
            let str_lit = match t.kind {
                TokenKind::String => text,
                _ => return Err(expect_string.into()),
            };

            let auv = AtomOrVariable::Variable(str_lit.into());
            Ok((auv, ctx))
        }),
    );

    cat.leading.insert("true".into(), Rc::new(|ctx, _| {
        let (_, ctx) = ctx.next_token()?;
        Ok((AtomOrVariable::Atom(Atom::Bool(true)), ctx))
    }));
    
    cat.leading.insert("false".into(), Rc::new(|ctx, _| {
        let (_, ctx) = ctx.next_token()?;
        Ok((AtomOrVariable::Atom(Atom::Bool(false)), ctx))
    }));

    let mut env = ParserEnv {
        categories: BTreeMap::new(),
    };

    env.categories.insert("Atom".to_string(), cat);
    env
}

const BASE_TYPE_LEADING: [ &str ; 13 ] = [ "ID", "i1", "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64", "str" ];
pub fn parse_base_type<'a>(ctx: ParserContext<'a>) -> ParserResult<'a, BaseType> {
    let (TokenTree::Token(t), ctx) = ctx.next_token()? else {
        return Err(format!("Expecting Type"));
    };

    let ty = match t.text {
        "ID" => BaseType::Id,
        "i1" => BaseType::I1,
        "i8" => BaseType::I8,
        "i16" => BaseType::I16,
        "i32" => BaseType::I32,
        "i64" => BaseType::I64,
        "u8" => BaseType::U8,
        "u16" => BaseType::U16,
        "u32" => BaseType::U32,
        "u64" => BaseType::U64,
        "f32" => BaseType::F32,
        "f64" => BaseType::F64,
        "str" => BaseType::Str,
        _ => return Err(format!("Unknown type: {}", t.text))
    };

    Ok((ty, ctx))
}

pub fn parse_type<'a>(ctx: ParserContext<'a>) -> ParserResult<'a, Type> {
    let TokenTree::Token(t) = ctx.peek().ok_or("Unexpected EOF")? else {
        return Err("Expecting type".into());
    };

    if BASE_TYPE_LEADING.contains(&t.text) {
        let (ty, ctx) = parse_base_type(ctx)?;
        Ok((Type::Base(ty), ctx))
    } else {
        let (name, ctx) = ctx.expect_ident()?;
        Ok((Type::Name(name), ctx))
    }
}

pub fn parse_type_list<'a>(ctx: ParserContext<'a>) -> ParserResult<'a, Box<[Type]>> {
    let mut types = Vec::new();

    let (ty, mut ctx) = parse_type(ctx)?;
    types.push(ty);

    while let Some(TokenTree::Token(t)) = ctx.peek()
    && t.text == "," {
        (_, ctx) = ctx.next_token()?;
        let (ty, mctx) = parse_type(ctx)?;
        ctx = mctx;
        types.push(ty);
    }

    Ok((types.into(), ctx))
}

// foo(A, B)
pub fn parse_type_constructor<'a>(ctx: ParserContext<'a>) -> ParserResult<'a, TypeConstructor> {
    let (name, ctx) = ctx.expect_ident()?;
    let (param_list, ctx) = ctx.next_token()?;
    let types= parse_inside_group(param_list, GroupKind::Paren, |ctx|{ 
        if ctx.peek().is_none() {
            Ok(([].into(), ctx))
        } else {
            parse_type_list(ctx)
        }
    })?;
    Ok((TypeConstructor(name, types), ctx))
}

const EXPR_LEADING: [ &str; 7 ] = [ "@ident", "-", "@int", "@str", "true", "false", "@paren" ];
#[must_use]
pub fn build_expr_parser_env(atom: Rc<ParserEnv<AtomOrVariable>>) -> ParserEnv<Expr> {
    let mut cat = initialize_category::<Expr>();

    for lead in ATOM_LEADINGS {
        let atom = atom.clone();
        cat.leading.insert(lead.to_string(), Rc::new(move |ctx, _| {
            let (atom, ctx) = ctx.parse(&atom, "Atom", 0)?;
            Ok((Expr::AtomOrVariable(atom), ctx))
        }));
    }

    // cat.leading.insert(
    //     "let".to_string(),
    //     Rc::new(|ctx, env| {
    //         let (_, ctx) = ctx.next_token()?;
    //         let (name, ctx) = ctx.expect_ident()?;
    //         let ((), ctx) = ctx.expect("=")?;
    //         let (val, ctx) = ctx.parse(env, "Expr", 0)?;
    //         let val = val.unwrap_expr()?;

    //         Ok((Body::Let(name, val).into(), ctx))
    //     }),
    // );

    cat.leading.insert(
        "@paren".to_string(),
        Rc::new(|ctx, env| {
            let (group, ctx) = ctx.next_token()?;
            let r =
                parse_inside_group(group, GroupKind::Paren, |inner| inner.parse(env, "Expr", 0))?;
            Ok((r, ctx))
        }),
    );

    let explicit_app: Rc<TrailingParserFn<Expr>> = Rc::new(|ctx, env, left, bp| {
        let (arg_expr, ctx) = ctx.parse(env, "Expr", bp)?;
        let expr = make_app(left, arg_expr)?;
        Ok((expr, ctx))
    });

    EXPR_LEADING.iter().for_each(|x| {
        cat.trailing.insert((*x).into(), (100, Assoc::Left, explicit_app.clone()));
    });

    let mut env = ParserEnv {
        categories: BTreeMap::new(),
    };

    env.categories.insert("Expr".to_string(), cat);
    env
}

const PATTERN_LEADING: [ &str; 8 ] = [ "@ident", "-", "@int", "@str", "true", "false", "@paren", "_" ];
pub fn build_pattern_parser_env(atom: Rc<ParserEnv<AtomOrVariable>>) -> ParserEnv<Pattern> {
    let mut cat = initialize_category::<Pattern>();

    for lead in ATOM_LEADINGS {
        let atom = atom.clone();
        cat.leading.insert(lead.to_string(), Rc::new(move |ctx, _| {
            let (atom, ctx) = ctx.parse(&atom, "Atom", 0)?;
            Ok((Pattern::AtomOrVariable(atom), ctx))
        }));
    }

    cat.leading.insert("_".into(), Rc::new(|ctx, _| {
        let (_, ctx) = ctx.next_token()?;

        Ok((Pattern::Wildcard, ctx))
    }));

    cat.leading.insert(
        "@paren".to_string(),
        Rc::new(|ctx, env| {
            let (group, ctx) = ctx.next_token()?;
            let r = parse_inside_group(group, GroupKind::Paren, |inner| {
                inner.parse(env, "Pattern", 0)
            })?;
            Ok((r, ctx))
        }),
    );

    let explicit_app: Rc<TrailingParserFn<Pattern>> = Rc::new(|ctx, env, left, bp| {
        let (arg_pat, ctx) = ctx.parse(env, "Pattern", bp)?;
        let p = match left {
            Pattern::Wildcard | Pattern::AtomOrVariable(AtomOrVariable::Atom(_)) => return Err("Unexpected pattern".into()),
            Pattern::AtomOrVariable(AtomOrVariable::Variable(v)) => {
                Pattern::Constructor(v, Box::new([ arg_pat ]))
            },
            Pattern::Constructor(f, patterns) => {
                let mut patterns = patterns.to_vec();
                patterns.push(arg_pat);
                Pattern::Constructor(f, patterns.into())
            },
        };

        Ok((p, ctx))
    });

    PATTERN_LEADING.iter().for_each(|x| {
        cat.trailing.insert((*x).into(), (100, Assoc::Left, explicit_app.clone()));
    });

    let mut env = ParserEnv {
        categories: BTreeMap::new(),
    };

    env.categories.insert("Pattern".into(), cat);
    env
}

pub fn parse_insert_like<'a>(
    ctx: ParserContext<'a>,
    env: &ParserEnv<Expr>,
)  -> ParserResult<'a, (FunctionCall, Option<Expr>)> {
    let (call, mut ctx)  = ctx.parse(env, "Expr", 0)?;
    let Expr::FunctionCall(call) = call else {
        return Err("Only function call can be inserted.".into());
    };

    let mut result = None;

    if let Some(t) = ctx.peek()
    && let TokenTree::Token(t) = t
    && t.text == "->"
    {
        (_, ctx) = ctx.next_token()?;
        let (expr, ctx0) = ctx.parse(env, "Expr", 0)?;
        ctx = ctx0;
        result = Some(expr)
    }

    Ok(((call, result), ctx))
}

pub fn parse_body<'a>(
    ctx: ParserContext<'a>,
    env: &ParserEnv<Expr>,
) -> ParserResult<'a, Body> {
    let token = ctx.peek().ok_or("Unexpected EOF")?;
    let key = match token {
        TokenTree::Token(token) => token.text,
        TokenTree::Group { .. } => return Err("Expecting body".into()),
    };

    match key {
        "union" => {
            let (_, ctx) = ctx.next_token()?;
            let (arg0, ctx)  = ctx.parse(env, "Expr", 0)?;
            let (_, ctx ) = ctx.expect("<-")?;
            let (arg1, ctx)  = ctx.parse(env, "Expr", 0)?;

            Ok((Body::Union(arg0, arg1).into(), ctx))
        }
        "insert" => {
            let (_, ctx) = ctx.next_token()?;
            let ((call, result), ctx) = parse_insert_like(ctx, env)?;
            Ok((Body::Insert(call, result).into(), ctx))
        }
        _ => Err(format!("Unknown body: {key}"))
    }
}

pub fn parse_heads<'a>(
    mut ctx: ParserContext<'a>,
    expr_env: &ParserEnv<Expr>,
    pat_env: &ParserEnv<Pattern>,
) -> ParserResult<'a, Box<[Head]>> {
    let mut heads = Vec::new();

    loop {
        let token = ctx.peek().ok_or("Unexpected EOF")?;
        let key = match token {
            TokenTree::Token(t) => Some(t.text),
            TokenTree::Group { .. } => None,
        };

        match key {
            Some("if") => {
                (_, ctx) = ctx.next_token()?;
                let (l, mctx) = ctx.parse(expr_env, "Expr", 0)?;
                let (t, mctx) = mctx.next_token()?;
                let (r, mctx) = mctx.parse(expr_env, "Expr", 0)?;
                
                let TokenTree::Token(t) = t else {
                    return Err("Expecting compare operation".into());
                };
                
                let op = match t.text {
                    "=" => Op::Equ,
                    _ => todo!()        // TODO opcode
                };
                
                heads.push(Head::Guard(op, l, r));
                ctx = mctx;
            }
            Some("leteq") => {
                (_, ctx) = ctx.next_token()?;
                let (l, mctx) = ctx.parse(expr_env, "Expr", 0)?;
                let (_, mctx) = mctx.expect("<-")?;
                let (r, mctx) = mctx.parse(expr_env, "Expr", 0)?;
                
                heads.push(Head::LetEq(l, r));
                ctx = mctx;
            }
            _ => {
                let (p, mctx) = ctx.parse(pat_env, "Pattern", 0)?;
                heads.push(Head::Pattern(p));
                ctx = mctx;
            }
        }

        if let Some(TokenTree::Token(t)) = ctx.peek()
        && t.text == "," {
            (_, ctx) = ctx.next_token()?;
            continue;
        } else {
            break;
        }
    }

    // after loop, `ctx` points to the next token after the last `Head`

    Ok((heads.into(), ctx))
}

// rule (pat...), if a = b, leteq c d => body0 ; body1 ; body2
pub fn parse_rule<'a>(
    ctx: ParserContext<'a>,
    env: &ParserEnv<Expr>,
    pat_env: &ParserEnv<Pattern>,
) -> ParserResult<'a, Rule> {
    let mut bodies = Vec::new();

    // parse head part
    let (heads, ctx) = parse_heads(ctx, env, pat_env)?;
    let (token, mut ctx) = ctx.next_token()?;
    if let TokenTree::Token(t) = token && t.text == "=>" {
        // do nothing
    } else {
        return Err("Expecting rule body".into());
    }

    // parse body part

    loop {
        let (body, mctx) = parse_body(ctx, env)?;
        ctx = mctx;
        bodies.push(body);

        if let Some(TokenTree::Token(t)) = ctx.peek()
        && t.text == ";" {
            (_, ctx) = ctx.next_token()?;
            continue;
        } else {
            break;
        }
    }

    Ok((Rule {
        heads: heads.into(),
        bodys: bodies.into(),
    }, ctx))
}

pub fn parse_command<'a>(
    ctx: ParserContext<'a>,
    expr_env: &ParserEnv<Expr>,
    pat_env: &ParserEnv<Pattern>,
) -> ParserResult<'a, Command> {
    let (token, ctx) = ctx.next_token()?;
    let key = match token {
        TokenTree::Token(t) => t.text,
        TokenTree::Group { .. } => return Err("Expecting command".into()),
    };

    match key {
        "rule" => {
            let (rule, ctx) = parse_rule(ctx, expr_env, pat_env)?;
            Ok((Command::Rule(rule), ctx))
        }
        "fact" => {
            let (expr, ctx) = ctx.parse(expr_env, "Expr", 0)?;
            let fact = expr.try_into()?;
            Ok((Command::Fact(fact), ctx))
        }
        "type" => {
            let (name, mut ctx) = ctx.expect_ident()?;
            let mut cons = Vec::new();
            
            while let Some(TokenTree::Token(t)) = ctx.peek()
            && t.text == "|" {
                (_, ctx) = ctx.next_token()?;
                let (con, mctx) = parse_type_constructor(ctx)?;
                cons.push(con);
                ctx = mctx;
            }

            let def = TypeDef(name.clone(), SumType(cons.into()));
            let def = Command::TypeDef(name, def);

            Ok((def, ctx))
        }
        "table" => {
            let (TypeConstructor(name, params), ctx) = parse_type_constructor(ctx)?;
            let (ret, ctx) = if let Some(TokenTree::Token(t)) = ctx.peek()
            && t.text == "->" {
                let (_, ctx) = ctx.next_token()?;
                let (ty, ctx) = parse_type(ctx)?;
                (Some(ty), ctx)
            } else {
                (None, ctx)
            };

            let def = TableDef(name.clone(), params, ret);
            Ok((Command::TableDef(name, def), ctx))
        }
        "query" => {
            let (heads, ctx) = parse_heads(ctx, expr_env, pat_env)?;
            Ok((Command::Query(heads), ctx))
        }
        _ => Err(format!("Unknown command: {key}")),
    }
}

pub fn parse_commands(line: &str) -> Result<Vec<Command>, String> {
    let tokens = scan_tokens(line)?;
    let token_trees = parse_brackets(&tokens).map_err(|err| format!("{err:?}"))?;

    let atom_env = Rc::new(build_atom_parser_env());
    let expr_env = build_expr_parser_env(atom_env.clone());
    let pat_env = build_pattern_parser_env(atom_env);
    let mut ctx = ParserContext(&token_trees[..]);
    let mut commands: Vec<Command> = vec![];
    while let Some(_) = ctx.peek() {
        let (r, rest) = parse_command(ctx, &expr_env, &pat_env)?;
        ctx = rest;
        commands.push(r);
    }

    if ctx.peek().is_some() {
        Err(format!("{ctx:?} not a command."))
    } else {
        Ok(commands)
    }
}

#[cfg(test)]
mod tests {
    use std::{println, string::ToString};
    use crate::min_parser::core_syntax::parse_commands;

    fn assert_commands(src: &str, expected: &[&str]) {
        let cmds = parse_commands(src).unwrap();
        assert_eq!(expected.len(), cmds.len(), "Command size doesn't match");

        for (expected, cmd) in expected.iter().zip(cmds) {
            println!("{}", cmd);
            assert_eq!(expected.to_string(), cmd.to_string());
        }
    }

    #[test]
    pub fn test_parse0() {
        let code = r"
fact path 1 2
fact 5
fact path 2 3
fact path 3 4
rule path a b, path c d, if b = c => union (path a b) <- (path c d)
rule path _ (path a 1) => union path a a <- path a a
type Nat
| zro()
| suc(Nat)
table plus(Nat, Nat) -> Nat
table relation(Nat, Nat)
query path 1 4
query path a b, if a = b";

        assert_commands(code, &[
            "fact path 1u 2u",
            "fact 5u",
            "fact path 2u 3u",
            "fact path 3u 4u",
            "rule path a b, path c d, if b = c => union (path a b) <- (path c d)",
            "rule path _ (path a 1u) => union (path a a) <- (path a a)",
            r"type Nat
| zro()
| suc(Nat)
",
            "table plus(Nat, Nat) -> Nat",
            "table relation(Nat, Nat)",
            "query path 1u 4u",
            "query path a b, if a = b",
        ]);
    }
}