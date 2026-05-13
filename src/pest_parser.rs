use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use pest::Parser;
use pest_derive::Parser;

use crate::engine::frontend::syntax::{
    AtomOrVariable, Body, Command, ConstructorPattern, Expr, FunctionCall, Head, Pattern,
    Rule as SyntaxRule,
};
use crate::regraph::common::{Atom, Name, TypeName};
use crate::regraph::rule::Op;
use crate::regraph::types::{BaseType, SumType, TableDef, Type, TypeConstructor, TypeDef};

#[derive(Parser)]
#[grammar = "../docs/grammar.pest"]
pub struct QuineParser;

fn to_name(s: &str) -> Name {
    String::from(s)
}

fn parse_variable(pair: pest::iterators::Pair<Rule>) -> Name {
    to_name(pair.as_str())
}

fn parse_atom(pair: pest::iterators::Pair<Rule>) -> Atom {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::boolean => Atom::Bool(inner.as_str() == "true"),
        Rule::int8 => Atom::I8(inner.as_str().trim_end_matches("i8").parse().unwrap()),
        Rule::int16 => Atom::I16(inner.as_str().trim_end_matches("i16").parse().unwrap()),
        Rule::int32 => Atom::I32(inner.as_str().trim_end_matches("i32").parse().unwrap()),
        Rule::int64 => Atom::I64(inner.as_str().trim_end_matches("i64").parse().unwrap()),
        Rule::uint8 => Atom::U8(inner.as_str().trim_end_matches("u8").parse().unwrap()),
        Rule::uint16 => Atom::U16(inner.as_str().trim_end_matches("u16").parse().unwrap()),
        Rule::uint32 => Atom::U32(inner.as_str().trim_end_matches("u32").parse().unwrap()),
        Rule::uint64 => Atom::U64(inner.as_str().trim_end_matches("u64").parse().unwrap()),
        Rule::string => {
            let mut inners = inner.clone().into_inner();
            if let Some(content) = inners.next() {
                Atom::Str(to_name(content.as_str()))
            } else {
                let s = inner.as_str();
                Atom::Str(to_name(&s[1..s.len() - 1]))
            }
        }
        _ => unreachable!("unexpected atom variant: {:?}", inner.as_rule()),
    }
}

fn parse_op(pair: pest::iterators::Pair<Rule>) -> Op {
    match pair.as_str() {
        "==" => Op::Equ,
        "!=" => Op::Neq,
        "<" => Op::Lt,
        ">" => Op::Gt,
        "<=" => Op::Leq,
        ">=" => Op::Geq,
        _ => unreachable!("unexpected op: {}", pair.as_str()),
    }
}

fn parse_atom_or_variable(pair: pest::iterators::Pair<Rule>) -> AtomOrVariable {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::atom => AtomOrVariable::Atom(parse_atom(inner)),
        Rule::variable => AtomOrVariable::Variable(parse_variable(inner)),
        _ => unreachable!("unexpected atom_or_variable variant: {:?}", inner.as_rule()),
    }
}

fn parse_pattern_inner(pair: pest::iterators::Pair<Rule>) -> Pattern {
    match pair.into_inner().next() {
        Some(inner) => match inner.as_rule() {
            Rule::constructor_pattern => {
                let cp = parse_constructor_pattern(inner);
                if cp.1.is_empty() {
                    Pattern::Variable(cp.0)
                } else {
                    Pattern::Constructor(cp)
                }
            }
            Rule::atom => Pattern::Atom(parse_atom(inner)),
            Rule::variable => Pattern::Variable(parse_variable(inner)),
            _ => Pattern::Wildcard,
        },
        None => Pattern::Wildcard,
    }
}

fn parse_constructor_pattern(pair: pest::iterators::Pair<Rule>) -> ConstructorPattern {
    let mut inners = pair.into_inner();
    let name = parse_variable(inners.next().unwrap());
    let args: Box<[Pattern]> = inners.map(parse_pattern_inner).collect();
    ConstructorPattern(name, args)
}

fn parse_expr(pair: pest::iterators::Pair<Rule>) -> Expr {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::atom_or_variable => Expr::AtomOrVariable(parse_atom_or_variable(inner)),
        Rule::function_call => Expr::FunctionCall(parse_function_call(inner)),
        _ => unreachable!("unexpected expr variant: {:?}", inner.as_rule()),
    }
}

fn parse_function_call(pair: pest::iterators::Pair<Rule>) -> FunctionCall {
    let mut inners = pair.into_inner();
    let name = parse_variable(inners.next().unwrap());
    let args: Box<[Expr]> = inners.map(parse_expr).collect();
    FunctionCall(name, args)
}

