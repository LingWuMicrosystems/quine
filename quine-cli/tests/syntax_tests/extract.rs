use quine::pest_parser::parse_file;
use quine_frontend::compile::compile_command;
use quine_frontend::prelude::register_prelude;
use quine_frontend::syntax::Command;
use quine_frontend::{CompiledUnit, EngineContext};

fn make_ctx() -> EngineContext {
    let mut ctx = EngineContext::default();
    register_prelude(&mut ctx);
    ctx
}

// ============================================================================
// AC-1: Simple extract parses
// ============================================================================

/// Parse a simple extract query.
///
/// Given: `extract expr(x) print(x)`
/// When:  parsed
/// Then:  Command::Extract with 1 head and 1 print var
#[test]
fn ac1_extract_simple_parses() {
    let input = "extract expr(x) print(x)\n";
    let commands = parse_file(input).unwrap();
    assert_eq!(commands.len(), 1);
    match &commands[0] {
        Command::Extract(heads, vars) => {
            assert_eq!(heads.len(), 1);
            assert_eq!(vars.len(), 1);
            assert_eq!(&vars[0], "x");
        }
        other => panic!("expected Extract, got {:?}", other),
    }
}

// ============================================================================
// AC-2: Extract with guard parses
// ============================================================================

/// Parse extract with guard and multiple print vars.
///
/// Given: `extract path(x, y), if x > 0i32 print(x, y)`
/// When:  parsed
/// Then:  Command::Extract with 2 heads and 2 print vars
#[test]
fn ac2_extract_with_guard_parses() {
    let input = "extract path(x, y), if x > 0i32 print(x, y)\n";
    let commands = parse_file(input).unwrap();
    assert_eq!(commands.len(), 1);
    match &commands[0] {
        Command::Extract(heads, vars) => {
            assert_eq!(heads.len(), 2);
            assert_eq!(vars.len(), 2);
            assert_eq!(&vars[0], "x");
            assert_eq!(&vars[1], "y");
        }
        other => panic!("expected Extract, got {:?}", other),
    }
}

// ============================================================================
// AC-3: Extract with leteq parses
// ============================================================================

/// Parse extract with leteq unification.
///
/// Given: `extract node(x), leteq x = y print(y)`
/// When:  parsed
/// Then:  Command::Extract with 2 heads
#[test]
fn ac3_extract_with_leteq_parses() {
    let input = "extract node(x), leteq x = y print(y)\n";
    let commands = parse_file(input).unwrap();
    assert_eq!(commands.len(), 1);
    match &commands[0] {
        Command::Extract(heads, vars) => {
            assert_eq!(heads.len(), 2);
            assert_eq!(vars.len(), 1);
        }
        other => panic!("expected Extract, got {:?}", other),
    }
}

// ============================================================================
// AC-4: Extract with no print vars
// ============================================================================

/// Empty print() is rejected — grammar requires at least one variable.
///
/// Given: `extract expr(x) print()`
/// When:  parsed
/// Then:  parse_file returns an Err
#[test]
fn ac4_extract_empty_print_rejected() {
    let input = "extract expr(x) print()\n";
    let result = parse_file(input);
    assert!(result.is_err(), "empty print() should be rejected");
}

// ============================================================================
// AC-5: Display round-trip for Extract
// ============================================================================

/// Extract Display output is consistent.
///
/// Given: a parsed Extract command
/// When:  Display::fmt is called
/// Then:  output follows the same format as query (constructor args space-separated)
#[test]
fn ac5_extract_display_consistent() {
    let input = "extract expr(x) print(x)\n";
    let commands = parse_file(input).unwrap();
    let display = format!("{}", commands[0]);
    // Display uses space-separated args (no parens), matching query's Display format
    assert!(display.starts_with("extract "));
    assert!(display.contains("print(x)"));
    assert!(display.contains("expr"));
}

// ============================================================================
// AC-6: Extract compiles to CompiledUnit::Extract
// ============================================================================

/// Full parse -> compile produces CompiledUnit::Extract.
///
/// Given: data Expr = Add(x, y) | Const(v) and extract Expr.Add(x, y) print(x, y)
/// When:  parsed and compiled
/// Then:  CompiledUnit::Extract with correct query and vars
#[test]
fn ac6_extract_compiles() {
    let input = "data Expr = Add(x, y) | Const(v)\nextract Expr.Add(x, y) print(x, y)\n";
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
        CompiledUnit::Extract(query, vars) => {
            assert_eq!(vars.len(), 2);
            assert_eq!(vars[0], "x");
            assert_eq!(vars[1], "y");
            assert_eq!(query.scan_steps.len(), 1);
        }
        other => panic!("expected CompiledUnit::Extract, got {:?}", other),
    }
}
