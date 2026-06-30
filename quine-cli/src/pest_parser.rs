use pest::Parser;
use pest_derive::Parser;
use quine_core::{rule::Op, types::*};

use quine_frontend::syntax::{
    Atom, AtomOrVariable, Body, Command, ConstructorPattern, CostDef, Expr, ExtractMode, FunctionCall, Head,
    Pattern, Rule as SyntaxRule, Span,
};
use quine_core::related_egraph::RunMode;
use quine_frontend::{Run as SyntaxRun, RunBody};

#[derive(Parser)]
#[grammar = "../docs/grammar.pest"]
pub struct QuineParser;

fn to_name(s: &str) -> String {
    String::from(s)
}

fn to_span(s: pest::Span) -> Span {
    Span::new(s.start(), s.end())
}

fn parse_variable(pair: pest::iterators::Pair<Rule>) -> String {
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
    let span = to_span(pair.as_span());
    match pair.into_inner().next() {
        Some(inner) => match inner.as_rule() {
            Rule::constructor_pattern => {
                let cp = parse_constructor_pattern(inner);
                if cp.args.is_empty() {
                    Pattern::Variable(span, cp.name)
                } else {
                    Pattern::Constructor(span, cp)
                }
            }
            Rule::atom => Pattern::Atom(span, parse_atom(inner)),
            Rule::variable => Pattern::Variable(span, parse_variable(inner)),
            _ => Pattern::Wildcard(span),
        },
        None => Pattern::Wildcard(span),
    }
}

fn parse_constructor_pattern(pair: pest::iterators::Pair<Rule>) -> ConstructorPattern {
    let span = to_span(pair.as_span());
    let mut inners = pair.into_inner();
    let name = parse_variable(inners.next().unwrap());
    let args: Box<[Pattern]> = inners.map(parse_pattern_inner).collect();
    ConstructorPattern { name, args, span }
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
    let span = to_span(pair.as_span());
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::constructor_pattern => Head::Match(span, parse_constructor_pattern(inner)),
        Rule::leteq => {
            let mut parts = inner.into_inner();
            let var_pair = parts.next().unwrap();
            let var_span = to_span(var_pair.as_span());
            let var = parse_variable(var_pair);
            let pat = parse_pattern_inner(parts.next().unwrap());
            Head::LetEq(span, Pattern::Variable(var_span, var), pat)
        }
        Rule::guard => {
            let mut parts = inner.into_inner();
            let var = parse_variable(parts.next().unwrap());
            let op = parse_op(parts.next().unwrap());
            let aov = parse_atom_or_variable(parts.next().unwrap());
            Head::Guard(span, op, var, aov)
        }
        _ => unreachable!("unexpected head variant: {:?}", inner.as_rule()),
    }
}

fn parse_heads(pair: pest::iterators::Pair<Rule>) -> Box<[Head]> {
    pair.into_inner().map(parse_head).collect()
}

fn parse_body(pair: pest::iterators::Pair<Rule>) -> Body {
    let span = to_span(pair.as_span());
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::let_ => {
            let mut parts = inner.into_inner();
            let var = parse_variable(parts.next().unwrap());
            let call = parse_function_call(parts.next().unwrap());
            Body::Let(span, var, call)
        }
        Rule::insert => {
            let mut parts = inner.into_inner();
            let call = parse_function_call(parts.next().unwrap());
            let expr = parts.next().map(parse_expr);
            Body::Insert(span, call, expr)
        }
        Rule::union => {
            let mut parts = inner.into_inner();
            let left = parse_expr(parts.next().unwrap());
            let right = parse_expr(parts.next().unwrap());
            Body::Union(span, left, right)
        }
        _ => unreachable!("unexpected body variant: {:?}", inner.as_rule()),
    }
}

fn parse_bodies(pair: pest::iterators::Pair<Rule>) -> Box<[Body]> {
    pair.into_inner().map(parse_body).collect()
}

