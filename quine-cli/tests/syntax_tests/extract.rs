use quine::pest_parser::parse_file;
use quine_frontend::compile::compile_command;
use quine_frontend::prelude::register_prelude;
use quine_frontend::syntax::Command;
use quine_frontend::syntax::{Atom, AtomOrVariable};
use quine_frontend::{CompiledUnit, EngineContext};

fn make_ctx() -> EngineContext {
    let mut ctx = EngineContext::default();
    register_prelude(&mut ctx);
    ctx
}

// ============================================================================
// AC-1: Parse `extract <expr>` with nested constructor calls
// ============================================================================

/// Parse an extract with nested constructor calls (no pattern variables).
///
/// Given: `extract Expr.Add(Expr.Const(0i32), Expr.Const(4i32))`
/// When:  parsed
/// Then:  Command::Extract with a FunctionCall tree
#[test]
fn ac1_extract_nested_expr_parses() {
    let input = "extract Expr.Add(Expr.Const(0i32), Expr.Const(4i32))\n";
    let commands = parse_file(input).unwrap();
    assert_eq!(commands.len(), 1);
    match &commands[0] {
        Command::Extract(expr, _) => {
            // Verify it's a FunctionCall with two args
            match expr {
                quine_frontend::syntax::Expr::FunctionCall(call) => {
                    assert_eq!(call.0, "Expr.Add");
                    assert_eq!(call.1.len(), 2);
                }
                other => panic!("expected FunctionCall, got {:?}", other),
            }
        }
        other => panic!("expected Extract, got {:?}", other),
    }
}

// ============================================================================
// AC-2: Old syntax `extract <pattern> print(<vars>)` is rejected
// ============================================================================

/// Old pattern+print syntax is no longer valid.
///
/// Given: `extract expr(x) print(x)`
/// When:  parsed
/// Then:  parse_file returns an Err
#[test]
fn ac2_old_syntax_rejected() {
    let input = "extract expr(x) print(x)\n";
    let result = parse_file(input);
    assert!(result.is_err(), "old syntax should be rejected");
}

// ============================================================================
// AC-3: Atom literal in extract
// ============================================================================

/// Parse an extract with a literal atom.
///
/// Given: `extract 42i32`
/// When:  parsed
/// Then:  Command::Extract with AtomOrVariable::Atom(I32(42))
#[test]
fn ac3_extract_atom_literal() {
    let input = "extract 42i32\n";
    let commands = parse_file(input).unwrap();
    assert_eq!(commands.len(), 1);
    match &commands[0] {
        Command::Extract(expr, _) => {
            match expr {
                quine_frontend::syntax::Expr::AtomOrVariable(AtomOrVariable::Atom(Atom::I32(42))) => {}
                other => panic!("expected AtomOrVariable::Atom(I32(42)), got {:?}", other),
            }
        }
        other => panic!("expected Extract, got {:?}", other),
    }
}

// ============================================================================
// AC-4: Display round-trip for Extract
// ============================================================================

/// Extract Display output uses the s-expression format.
///
/// Given: a parsed Extract command with a nested expr
/// When:  Display::fmt is called
/// Then:  output starts with "extract " and contains the constructor names
#[test]
fn ac4_extract_display_consistent() {
    let input = "extract Expr.Add(Expr.Const(0i32), Expr.Const(4i32))\n";
    let commands = parse_file(input).unwrap();
    let display = format!("{}", commands[0]);
    // Display uses s-expression format (space-separated, paren-wrapped)
    assert!(display.starts_with("extract "));
    assert!(display.contains("Expr.Add"));
    assert!(display.contains("Expr.Const"));
}

// ============================================================================
// AC-5: Extract compiles to CompiledUnit::Extract
// ============================================================================

/// Full parse -> compile produces CompiledUnit::Extract with an Expr.
///
/// Given: data Expr = Add(x, y) | Const(v) and extract Expr.Add(Expr.Const(0i32), Expr.Const(4i32))
/// When:  parsed and compiled
/// Then:  CompiledUnit::Extract with correct Expr
#[test]
fn ac5_extract_compiles() {
    let input = "data Expr = Add(x, y) | Const(v)\nextract Expr.Add(Expr.Const(0i32), Expr.Const(4i32))\n";
    let commands = parse_file(input).unwrap();
    assert_eq!(commands.len(), 2);

    let mut ctx = make_ctx();
    // Apply type def
    let unit = compile_command(
        &commands[0],
        &mut ctx.data_types,
        &mut ctx.table_types,
        &mut ctx.interner,
        &ctx.native_names,
        &ctx.native_signatures,
    )
    .unwrap();
    ctx.apply(unit);

    // Compile extract
    let unit = compile_command(
        &commands[1],
        &mut ctx.data_types,
        &mut ctx.table_types,
        &mut ctx.interner,
        &ctx.native_names,
        &ctx.native_signatures,
    )
    .unwrap();

    match unit {
        CompiledUnit::Extract(expr, _) => {
            // Verify it's the expected Expr structure
            match expr {
                quine_frontend::syntax::Expr::FunctionCall(call) => {
                    assert_eq!(call.0, "Expr.Add");
                    assert_eq!(call.1.len(), 2);
                }
                other => panic!("expected FunctionCall, got {:?}", other),
            }
        }
        other => panic!("expected CompiledUnit::Extract, got {:?}", other),
    }
}

// ============================================================================
// AC-6: Bare variable name in extract
// ============================================================================

/// Parse extract with a bare variable name (no parens, no args).
///
/// Given: `extract x`
/// When:  parsed
/// Then:  Command::Extract with AtomOrVariable::Variable("x")
#[test]
fn ac6_extract_variable_name() {
    let input = "extract x\n";
    let commands = parse_file(input).unwrap();
    assert_eq!(commands.len(), 1);
    match &commands[0] {
        Command::Extract(expr, _) => {
            match expr {
                quine_frontend::syntax::Expr::AtomOrVariable(AtomOrVariable::Variable(v)) => {
                    assert_eq!(v, "x");
                }
                other => panic!("expected Variable(\"x\"), got {:?}", other),
            }
        }
        other => panic!("expected Extract, got {:?}", other),
    }
}
