use quine_core::common::RowIndex;
use quine_core::related_egraph::RelatedEGraph;
use quine_core::table::Row;
use quine_core::types::{BaseType, TableDef, Type};

// All tables have: [key_cols..., value_col] where value_col is Id-typed.
// arity = number of key columns.

fn expr_lit_table() -> TableDef {
    // data Expr = Lit(i32)  →  key: i32, value: Id
    TableDef(
        "Expr.Lit".into(),
        Box::new([Type::Base(BaseType::I32), Type::Base(BaseType::Id)]),
        None,
    )
}

fn expr_mul_table() -> TableDef {
    // data Expr = Mul(Expr, Expr)  →  key: [Id, Id], value: Id
    TableDef(
        "Expr.Mul".into(),
        Box::new([
            Type::Base(BaseType::Id),
            Type::Base(BaseType::Id),
            Type::Base(BaseType::Id),
        ]),
        None,
    )
}

fn expr_add_table() -> TableDef {
    // data Expr = Add(Expr, Expr)  →  key: [Id, Id], value: Id
    TableDef(
        "Expr.Add".into(),
        Box::new([
            Type::Base(BaseType::Id),
            Type::Base(BaseType::Id),
            Type::Base(BaseType::Id),
        ]),
        None,
    )
}

fn expr_pair_table() -> TableDef {
    // data Expr = Pair(Expr, Expr)  →  key: [Id, Id], value: Id
    TableDef(
        "Expr.Pair".into(),
        Box::new([
            Type::Base(BaseType::Id),
            Type::Base(BaseType::Id),
            Type::Base(BaseType::Id),
        ]),
        None,
    )
}

// Helper: build a Row from a Vec.
fn row(v: Vec<quine_core::common::Value>) -> Row {
    Row(v.into())
}

// ============================================================================
// AC-1: Single enode cost equals constructor cost
// ============================================================================

/// When an enode with no Id-typed children is inserted, its eclass cost
/// equals the constructor cost (no child costs to add).
///
/// Given: `data Expr = Lit(i32)` with `cost Expr.Lit = 5`
/// When:  a Lit enode is inserted
/// Then:  eclass_cost = 5 and cost_select points to the enode
#[test]
fn ac1_single_enode_cost_equals_constructor_cost() {
    let mut eg = RelatedEGraph::default();
    eg.add_table(expr_lit_table());
    eg.set_cost_model("Expr.Lit".into(), 5);

    let v = eg.fresh_id();
    eg.insert(0, row(vec![quine_core::common::Value(42)]), v);

    assert_eq!(eg.eclass_cost(v), 5);
    assert_eq!(eg.cost_select(v), Some((0, RowIndex(0))));
}

// ============================================================================
// AC-2: Multi-enode eclass returns min cost
// ============================================================================

/// When two equivalent expressions (different costs) share an eclass via union,
/// the eclass cost is the minimum.
#[test]
fn ac2_multi_enode_eclass_returns_min_cost() {
    let mut eg = RelatedEGraph::default();
    eg.add_table(expr_lit_table());
    eg.set_cost_model("Expr.Lit".into(), 5);

    let v1 = eg.fresh_id();
    let v2 = eg.fresh_id();
    eg.insert(0, row(vec![quine_core::common::Value(1)]), v1);
    eg.insert(0, row(vec![quine_core::common::Value(2)]), v2);

    // Both have cost 5 independently
    assert_eq!(eg.eclass_cost(v1), 5);
    assert_eq!(eg.eclass_cost(v2), 5);

    // Add a second constructor with a cheaper cost
    eg.add_table(TableDef(
        "Expr.CheapLit".into(),
        Box::new([Type::Base(BaseType::I32), Type::Base(BaseType::Id)]),
        None,
    ));
    eg.set_cost_model("Expr.CheapLit".into(), 1);
    let v3 = eg.fresh_id();
    eg.insert(1, row(vec![quine_core::common::Value(3)]), v3);

    // Union cheap into v1's eclass
    eg.union(v1, v3);
    eg.rebuild();

    let canonical = eg.find(v1);
    // Cost should now be min(5, 1) = 1
    assert_eq!(eg.eclass_cost(canonical), 1);
    // cost_select should point to the cheaper enode
    assert_eq!(eg.cost_select(canonical), Some((1, RowIndex(0))));
}

// ============================================================================
// AC-3: Tree expression cost is sum of constructor costs
// ============================================================================

/// Cost of a tree expression = sum of constructor costs along the tree.
///
/// Given: `cost Expr.Add = 1`, `cost Expr.Mul = 2`, `cost Expr.Lit = 3`
/// When:  Add(Mul(Lit(1), Lit(2)), Lit(3)) is built bottom-up
/// Then:  cost = 1 + (2 + 3 + 3) + 3 = 12
#[test]
fn ac3_tree_expression_cost_is_sum() {
    let mut eg = RelatedEGraph::default();

    eg.add_table(expr_lit_table());
    eg.add_table(expr_mul_table());
    eg.add_table(expr_add_table());

    eg.set_cost_model("Expr.Lit".into(), 3);
    eg.set_cost_model("Expr.Mul".into(), 2);
    eg.set_cost_model("Expr.Add".into(), 1);

    // Build Lit(1) -> e1 (cost 3)
    let e1 = eg.fresh_id();
    eg.insert(0, row(vec![quine_core::common::Value(1)]), e1);
    assert_eq!(eg.eclass_cost(e1), 3);

    // Build Lit(2) -> e2 (cost 3)
    let e2 = eg.fresh_id();
    eg.insert(0, row(vec![quine_core::common::Value(2)]), e2);
    assert_eq!(eg.eclass_cost(e2), 3);

    // Build Mul(e1, e2) -> e3: cost = 2 + 3 + 3 = 8
    let e3 = eg.fresh_id();
    eg.insert(1, row(vec![e1, e2]), e3);
    assert_eq!(eg.eclass_cost(e3), 8);

    // Build Lit(3) -> e4 (cost 3)
    let e4 = eg.fresh_id();
    eg.insert(0, row(vec![quine_core::common::Value(3)]), e4);
    assert_eq!(eg.eclass_cost(e4), 3);

    // Build Add(e3, e4) -> e5: cost = 1 + 8 + 3 = 12
    let e5 = eg.fresh_id();
    eg.insert(2, row(vec![e3, e4]), e5);
    assert_eq!(eg.eclass_cost(e5), 12);

    // cost_select points to the Add enode
    assert_eq!(eg.cost_select(e5), Some((2, RowIndex(0))));
}

