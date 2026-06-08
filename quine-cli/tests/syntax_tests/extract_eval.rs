use quine::pest_parser::parse_file;
use quine_frontend::compile::compile_command;
use quine_frontend::prelude::register_prelude;
use quine_frontend::syntax::{Atom, AtomOrVariable, Command, Expr, FunctionCall};
use quine_frontend::EngineContext;
use quine_core::common::Value;
use quine_core::table::Row;

fn make_ctx() -> EngineContext {
    let mut ctx = EngineContext::default();
    register_prelude(&mut ctx);
    ctx
}

fn row(v: Vec<Value>) -> Row {
    Row(v.into())
}

// ============================================================================
// Helper: parse, compile, and apply all non-extract/query commands
// ============================================================================

fn setup_ctx(ctx: &mut EngineContext, source: &str) {
    let commands = parse_file(source).unwrap();
    for cmd in &commands {
        match cmd {
            Command::Query(..) | Command::Extract(..) => {}
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

/// Compile and apply an extract command, return result Term as string.
fn do_extract(ctx: &mut EngineContext, source: &str) -> String {
    let commands = parse_file(source).unwrap();
    let cmd = commands.iter().find(|c| matches!(c, Command::Extract(..))).unwrap();
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
    ctx.last_extract.as_ref().map(|t| format!("{t}")).unwrap_or_default()
}

// ============================================================================
// AC-1: Extract evaluates simple constructor expression
// ============================================================================

#[test]
fn test_extract_simple_constructor() {
    let mut ctx = make_ctx();

    let setup = "\
data Option = Some(i32) | None
cost Option.Some = 1
fact set Option.Some(42i32)
run saturate
";
    setup_ctx(&mut ctx, setup);

    let result = do_extract(&mut ctx, "extract Option.Some(42i32)\n");
    assert_eq!(result, "(Option.Some 42)");
}

// ============================================================================
// AC-2: Cost-aware extraction returns cheapest equivalent
// ============================================================================

#[test]
fn test_extract_cost_aware() {
    let mut ctx = make_ctx();

    let setup = "\
data T = A(i32) | B(i32)
cost T.A = 10
cost T.B = 5
fact set T.A(1i32); set T.B(1i32); union T.A(1i32) with T.B(1i32)
run saturate
";
    setup_ctx(&mut ctx, setup);

    // Extract should return the cheaper T.B form
    let result = do_extract(&mut ctx, "extract T.A(1i32)\n");
    assert_eq!(result, "(T.B 1)", "expected cheaper T.B form, got: {result}");
}

// ============================================================================
// AC-3: Extract with nested constructors resolves recursively
// ============================================================================

/// Nested constructor expression `Add(Const(1), Const(2))` is resolved
/// by recursively evaluating inner expressions and using their results
/// as keys for the outer lookup.
#[test]
fn test_extract_nested() {
    let mut ctx = make_ctx();

    // Define types via DSL (creates the tables)
    let setup = "\
data Expr = Add(i32, i32) | Const(i32)
cost Expr.Add = 1
cost Expr.Const = 3
";
    setup_ctx(&mut ctx, setup);

    // Insert Const(1) and Const(2) directly (bypass DSL type checking
    // since i32 columns accept raw Values)
    let const_table_id = *ctx.table_types.name_map.get("Expr.Const").unwrap();
    let v1 = ctx.regraph.fresh_id();
    ctx.regraph.insert(const_table_id, row(vec![Value::encode_i32(1)]), v1);
    let v2 = ctx.regraph.fresh_id();
    ctx.regraph.insert(const_table_id, row(vec![Value::encode_i32(2)]), v2);
    ctx.regraph.rebuild();

    // Get canonical eclasses for Const(1) and Const(2)
    let c1_canon = ctx.regraph.find(v1);
    let c2_canon = ctx.regraph.find(v2);

    // Insert Add(Const(1), Const(2)) — key is [c1_eclass, c2_eclass]
    let add_table_id = *ctx.table_types.name_map.get("Expr.Add").unwrap();
    let v_add = ctx.regraph.fresh_id();
    ctx.regraph.insert(add_table_id, row(vec![c1_canon, c2_canon]), v_add);
    ctx.regraph.rebuild();

    // Evaluate a nested expression — should recursively resolve Const(1),
    // Const(2), then Add(eclass1, eclass2)
    let nested = Expr::FunctionCall(FunctionCall(
        "Expr.Add".into(),
        Box::new([
            Expr::FunctionCall(FunctionCall(
                "Expr.Const".into(),
                Box::new([Expr::AtomOrVariable(AtomOrVariable::Atom(Atom::I32(1)))]),
            )),
            Expr::FunctionCall(FunctionCall(
                "Expr.Const".into(),
                Box::new([Expr::AtomOrVariable(AtomOrVariable::Atom(Atom::I32(2)))]),
            )),
        ]),
    ));

    let result = ctx.evaluate_expr(&nested).unwrap();
    let add_canon = ctx.regraph.find(v_add);
    assert_eq!(result, add_canon, "nested expression should resolve to Add eclass");
}

// ============================================================================
// AC-4: Extract with atom literal works
// ============================================================================

#[test]
fn test_extract_atom() {
    let mut ctx = make_ctx();

    let result = do_extract(&mut ctx, "extract 42u64\n");
    assert_eq!(result, "42");
}

// ============================================================================
// AC-5: Extract with undefined constructor errors at compile time
// ============================================================================

#[test]
fn test_extract_unknown_constructor_error() {
    let mut ctx = make_ctx();

    let commands = parse_file("extract NoSuch.Foo(1u64)\n").unwrap();
    let result = compile_command(
        &commands[0],
        &mut ctx.data_types,
        &mut ctx.table_types,
        &mut ctx.interner,
        &ctx.native_names,
        &ctx.native_signatures,
    );
    assert!(result.is_err(), "expected compile error for unknown constructor, got: {result:?}");
}

// ============================================================================
// AC-6: Extract with variable in expression errors at compile time
// ============================================================================

#[test]
fn test_extract_variable_error() {
    let mut ctx = make_ctx();

    let commands = parse_file("extract x\n").unwrap();
    let result = compile_command(
        &commands[0],
        &mut ctx.data_types,
        &mut ctx.table_types,
        &mut ctx.interner,
        &ctx.native_names,
        &ctx.native_signatures,
    );
    assert!(result.is_err(), "expected compile error for variable in extract, got: {result:?}");
}
