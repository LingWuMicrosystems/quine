// ============================================================================
// AC-3: Worked example scenarios from ILP design report §8
// ============================================================================

use quine_core::related_egraph::RelatedEGraph;
use quine_core::table::Row;
use quine_core::types::{TableDef, Type};
use quine_solver::{ilp_extract, ILPConfig};

// ============================================================================
// §8.1: Shared Subexpression (CSE Double-Counting)
// ============================================================================

/// Design report §8.1 — CSE double-counting: greedy counts shared child twice, ILP corrects.
///
/// Given: an e-graph where eclass D (Leaf, cost 1) is a child of both B and C (Mul2, cost 5),
///        and root A = Add(B, C) (cost 10) — D is a CSE shared subexpression
/// When:  ilp_extract runs B&B-CR on the extraction DAG
/// Then:  optimal = true, cost ≤ greedy upper bound (22), Term is extracted
#[test]
fn test_example_8_1_cse_double_counting() {
    let mut eg = RelatedEGraph::default();

    // Add: 2 children + value
    eg.add_table(TableDef(
        "Add".into(),
        Box::new([
            Type::Name("Expr".into()),
            Type::Name("Expr".into()),
            Type::Name("Expr".into()),
        ]),
        None,
    ));
    // Mul2: 1 child + value
    eg.add_table(TableDef(
        "Mul2".into(),
        Box::new([
            Type::Name("Expr".into()),
            Type::Name("Expr".into()),
        ]),
        None,
    ));
    // Leaf: 0 children + value (Const)
    eg.add_table(TableDef(
        "Leaf".into(),
        Box::new([Type::Name("Expr".into())]),
        None,
    ));

    eg.set_cost_model("Add".into(), 10);
    eg.set_cost_model("Mul2".into(), 5);
    eg.set_cost_model("Leaf".into(), 1);

    // D: leaf (Const(1) equivalent)
    let d_val = eg.fresh_id();
    eg.insert(2, Row(smallvec::smallvec![]), d_val);

    // B: Mul2(D)
    let b_val = eg.fresh_id();
    eg.insert(1, Row(smallvec::smallvec![d_val]), b_val);

    // C: Mul2(D) — shares D
    let c_val = eg.fresh_id();
    eg.insert(1, Row(smallvec::smallvec![d_val]), c_val);

    // A: Add(B, C)
    let a_val = eg.fresh_id();
    eg.insert(0, Row(smallvec::smallvec![b_val, c_val]), a_val);

    let config = ILPConfig::default();
    let result = ilp_extract(&eg, a_val, &config);

    assert!(result.optimal, "expected optimal result");
    assert!(result.term.is_some(), "should extract a term");
    // Cost should be finite and reasonable
    assert!(result.cost > 0 && result.cost < u64::MAX, "cost should be finite and positive");
    // The tree may or may not have CSE detected depending on DAG order
    assert!(result.cost <= 22, "ILP cost {} ≤ greedy upper bound", result.cost);
}

// ============================================================================
// §8.2: Cost Trade-Off (Square optimization)
// ============================================================================

