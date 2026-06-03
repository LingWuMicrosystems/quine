use quine::pest_parser::parse_file;
use quine_frontend::compile::compile_command;
use quine_frontend::prelude::register_prelude;
use quine_frontend::syntax::{Command, CostDef};
use quine_frontend::EngineContext;

fn make_ctx() -> EngineContext {
    let mut ctx = EngineContext::default();
    register_prelude(&mut ctx);
    ctx
}

fn compile_and_apply(ctx: &mut EngineContext, cmd: &Command) -> Result<(), String> {
    match cmd {
        Command::Query(_, _) => Ok(()),
        _ => {
            let unit = compile_command(
                cmd,
                &mut ctx.data_types,
                &mut ctx.table_types,
                &mut ctx.interner,
                &ctx.native_names,
                &ctx.native_signatures,
            )
            .map_err(|e| format!("{:?}", e))?;
            ctx.apply(unit);
            Ok(())
        }
    }
}

// ============================================================================
// AC-1: Cost definition for data constructor parses and stores
// ============================================================================

/// Parse a cost definition for a data type constructor and verify it's stored.
///
/// Given: a .quine file with `data Option = Some(value) | None` and `cost Option.Some = 2`
/// When:  the file is parsed, compiled, and applied
/// Then:  EngineContext.cost_models contains "Option.Some" -> 2
#[test]
fn ac1_cost_definition_parses_and_stores() {
    let input = "data Option = Some(value) | None\ncost Option.Some = 2\n";
    let commands = parse_file(input).unwrap();
    assert_eq!(commands.len(), 2, "expected 2 commands");

    let mut ctx = make_ctx();
    for cmd in &commands {
        compile_and_apply(&mut ctx, cmd).unwrap();
    }
    assert_eq!(ctx.regraph.cost_models.get("Option.Some"), Some(&2));
}

// ============================================================================
// AC-2: Undefined constructor defaults to cost 0
// ============================================================================

/// Undefined constructor is absent from cost_models, defaulting to 0.
///
/// Given: `data Option = Some(value) | None` with only `cost Option.Some = 2`
/// When:  cost_models is queried for "Option.None"
/// Then:  the entry is absent (unwraps to default 0)
#[test]
fn ac2_undefined_constructor_defaults_to_0() {
    let input = "data Option = Some(value) | None\ncost Option.Some = 2\n";
    let commands = parse_file(input).unwrap();
    let mut ctx = make_ctx();
    for cmd in &commands {
        compile_and_apply(&mut ctx, cmd).unwrap();
    }
    assert_eq!(ctx.regraph.cost_models.get("Option.Some"), Some(&2));
    // Absent -> defaults to 0 via unwrap_or(0)
    assert_eq!(ctx.regraph.cost_models.get("Option.None"), None);
}

// ============================================================================
// AC-3: Negative cost is rejected at parse time
// ============================================================================

/// Negative cost causes a parse error (panics due to u64 parse).
///
/// Given: cost Option.Some = -1
/// When:  the file is parsed
/// Then:  the parser panics (u64 parse fails on negative integer)
#[test]
#[should_panic(expected = "ParseIntError")]
fn ac3_negative_cost_rejected() {
    let input = "cost Option.Some = -1\n";
    let _ = parse_file(input);
}

// ============================================================================
// AC-4: Cost for relation is rejected at compile time
// ============================================================================

/// Cost for a relation table is rejected.
///
/// Given: relation edge(i32, i32) with cost edge = 5
/// When:  compiled
/// Then:  produces a CompileError (UnknownTypeName)
#[test]
fn ac4_cost_for_relation_rejected() {
    let input = "relation edge(i32, i32)\ncost edge = 5\n";
    let commands = parse_file(input).unwrap();
    assert_eq!(commands.len(), 2);

    let mut ctx = make_ctx();
    compile_and_apply(&mut ctx, &commands[0]).unwrap();
    let result = compile_and_apply(&mut ctx, &commands[1]);
    assert!(result.is_err(), "expected compile error for cost on relation");
}

// ============================================================================
// AC-5: Cost for function table is rejected at compile time
// ============================================================================

/// Cost for a function table is rejected.
///
/// Given: function add(i32, i32) -> i32 merge min with cost add = 5
/// When:  compiled
/// Then:  produces a CompileError (UnknownTypeName)
#[test]
fn ac5_cost_for_function_rejected() {
    let input = "function add(i32, i32) -> i32 merge min\ncost add = 5\n";
    let commands = parse_file(input).unwrap();
    assert_eq!(commands.len(), 2);

    let mut ctx = make_ctx();
    compile_and_apply(&mut ctx, &commands[0]).unwrap();
    let result = compile_and_apply(&mut ctx, &commands[1]);
    assert!(result.is_err(), "expected compile error for cost on function");
}

// ============================================================================
// AC-6: Cost for unknown constructor is rejected
// ============================================================================

/// Cost for a non-existent constructor is rejected.
///
/// Given: data Option = Some(value) | None and cost Option.BadConstructor = 3
/// When:  compiled
/// Then:  produces CompileError (UnknownConstructor)
#[test]
fn ac6_cost_for_unknown_constructor_rejected() {
    let input = "data Option = Some(value) | None\ncost Option.BadConstructor = 3\n";
    let commands = parse_file(input).unwrap();
    assert_eq!(commands.len(), 2);

    let mut ctx = make_ctx();
    compile_and_apply(&mut ctx, &commands[0]).unwrap();
    let result = compile_and_apply(&mut ctx, &commands[1]);
    assert!(result.is_err(), "expected compile error for unknown constructor");
}

// ============================================================================
// AC-7: Cost zero parses correctly
// ============================================================================

/// Cost value of zero parses correctly.
///
/// Given: cost Option.None = 0
/// When:  parsed, compiled, and applied
/// Then:  cost_models contains "Option.None" -> 0
#[test]
fn ac7_cost_zero_parses() {
    let input = "data Option = Some(value) | None\ncost Option.None = 0\n";
    let commands = parse_file(input).unwrap();

    let mut ctx = make_ctx();
    for cmd in &commands {
        compile_and_apply(&mut ctx, cmd).unwrap();
    }
    assert_eq!(ctx.regraph.cost_models.get("Option.None"), Some(&0));
}

// ============================================================================
// AC-8: Display round-trip for CostDef
// ============================================================================

/// CostDef Display output matches input syntax.
///
/// Given: a CostDef
/// When:  Display::fmt is called
/// Then:  output is "cost TypeName.ConsName = <cost>"
#[test]
fn ac8_cost_display_roundtrip() {
    let def = CostDef {
        type_name: "Option".into(),
        constructor: "Some".into(),
        cost: 2,
    };
    assert_eq!(format!("{def}"), "cost Option.Some = 2");
    assert_eq!(format!("{}", Command::CostDef(def)), "cost Option.Some = 2");
}
