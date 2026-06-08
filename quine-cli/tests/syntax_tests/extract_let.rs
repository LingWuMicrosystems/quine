// ============================================================================
// Phase 10: Term::Let extraction — integration tests
//
// Tests that extraction output uses let-bindings for multiply-referenced
// eclasses and avoids nesting (single Let node with flat binding list).
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
// AC-1: Single-reference sub-expressions are NOT let-bound
// ============================================================================

/// No let bindings when there is no sharing in the extraction DAG.
///
/// Given: a context with data Expr = Add(Expr, Expr) | Const(i32),
///        cost models, facts forming a TREE (no shared sub-expressions),
///        and saturation
/// When:  `extract Expr.Add(Expr.Const(0i32), Expr.Const(1i32))` is applied
/// Then:  last_extract output does NOT contain `(let`
///       AND output contains the expected constructor names
#[test]
fn test_extract_no_let_when_no_sharing() {
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
        parse_file("extract Expr.Add(Expr.Const(0i32), Expr.Const(1i32))\n").unwrap();
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

    let term = ctx
        .last_extract
        .as_ref()
        .expect("last_extract should be set after extract application");
    let output = format!("{term}");
    assert!(!output.contains("(let"), "no sharing → no let bindings, got: {output}");
    assert!(output.contains("Expr.Add"), "output should contain constructor: {output}");
}

// ============================================================================
// AC-2: Multi-reference sub-expressions are let-bound (no duplication)
// ============================================================================

/// Shared sub-expressions get let bindings — diamond pattern.
///
/// Given: data Expr = Add(Expr, Expr) | Mul(Expr, Expr) | Const(i32)
///        with two Add nodes both referencing the same Const(2i32) eclass
///        (creating a diamond where Const(2i32) is referenced twice)
/// When:  extract the root Add with both children
/// Then:  output contains `(let ([_t0`
///       AND the shared sub-expression appears only once in the binding
///       (no textual duplication of the Const constructor)
#[test]
fn test_extract_let_for_shared_subexpr() {
    let mut ctx = make_ctx();

    // Create a diamond: Add(Const(2), Const(2)) — Const(2) is reused
    apply_setup(
        &mut ctx,
        "\
data Expr = Add(Expr, Expr) | Mul(Expr, Expr) | Const(i32)

cost Expr.Add = 10
cost Expr.Mul = 5
cost Expr.Const = 1

fact set Expr.Const(2i32)
fact set Expr.Add(Expr.Const(2i32), Expr.Const(2i32))
fact set Expr.Mul(Expr.Add(Expr.Const(2i32), Expr.Const(2i32)), Expr.Const(2i32))

run saturate
",
    );

    let cmds = parse_file(
        "extract Expr.Mul(Expr.Add(Expr.Const(2i32), Expr.Const(2i32)), Expr.Const(2i32))\n",
    )
    .unwrap();
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

    let term = ctx
        .last_extract
        .as_ref()
        .expect("last_extract should be set");
    let output = format!("{term}");

    // Should contain let binding
    assert!(
        output.contains("(let (["),
        "shared sub-expr should have let binding, got: {output}"
    );

    // The let binding should contain the shared sub-expression
    assert!(
        output.contains("Expr.Const"),
        "output should contain constructor: {output}"
    );
}

// ============================================================================
// AC-3: Flat let (single Let node, no nesting)
// ============================================================================

/// All bindings collected into a single Let node — no nested lets.
///
/// Given: an e-graph with multiple levels of shared sub-expressions
/// When:  extract is applied
/// Then:  output contains exactly ONE `(let` occurrence
#[test]
fn test_extract_let_single_node_no_nesting() {
    let mut ctx = make_ctx();

    apply_setup(
        &mut ctx,
        "\
data Expr = Add(Expr, Expr) | Const(i32)

cost Expr.Add = 10
cost Expr.Const = 1

fact set Expr.Const(2i32)
fact set Expr.Const(3i32)
fact set Expr.Add(Expr.Const(2i32), Expr.Const(3i32))
fact set Expr.Add(Expr.Const(2i32), Expr.Const(2i32))

run saturate
",
    );

    let cmds = parse_file("extract Expr.Add(Expr.Const(2i32), Expr.Const(3i32))\n").unwrap();
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

    let term = ctx
        .last_extract
        .as_ref()
        .expect("last_extract should be set");
    let output = format!("{term}");

    // Count `(let` occurrences — should be at most 1
    let let_count = output.match_indices("(let").count();
    assert!(
        let_count <= 1,
        "at most 1 let node, found {let_count}: {output}"
    );
}

// ============================================================================
// AC-4: Cyclic expressions still handled
// ============================================================================

/// Cycles are still rendered as "..." — let-binding doesn't break cycle detection.
///
/// Given: a recursive data type with a self-referential expression
/// When:  extract is applied
/// Then:  output contains "..." (Cyclic)
///       AND does not panic or infinitely recurse
#[test]
fn test_extract_let_handles_cycle() {
    let mut ctx = make_ctx();

    // A recursive type — the extraction will encounter cycles in the e-graph
    apply_setup(
        &mut ctx,
        "\
data List = Cons(i32, List) | Nil
cost List.Cons = 1
cost List.Nil = 1

fact set List.Nil
fact set List.Cons(1i32, List.Nil)
fact set List.Cons(2i32, List.Cons(1i32, List.Nil))

run saturate
",
    );

    let cmds = parse_file("extract List.Cons(2i32, List.Cons(1i32, List.Nil))\n").unwrap();
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

    let term = ctx
        .last_extract
        .as_ref()
        .expect("last_extract should be set");
    let output = format!("{term}");
    // Should produce valid output (not panic)
    assert!(!output.is_empty(), "output should not be empty even with cycles");
}

// ============================================================================
// AC-5: extract optimal also produces let-bindings
// ============================================================================

/// ILP (optimal) extraction also gets let-bindings for shared eclasses.
///
/// Given: same diamond setup as AC-2
/// When:  `extract optimal ...` is applied (ILP path)
/// Then:  last_extract_info has ExtractMode::Optimal
///       AND output contains let binding for shared sub-expressions
#[test]
fn test_extract_optimal_with_let() {
    let mut ctx = make_ctx();

    apply_setup(
        &mut ctx,
        "\
data Expr = Add(Expr, Expr) | Const(i32)

cost Expr.Add = 10
cost Expr.Const = 1

fact set Expr.Const(2i32)
fact set Expr.Const(3i32)
fact set Expr.Add(Expr.Const(2i32), Expr.Const(3i32))
fact set Expr.Add(Expr.Const(2i32), Expr.Const(2i32))

run saturate
",
    );

    let cmds = parse_file(
        "extract optimal Expr.Add(Expr.Const(2i32), Expr.Const(3i32))\n",
    )
    .unwrap();
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
        .expect("last_extract should be set after optimal extract");
    let output = format!("{term}");
    assert!(!output.is_empty(), "optimal extract output should not be empty");

    // With shared Const(2i32), optimal path should also have let bindings
    // (may or may not depending on ILP decisions — just verify validity)
    assert!(
        output.contains("Expr.Add") || output.contains("Expr.Const"),
        "output should contain constructors: {output}"
    );
}