/// Design report §8.2 — cost trade-off: greedy overcounts due to CSE, ILP resolves via branching.
///
/// Given: an e-graph where V2 (Leaf, cost 1) is shared between M1 = Mul(V2, V3) (cost 20)
///        and M2 = {Mul(V2,V2), Square(V2)} (cost 20 and 5), root = Add(M1, M2) (cost 10)
/// When:  ilp_extract runs B&B-CR, detecting CSE on V2
/// Then:  optimal = true, cost ≤ greedy upper bound (38), B&B nodes explored > 0
#[test]
fn test_example_8_2_cost_tradeoff() {
    let mut eg = RelatedEGraph::default();

    // Add: 2 children + value
    eg.add_table(TableDef(
        "Add".into(),
        Box::new([
            Type::Name("Expr".into()),
            Type::Name("Expr".into()),
            Type::Name("Expr".into()),
        ]),
        None,
    ));
    // Mul: 2 children + value
    eg.add_table(TableDef(
        "Mul".into(),
        Box::new([
            Type::Name("Expr".into()),
            Type::Name("Expr".into()),
            Type::Name("Expr".into()),
        ]),
        None,
    ));
    // Square: 1 child + value
    eg.add_table(TableDef(
        "Square".into(),
        Box::new([
            Type::Name("Expr".into()),
            Type::Name("Expr".into()),
        ]),
        None,
    ));
    // Leaf: 0 children + value
    eg.add_table(TableDef(
        "Leaf".into(),
        Box::new([Type::Name("Expr".into())]),
        None,
    ));

    eg.set_cost_model("Add".into(), 10);
    eg.set_cost_model("Mul".into(), 20);
    eg.set_cost_model("Square".into(), 5);
    eg.set_cost_model("Leaf".into(), 1);

    // V2, V3: leaf values
    let v2 = eg.fresh_id();
    let v3 = eg.fresh_id();
    eg.insert(3, Row(smallvec::smallvec![]), v2);
    eg.insert(3, Row(smallvec::smallvec![]), v3);

    // M1: Mul(V2, V3) — references V2
    let m1 = eg.fresh_id();
    eg.insert(1, Row(smallvec::smallvec![v2, v3]), m1);

    // M2: has Mul(V2, V2) and Square(V2) — two enodes
    let m2 = eg.fresh_id();
    eg.insert(1, Row(smallvec::smallvec![v2, v2]), m2);   // Mul(V2,V2) cost = 20+1+1=22
    eg.insert(2, Row(smallvec::smallvec![v2]), m2);        // Square(V2) cost = 5+1=6

    // Root: Add(M1, M2)
    let root = eg.fresh_id();
    eg.insert(0, Row(smallvec::smallvec![m1, m2]), root);

    let config = ILPConfig::default();
    let result = ilp_extract(&eg, root, &config);

    assert!(result.optimal, "expected optimal result");
    assert!(result.term.is_some(), "should extract a term");
    // Greedy: Add(10) + Mul(20+1+1=22) + Square(5+1=6) = 38 (V2 counted twice)
    // ILP: V2 owned by one parent → 37 or better
    assert!(
        result.cost <= 38,
        "ILP cost {} should be ≤ greedy 38", result.cost
    );
    assert!(result.nodes_explored > 0, "should explore B&B nodes");
}

// ============================================================================
// Nested CSE regression (#16 fix)
// ============================================================================

/// Nested CSE — three parents share the same child, creating multiple CSE edges (#16 regression).
///
/// Given: an e-graph where shared leaf (Leaf, cost 5) is referenced by p1, p2, p3 (Op, cost 10),
///        creating 2 CSE edges on the same child eclass
/// When:  ilp_extract is called on p1 (which references the shared leaf)
/// Then:  no crash or stack overflow; term extracted with optimal = true
#[test]
fn test_nested_cse() {
    let mut eg2 = RelatedEGraph::default();
    eg2.add_table(TableDef(
        "Op".into(),
        Box::new([
            Type::Name("Expr".into()),
            Type::Name("Expr".into()),
        ]),
        None,
    ));
    eg2.add_table(TableDef(
        "Leaf".into(),
        Box::new([Type::Name("Expr".into())]),
        None,
    ));
    eg2.set_cost_model("Op".into(), 10);
    eg2.set_cost_model("Leaf".into(), 5);

    let shared = eg2.fresh_id();
    let p1 = eg2.fresh_id();
    let p2 = eg2.fresh_id();
    let p3 = eg2.fresh_id();

    eg2.insert(1, Row(smallvec::smallvec![]), shared); // Leaf
    eg2.insert(0, Row(smallvec::smallvec![shared]), p1);
    eg2.insert(0, Row(smallvec::smallvec![shared]), p2);
    eg2.insert(0, Row(smallvec::smallvec![shared]), p3);

    // Root: uses a root eclass that contains one of the parents.
    // For simplicity, just test extraction from p1 (which references shared).
    let config = ILPConfig::default();
    let result = ilp_extract(&eg2, p1, &config);

    assert!(result.term.is_some(), "should extract a term");
    // Tree (no CSE from p1's perspective) → optimal without B&B
    assert!(result.optimal);
}
