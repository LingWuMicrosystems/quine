// ============================================================================
// AC-3 & AC-4: Integration tests for extract optimal / extract greedy paths
//
// Tests the full pipeline: parse → compile → apply → extract result.
// Covers both ExtractMode::Optimal (ILP solver) and ExtractMode::Greedy
// (backward-compatible materialize_cheapest).
// ============================================================================

use quine::pest_parser::parse_file;
use quine_frontend::compile::compile_command;
use quine_frontend::prelude::register_prelude;
use quine_frontend::syntax::{Command as QuineCommand, ExtractMode};
use quine_frontend::{CompiledUnit, EngineContext};

fn make_ctx() -> EngineContext {
    let mut ctx = EngineContext::default();
    register_prelude(&mut ctx);
    ctx
}

/// Parse, compile, and apply all non-Extract/Query commands as setup.
fn apply_setup(ctx: &mut EngineContext, source: &str) {
    let commands = parse_file(source).unwrap();
    for cmd in &commands {
        match cmd {
            QuineCommand::Query(..) | QuineCommand::Extract(..) => {}
            _ => {
                let unit = compile_command(
                    cmd,
                    &mut ctx.data_types,
                    &mut ctx.table_types,
                    &mut ctx.interner,
                    &ctx.native_names,
                    &ctx.native_signatures,
                )
                .unwrap();
                ctx.apply(unit);
            }
        }
    }
}

// ============================================================================
// AC-3: extract optimal produces valid output (ILP solver path)
// ============================================================================

/// Full pipeline: extract optimal → ctx.last_extract has Term via greedy path
/// and last_extract_info confirms ExtractMode::Optimal routing.
///
/// Given: a context with data Expr = Add(Expr, Expr) | Const(i32), cost models,
///        facts, and saturation
/// When:  `extract optimal Expr.Add(Expr.Const(0i32), Expr.Const(1i32))`
///        is parsed, compiled, and applied
/// Then:  last_extract_info has ExtractMode::Optimal
///       AND last_extract has a valid Term (non-empty Display output)
#[test]
fn test_extract_optimal_produces_valid_output() {
    let mut ctx = make_ctx();

    apply_setup(
        &mut ctx,
        "\
data Expr = Add(Expr, Expr) | Const(i32)

cost Expr.Add = 10
cost Expr.Const = 1

fact set Expr.Const(0i32)
fact set Expr.Const(1i32)
fact set Expr.Add(Expr.Const(0i32), Expr.Const(1i32))

run saturate
",
    );

    let cmds =
        parse_file("extract optimal Expr.Add(Expr.Const(0i32), Expr.Const(1i32))\n").unwrap();
    let cmd = &cmds[0];
    let unit = compile_command(
        cmd,
        &mut ctx.data_types,
        &mut ctx.table_types,
        &mut ctx.interner,
        &ctx.native_names,
        &ctx.native_signatures,
    )
    .unwrap();

    match &unit {
        CompiledUnit::Extract(_, mode) => {
            assert!(
                matches!(mode, ExtractMode::Optimal),
                "expected ExtractMode::Optimal"
            );
        }
        other => panic!("expected CompiledUnit::Extract, got {:?}", other),
    }

    ctx.apply(unit);

    let term = ctx
        .last_extract
        .as_ref()
        .expect("last_extract should be set after extract application");
    let output = format!("{term}");
    assert!(!output.is_empty(), "extracted term output should not be empty");
    assert!(
        output.contains("Expr.Add") || output.contains("Expr.Const"),
        "output should contain constructor name, got: {}",
        output
    );
}

// ============================================================================
// AC-4: extract (greedy) backward compatibility
// ============================================================================