fn parse_head(pair: pest::iterators::Pair<Rule>) -> Head {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::constructor_pattern => Head::Match(parse_constructor_pattern(inner)),
        Rule::leteq => {
            let mut parts = inner.into_inner();
            let var = parse_variable(parts.next().unwrap());
            let pat = parse_pattern_inner(parts.next().unwrap());
            Head::LetEq(Pattern::Variable(var), pat)
        }
        Rule::guard => {
            let mut parts = inner.into_inner();
            let var = parse_variable(parts.next().unwrap());
            let op = parse_op(parts.next().unwrap());
            let aov = parse_atom_or_variable(parts.next().unwrap());
            Head::Guard(op, var, aov)
        }
        _ => unreachable!("unexpected head variant: {:?}", inner.as_rule()),
    }
}

fn parse_heads(pair: pest::iterators::Pair<Rule>) -> Box<[Head]> {
    pair.into_inner().map(parse_head).collect()
}

fn parse_body(pair: pest::iterators::Pair<Rule>) -> Body {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::let_ => {
            let mut parts = inner.into_inner();
            let var = parse_variable(parts.next().unwrap());
            let call = parse_function_call(parts.next().unwrap());
            Body::Let(var, call)
        }
        Rule::insert => {
            let call = parse_function_call(inner.into_inner().next().unwrap());
            Body::Insert(call, None)
        }
        Rule::union => {
            let mut parts = inner.into_inner();
            let left = parse_expr(parts.next().unwrap());
            let right = parse_expr(parts.next().unwrap());
            Body::Union(left, right)
        }
        _ => unreachable!("unexpected body variant: {:?}", inner.as_rule()),
    }
}

fn parse_bodies(pair: pest::iterators::Pair<Rule>) -> Box<[Body]> {
    pair.into_inner().map(parse_body).collect()
}

fn parse_base_type(pair: pest::iterators::Pair<Rule>) -> BaseType {
    match pair.as_str() {
        "bool" => BaseType::I1,
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
        "string" => BaseType::Str,
        _ => unreachable!("unexpected base type: {}", pair.as_str()),
    }
}

fn parse_type(pair: pest::iterators::Pair<Rule>) -> Type {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::base_type => Type::Base(parse_base_type(inner)),
        Rule::variable => Type::Name(parse_variable(inner)),
        _ => unreachable!("unexpected type variant: {:?}", inner.as_rule()),
    }
}

fn parse_type_constructor(pair: pest::iterators::Pair<Rule>) -> TypeConstructor {
    let mut inners = pair.into_inner();
    let name = parse_variable(inners.next().unwrap());
    let types: Box<[Type]> = inners.map(parse_type).collect();
    TypeConstructor(name, types)
}

fn parse_command(pair: pest::iterators::Pair<Rule>) -> Command {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::type_def => {
            let mut parts = inner.into_inner();
            let name = parse_variable(parts.next().unwrap());
            let constructors: Box<[TypeConstructor]> = parts.map(parse_type_constructor).collect();
            let type_def = TypeDef(name.clone(), SumType(constructors));
            Command::TypeDef(TypeName(name), type_def)
        }
        Rule::relation_def => {
            let mut parts = inner.into_inner();
            let name = parse_variable(parts.next().unwrap());
            let types: Box<[Type]> = parts.map(parse_type).collect();
            Command::TableDef(name.clone(), TableDef(name, types, None))
        }
        Rule::function_def => {
            let mut parts = inner.into_inner();
            let name = parse_variable(parts.next().unwrap());
            let mut types = Vec::new();
            for part in parts {
                if part.as_rule() == Rule::r#type {
                    types.push(parse_type(part));
                }
            }
            let ret = types.pop();
            Command::TableDef(name.clone(), TableDef(name, types.into(), ret))
        }
        Rule::rule => {
            let mut parts = inner.into_inner();
            let heads = parse_heads(parts.next().unwrap());
            let bodys = parse_bodies(parts.next().unwrap());
            Command::Rule(SyntaxRule { heads, bodys })
        }
        Rule::fact => {
            let bodie = parse_bodies(inner.into_inner().next().unwrap());
            Command::Fact(bodie)
        }
        Rule::query => {
            let heads = parse_heads(inner.into_inner().next().unwrap());
            Command::Query(heads)
        }
        _ => unreachable!("unexpected command variant: {:?}", inner.as_rule()),
    }
}

pub fn parse_commands(input: &str) -> Result<Vec<Command>, String> {
    let mut commands = Vec::new();
    let mut pos = 0;

    while pos < input.len() {
        let remaining = &input[pos..];
        if remaining.trim().is_empty() {
            break;
        }

        let pairs = QuineParser::parse(Rule::command, remaining).map_err(|e| format!("{}", e))?;

        for pair in pairs {
            let end = pair.as_span().end();
            commands.push(parse_command(pair));
            pos += end;
        }
    }

    Ok(commands)
}