// ============================================================================
// AC-4: Undefined constructor defaults to cost 0
// ============================================================================

/// When no cost is defined for a constructor, it defaults to 0.
#[test]
fn ac4_undefined_constructor_defaults_to_zero() {
    let mut eg = RelatedEGraph::default();

    eg.add_table(expr_lit_table());
    eg.add_table(expr_add_table());

    // Only define cost for Lit, NOT for Add
    eg.set_cost_model("Expr.Lit".into(), 10);

    // Build Lit(1) -> e1 (cost 10)
    let e1 = eg.fresh_id();
    eg.insert(0, row(vec![quine_core::common::Value(1)]), e1);

    // Build Lit(2) -> e2 (cost 10)
    let e2 = eg.fresh_id();
    eg.insert(0, row(vec![quine_core::common::Value(2)]), e2);

    // Build Add(e1, e2) -> e3: cost = 0 (default) + 10 + 10 = 20
    let e3 = eg.fresh_id();
    eg.insert(1, row(vec![e1, e2]), e3);
    assert_eq!(eg.eclass_cost(e3), 20);
}

// ============================================================================
// AC-5: cost_select set on insert
// ============================================================================

/// cost_select is populated when an enode is inserted and its cost is
/// the cheapest for its eclass.
#[test]
fn ac5_cost_select_set_on_insert() {
    let mut eg = RelatedEGraph::default();
    eg.add_table(expr_lit_table());
    eg.set_cost_model("Expr.Lit".into(), 5);

    let v = eg.fresh_id();
    eg.insert(0, row(vec![quine_core::common::Value(42)]), v);

    let select = eg.cost_select(v);
    assert!(select.is_some(), "cost_select should be set on insert");
    assert_eq!(select, Some((0, RowIndex(0))));
}

// ============================================================================
// AC-6: cost_select patched when enode absorbed during rebuild
// ============================================================================

/// When rebuild absorbs a scanned enode into an existing one,
/// cost_select is redirected from absorbed -> surviving RowIndex.
#[test]
fn ac6_cost_select_patched_on_rebuild_absorption() {
    let mut eg = RelatedEGraph::default();

    // Seed table to give child eclasses a known cost.
    // data Seed = S(i32) with cost 0
    eg.add_table(TableDef(
        "Seed.S".into(),
        Box::new([Type::Base(BaseType::I32), Type::Base(BaseType::Id)]),
        None,
    ));
    eg.set_cost_model("Seed.S".into(), 0);

    eg.add_table(expr_pair_table());
    eg.set_cost_model("Expr.Pair".into(), 7);

    // Create two child eclasses and seed them with costs
    let child1 = eg.fresh_id();
    eg.insert(0, row(vec![quine_core::common::Value(1)]), child1);
    assert_eq!(eg.eclass_cost(child1), 0);

    let child2 = eg.fresh_id();
    eg.insert(0, row(vec![quine_core::common::Value(2)]), child2);
    assert_eq!(eg.eclass_cost(child2), 0);

    // Insert Pair(child1, child1) -> v1 (RowIndex(0))
    // enode_cost = 7 + 0 + 0 = 7
    let v1 = eg.fresh_id();
    eg.insert(1, row(vec![child1, child1]), v1);
    assert_eq!(eg.cost_select(v1), Some((1, RowIndex(0))));

    // Insert Pair(child1, child2) -> v2 (RowIndex(1)) - different key
    // enode_cost = 7 + 0 + 0 = 7
    let v2 = eg.fresh_id();
    eg.insert(1, row(vec![child1, child2]), v2);
    assert_eq!(eg.cost_select(v2), Some((1, RowIndex(1))));

    // Now union child1 and child2 — canonical(child1) = canonical(child2).
    // Both Pair rows now have canonical key (c, c) where c = find(child1).
    // Rebuild absorbs RowIndex(0) into RowIndex(1).
    eg.union(child1, child2);
    eg.rebuild();

    // After rebuild, v1 and v2 are in the same eclass.
    // R1 (Pair(child1,child2)) is absorbed into R0 (Pair(child1,child1)).
    // cost_select for the merged eclass should point to the surviving row R0.
    let canonical = eg.find(v1);
    let select = eg.cost_select(canonical);
    assert!(select.is_some(), "cost_select should still exist after rebuild");
    // cost_select should point to surviving row R0, not absorbed row R1
    assert_ne!(select, Some((1, RowIndex(1))), "cost_select should not point to absorbed row");
    assert_eq!(select, Some((1, RowIndex(0))));
}