fn parse_merge_fn(pair: pest::iterators::Pair<Rule>) -> MergeFn {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_str() {
        "min" => MergeFn::Min,
        "max" => MergeFn::Max,
        _ => unreachable!(),
    }
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
        "str" => BaseType::Str,
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

fn parse_cost_def(pair: pest::iterators::Pair<Rule>) -> CostDef {
    let mut parts = pair.into_inner();
    let full_name = parse_variable(parts.next().unwrap());
    let cost_str = parts.next().unwrap().as_str();
    // Parse as u64 — panics on negative input (u64 rejects "-1")
    let cost: u64 = cost_str.parse().unwrap();
    // Split "TypeName.ConstructorName" on the last dot
    let (type_name, constructor) = full_name
        .rsplit_once('.')
        .map(|(t, c)| (t.to_string(), c.to_string()))
        .unwrap_or_else(|| {
            // If no dot, the full name is both type and constructor
            // (e.g., for single-constructor types where the name IS the constructor)
            (full_name.clone(), full_name)
        });
    CostDef {
        type_name,
        constructor,
        cost,
    }
}

fn parse_command(pair: pest::iterators::Pair<Rule>) -> Command {
    debug_assert!(matches!(pair.as_rule(), Rule::command));
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::load => {
            // load = { "load" ~ string }
            let string_pair = inner.into_inner().next().unwrap();
            let raw = string_pair.as_str();
            let path = &raw[1..raw.len() - 1]; // strip surrounding quotes
            Command::Load(path.into())
        }
        Rule::type_def => {
            let mut parts = inner.into_inner();
            let name = parse_variable(parts.next().unwrap());
            let constructors: Box<[TypeConstructor]> = parts.map(parse_type_constructor).collect();
            let type_def = TypeDef(name.clone(), SumType(constructors));
            Command::TypeDef(name, type_def)
        }
        Rule::relation_def => {
            let mut parts = inner.into_inner();
            let name = parse_variable(parts.next().unwrap());
            let mut types: Vec<_> = parts.map(parse_type).collect();
            types.push(Type::Base(BaseType::Id));
            Command::TableDef(name.clone(), TableDef(name, types.into(), None))
        }
        Rule::function_def => {
            let mut parts = inner.into_inner();
            let name = parse_variable(parts.next().unwrap());
            let mut types = Vec::new();
            let mut merge = None;
            for part in parts {
                match part.as_rule() {
                    Rule::r#type => types.push(parse_type(part)),
                    Rule::merge_suffix => merge = Some(parse_merge_fn(part)),
                    _ => {}
                }
            }
            Command::TableDef(name.clone(), TableDef(name, types.into(), merge))
        }
        Rule::rule => {
            let mut parts = inner.into_inner();
            let first = parts.next().unwrap();
            let (group, heads, bodys) = if first.as_rule() == Rule::string {
                let s = first.as_str();
                let group = Some(s[1..s.len() - 1].into());
                let heads = parse_heads(parts.next().unwrap());
                let bodys = parse_bodies(parts.next().unwrap());
                (group, heads, bodys)
            } else {
                let heads = parse_heads(first);
                let bodys = parse_bodies(parts.next().unwrap());
                (None, heads, bodys)
            };
            Command::Rule(SyntaxRule { group, heads, bodys })
        }
        Rule::fact => {
            let bodys = parse_bodies(inner.into_inner().next().unwrap());
            Command::Fact(bodys)
        }
        Rule::query => {
            let mut parts = inner.into_inner();
            let heads = parse_heads(parts.next().unwrap());
            let vars: Vec<_> = parts.map(|p| parse_variable(p)).collect();
            Command::Query(heads, vars)
        }
        Rule::run => {
            let run_item = inner.into_inner().next().unwrap();
            Command::Run(parse_run_item(run_item))
        }
        Rule::cost_def => Command::CostDef(parse_cost_def(inner)),
        Rule::extract_query => {
            // pest strips all literal strings ("extract", "optimal") from
            // children, so inner pairs always = [expr].  Use the full matched
            // text to decide the extract mode.
            let mode = if inner.as_str().starts_with("extract optimal") {
                ExtractMode::Optimal
            } else {
                ExtractMode::Greedy
            };
            let expr = parse_expr(inner.into_inner().next().unwrap());
            Command::Extract(expr, mode)
        }
        _ => unreachable!("unexpected command variant: {:?}", inner.as_rule()),
    }
}

fn parse_run_item(pair: pest::iterators::Pair<Rule>) -> SyntaxRun {
    let mut parts = pair.into_inner();
    let mode = parse_run_mode(parts.next().unwrap());
    let body = parts.next().map(parse_run_body).unwrap_or(RunBody::All);
    SyntaxRun(mode, body)
}

fn parse_run_mode(pair: pest::iterators::Pair<Rule>) -> RunMode {
    let mut inner = pair.into_inner();
    match inner.next() {
        Some(p) if p.as_rule() == Rule::integer => {
            RunMode::Repeat(p.as_str().parse().unwrap_or(0))
        }
        _ => RunMode::Saturate,
    }
}

fn parse_run_body(pair: pest::iterators::Pair<Rule>) -> RunBody {
    let inners: Vec<_> = pair.into_inner().collect();
    match inners.first().map(|p| p.as_rule()) {
        Some(Rule::string) => {
            let s = inners[0].as_str();
            RunBody::Group(s[1..s.len() - 1].into())
        }
        Some(Rule::run_item) => {
            let runs: Box<[SyntaxRun]> = inners.into_iter().map(parse_run_item).collect();
            RunBody::Program(runs)
        }
        _ => unreachable!(),
    }
}

pub fn parse_file(input: &str) -> Result<Vec<Command>, String> {
    let pairs: Vec<_> = QuineParser::parse(Rule::TOP_LEVEL, input)
        .map_err(|e| format!("{}", e))?
        .collect();

    let consumed = pairs.last().map_or(0, |p| p.as_span().end());
    let remaining = input[consumed..].trim();
    if !remaining.is_empty() {
        return Err(format!(
            "unexpected input at position {consumed}: {remaining}"
        ));
    }

    let commands = pairs
        .into_iter()
        .flat_map(|pair| pair.into_inner().map(parse_command))
        .collect();
    Ok(commands)
}

/// Check whether a REPL input line is syntactically incomplete (needs more
/// input to form a valid command).  Uses pest's error position: if the
/// parser failed at the end of the input it was expecting more tokens;
/// if it failed earlier there is a genuine syntax error.
pub fn is_repl_input_incomplete(input: &str) -> bool {
    use pest::error::InputLocation;

    // The REPL parses individual commands, not TOP_LEVEL.
    match QuineParser::parse(Rule::command, input) {
        Ok(_) => false,
        Err(e) => {
            let end = match e.location {
                InputLocation::Pos(p) => p,
                InputLocation::Span((_, e)) => e,
            };
            // Error position at or past end → parser ran out of input.
            end >= input.len()
        }
    }
}

pub fn parse_repl_commands(input: &str) -> Result<Vec<Command>, String> {
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