/// Greedy extraction path still works (no "optimal" keyword).
///
/// Given: same setup as optimal test
/// When:  `extract Expr.Add(Expr.Const(0i32), Expr.Const(1i32))` (no "optimal")
///        is parsed, compiled, and applied
/// Then:  last_extract_info has ExtractMode::Greedy
///       AND last_extract has a valid Term
#[test]
fn test_extract_greedy_backward_compatible() {
    let mut ctx = make_ctx();

    apply_setup(
        &mut ctx,
        "\
data Expr = Add(Expr, Expr) | Const(i32)

cost Expr.Add = 10
cost Expr.Const = 1

fact set Expr.Const(0i32)
fact set Expr.Const(1i32)
fact set Expr.Add(Expr.Const(0i32), Expr.Const(1i32))

run saturate
",
    );

    let cmds = parse_file("extract Expr.Add(Expr.Const(0i32), Expr.Const(1i32))\n").unwrap();
    let cmd = &cmds[0];
    let unit = compile_command(
        cmd,
        &mut ctx.data_types,
        &mut ctx.table_types,
        &mut ctx.interner,
        &ctx.native_names,
        &ctx.native_signatures,
    )
    .unwrap();

    match &unit {
        CompiledUnit::Extract(_, mode) => {
            assert!(
                matches!(mode, ExtractMode::Greedy),
                "expected ExtractMode::Greedy, got {:?}",
                mode
            );
        }
        other => panic!("expected CompiledUnit::Extract, got {:?}", other),
    }

    ctx.apply(unit);

    let term = ctx
        .last_extract
        .as_ref()
        .expect("last_extract should be set");
    let output = format!("{term}");
    assert!(!output.is_empty(), "extracted term output should not be empty");
}

// ============================================================================
// AC-3 (extended): extract optimal compiles to ExtractMode::Optimal
// ============================================================================

/// extract optimal uses the ILP solver mode — verified via last_extract_info.
///
/// Given: a context with data Expr = Const(i32), cost, and a fact
/// When:  extract optimal Expr.Const(42i32) is compiled and applied
/// Then:  last_extract_info contains (expr, ExtractMode::Optimal)
#[test]
fn test_extract_optimal_mode_in_context() {
    let mut ctx = make_ctx();

    apply_setup(
        &mut ctx,
        "\
data Expr = Const(i32)
cost Expr.Const = 1
fact set Expr.Const(42i32)
run saturate
",
    );

    let cmds = parse_file("extract optimal Expr.Const(42i32)\n").unwrap();
    let cmd = &cmds[0];
    let unit = compile_command(
        cmd,
        &mut ctx.data_types,
        &mut ctx.table_types,
        &mut ctx.interner,
        &ctx.native_names,
        &ctx.native_signatures,
    )
    .unwrap();
    ctx.apply(unit);

    let (_, mode) = ctx
        .last_extract_info
        .take()
        .expect("last_extract_info should be set for Extract command");
    assert!(
        matches!(mode, ExtractMode::Optimal),
        "expected ExtractMode::Optimal, got {:?}",
        mode
    );
}

// ============================================================================
// AC-4 (extended): extract greedy sets ExtractMode::Greedy
// ============================================================================

/// extract (no "optimal") sets ExtractMode::Greedy.
///
/// Given: same setup as optimal mode test
/// When:  extract Expr.Const(42i32) (without "optimal") is compiled and applied
/// Then:  last_extract_info contains (expr, ExtractMode::Greedy)
#[test]
fn test_extract_greedy_mode_in_context() {
    let mut ctx = make_ctx();

    apply_setup(
        &mut ctx,
        "\
data Expr = Const(i32)
cost Expr.Const = 1
fact set Expr.Const(42i32)
run saturate
",
    );

    let cmds = parse_file("extract Expr.Const(42i32)\n").unwrap();
    let cmd = &cmds[0];
    let unit = compile_command(
        cmd,
        &mut ctx.data_types,
        &mut ctx.table_types,
        &mut ctx.interner,
        &ctx.native_names,
        &ctx.native_signatures,
    )
    .unwrap();
    ctx.apply(unit);

    let (_, mode) = ctx
        .last_extract_info
        .take()
        .expect("last_extract_info should be set for Extract command");
    assert!(
        matches!(mode, ExtractMode::Greedy),
        "expected ExtractMode::Greedy, got {:?}",
        mode
    );
}
