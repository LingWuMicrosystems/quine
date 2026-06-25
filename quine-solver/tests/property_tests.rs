// ============================================================================
// AC-5: Property invariants for the B&B-CR solver
// ============================================================================

use quine_core::common::Value;
use quine_core::related_egraph::RelatedEGraph;
use quine_core::table::Row;
use quine_core::term::Term;
use quine_core::types::{TableDef, Type};
use quine_solver::{ilp_extract, ILPConfig};

// ============================================================================
// No-CSE case: ILP == greedy (fast path)
// ============================================================================

/// No-CSE tree: ILP fast path returns optimal without B&B.
///
/// Given: a 2-eclass tree e-graph (root Link → child Leaf, costs 5 each)
/// When:  ilp_extract is called — no CSE edges → fast path
/// Then:  optimal = true, nodes_explored = 0, cost = 10 (= 5 + 5)
#[test]
fn test_no_cse_optimal() {
    let mut eg = RelatedEGraph::default();
    // Internal node: "Link" with 1 eclass child
    eg.add_table(TableDef(
        "Link".into(),
        Box::new([Type::Name("Expr".into()), Type::Name("Expr".into())]),
        None,
    ));
    // Leaf: "Leaf" with 0 children
    eg.add_table(TableDef(
        "Leaf".into(),
        Box::new([Type::Name("Expr".into())]),
        None,
    ));
    eg.set_cost_model("Link".into(), 5);
    eg.set_cost_model("Leaf".into(), 5);

    let child = eg.fresh_id();
    let root = eg.fresh_id();

    // child = Leaf (arity 0, no children)
    eg.insert(1, Row(smallvec::smallvec![]), child);
    // root = Link(child)
    eg.insert(0, Row(smallvec::smallvec![child]), root);

    let result = ilp_extract(&eg, root, &ILPConfig::default());
    assert!(result.optimal, "no-CSE should be optimal");
    assert_eq!(result.nodes_explored, 0, "no B&B needed for tree");
    assert_eq!(result.cost, 10, "5 (Leaf) + 5 (Link) = 10");
    assert!(result.term.is_some());
}

// ============================================================================
// ILP cost ≤ greedy cost (with CSE)
// ============================================================================

/// ILP cost never exceeds greedy cost — CSE-aware accounting corrects double-counting.
///
/// Given: an e-graph where shared leaf (cost 5) is referenced by p1 and p2 (Mul, cost 5),
///        root = Add(p1, p2) (cost 10) — greedy double-counts: 10+10+10 = 30
/// When:  ilp_extract runs B&B-CR
/// Then:  ILP cost ≤ 30 (CSE correction: shared counted once = 25)
#[test]
fn test_ilp_cost_le_greedy() {
    let mut eg = RelatedEGraph::default();
    // "Add" with 2 eclass children
    eg.add_table(TableDef(
        "Add".into(),
        Box::new([
            Type::Name("Expr".into()),
            Type::Name("Expr".into()),
            Type::Name("Expr".into()),
        ]),
        None,
    ));
    // "Mul" with 1 eclass child
    eg.add_table(TableDef(
        "Mul".into(),
        Box::new([Type::Name("Expr".into()), Type::Name("Expr".into())]),
        None,
    ));
    // "Leaf" with 0 children
    eg.add_table(TableDef(
        "Leaf".into(),
        Box::new([Type::Name("Expr".into())]),
        None,
    ));
    eg.set_cost_model("Add".into(), 10);
    eg.set_cost_model("Mul".into(), 5);
    eg.set_cost_model("Leaf".into(), 5);

    let shared = eg.fresh_id(); // leaf shared by p1 and p2
    let p1 = eg.fresh_id();
    let p2 = eg.fresh_id();
    let root = eg.fresh_id();

    eg.insert(2, Row(smallvec::smallvec![]), shared); // Leaf(shared)
    eg.insert(1, Row(smallvec::smallvec![shared]), p1); // Mul(shared)
    eg.insert(1, Row(smallvec::smallvec![shared]), p2); // Mul(shared)
    eg.insert(0, Row(smallvec::smallvec![p1, p2]), root); // Add(p1, p2)

    let result = ilp_extract(&eg, root, &ILPConfig::default());

    // Greedy/relaxation: Add(10) + Mul(5+5) + Mul(5+5) = 30 (shared counted twice)
    // ILP with CSE: Add(10) + Mul(5) + Mul(5) + shared(5) = 25 (shared once)
    assert!(
        result.cost <= 30,
        "ILP cost {} should be ≤ greedy cost 30", result.cost
    );
    assert!(result.term.is_some());
}

// ============================================================================
// Valid root produces a term
// ============================================================================

/// A valid e-graph root with enodes produces a concrete Term.
///
/// Given: an e-graph with a single eclass containing one Leaf enode (cost 1)
/// When:  ilp_extract is called on that root
/// Then:  term is Some, optimal = true, cost = 1
#[test]
fn test_valid_root_produces_term() {
    let mut eg = RelatedEGraph::default();
    eg.add_table(TableDef(
        "Leaf".into(),
        Box::new([Type::Name("Expr".into())]),
        None,
    ));
    eg.set_cost_model("Leaf".into(), 1);

    let root = eg.fresh_id();
    eg.insert(0, Row(smallvec::smallvec![]), root);

    let result = ilp_extract(&eg, root, &ILPConfig::default());
    assert!(result.term.is_some(), "valid root should produce a term");
    assert!(result.optimal, "single enode should be optimal");
    assert_eq!(result.cost, 1);
}

// ============================================================================
// max_eclasses fallback
// ============================================================================

/// ILPConfig.max_eclasses threshold triggers greedy fallback when exceeded.
///
/// Given: a connected chain of 5 eclasses (Link + Leaf, costs 5 each)
/// When:  max_eclasses = 2 (actual DAG has 5 eclasses → exceeds threshold)
/// Then:  optimal = false, nodes_explored = 0 (no B&B), term still produced via greedy
#[test]
fn test_max_eclasses_fallback() {
    let mut eg = RelatedEGraph::default();
    eg.add_table(TableDef(
        "Link".into(),
        Box::new([Type::Name("Expr".into()), Type::Name("Expr".into())]),
        None,
    ));
    eg.add_table(TableDef(
        "Leaf".into(),
        Box::new([Type::Name("Expr".into())]),
        None,
    ));
    eg.set_cost_model("Link".into(), 5);
    eg.set_cost_model("Leaf".into(), 5);

    // Build a connected chain of 5 eclasses: root → a → b → c → leaf
    let vals: Vec<Value> = (0..5).map(|_| eg.fresh_id()).collect();
    eg.insert(1, Row(smallvec::smallvec![]), vals[4]); // leaf
    for i in 0..4 {
        eg.insert(0, Row(smallvec::smallvec![vals[i + 1]]), vals[i]);
    }

    // Artificially low threshold
    let config = ILPConfig {
        max_eclasses: 2,
        ..ILPConfig::default()
    };

    let result = ilp_extract(&eg, vals[0], &config);
    assert!(!result.optimal, "should fall back to greedy");
    assert_eq!(result.nodes_explored, 0);
    assert!(result.term.is_some(), "fallback should still produce a term");
}

// ============================================================================
// Cost is non-negative and finite
// ============================================================================

/// Extracted cost is always finite and non-negative (never u64::MAX).
///
/// Given: a single-eclass e-graph with one Leaf enode (cost 5)
/// When:  ilp_extract runs to completion
/// Then:  cost ≠ u64::MAX, cost < u64::MAX, optimal = true
#[test]
fn test_cost_finite() {
    let mut eg = RelatedEGraph::default();
    eg.add_table(TableDef(
        "Leaf".into(),
        Box::new([Type::Name("Expr".into())]),
        None,
    ));
    eg.set_cost_model("Leaf".into(), 5);

    let root = eg.fresh_id();
    eg.insert(0, Row(smallvec::smallvec![]), root);

    let result = ilp_extract(&eg, root, &ILPConfig::default());
    assert_ne!(result.cost, u64::MAX, "cost should not be MAX");
    assert!(result.cost < u64::MAX, "cost should be finite");
    assert!(result.optimal);
}

// ============================================================================
// Single eclass → picks cheapest enode
// ============================================================================

/// Single eclass with multiple enodes picks the globally cheapest one.
///
/// Given: an eclass with two enodes — Expensive (cost 5) and Cheap (cost 1)
/// When:  ilp_extract evaluates both enodes
/// Then:  cost = 1 (cheaper enode selected), optimal = true
#[test]
fn test_single_eclass_picks_cheapest() {
    let mut eg = RelatedEGraph::default();
    eg.add_table(TableDef(
        "Expensive".into(),
        Box::new([Type::Name("Expr".into())]),
        None,
    ));
    eg.add_table(TableDef(
        "Cheap".into(),
        Box::new([Type::Name("Expr".into())]),
        None,
    ));
    eg.set_cost_model("Expensive".into(), 5);
    eg.set_cost_model("Cheap".into(), 1);

    let root = eg.fresh_id();
    // Two enodes in same eclass: expensive (table 0) and cheap (table 1)
    eg.insert(0, Row(smallvec::smallvec![]), root);
    eg.insert(1, Row(smallvec::smallvec![]), root);

    let result = ilp_extract(&eg, root, &ILPConfig::default());
    assert!(result.optimal);
    assert_eq!(result.cost, 1, "should pick cheaper enode (cost=1)");
    assert!(result.term.is_some());
}

// ============================================================================
// Extracted term is structurally valid
// ============================================================================

/// Extracted Term has correct structure — constructor name and child arity.
///
/// Given: a 2-eclass tree (root Link → child Leaf)
/// When:  ilp_extract returns a term
/// Then:  term is Term::App("Link", [child]), child is a valid sub-term
#[test]
fn test_extracted_term_valid() {
    let mut eg = RelatedEGraph::default();
    eg.add_table(TableDef(
        "Link".into(),
        Box::new([Type::Name("Expr".into()), Type::Name("Expr".into())]),
        None,
    ));
    eg.add_table(TableDef(
        "Leaf".into(),
        Box::new([Type::Name("Expr".into())]),
        None,
    ));
    eg.set_cost_model("Link".into(), 5);
    eg.set_cost_model("Leaf".into(), 5);

    let child = eg.fresh_id();
    let root = eg.fresh_id();
    eg.insert(1, Row(smallvec::smallvec![]), child);
    eg.insert(0, Row(smallvec::smallvec![child]), root);

    let result = ilp_extract(&eg, root, &ILPConfig::default());
    assert!(result.term.is_some());

    let term = result.term.unwrap();
    match &term {
        Term::App(name, children) => {
            assert_eq!(name, "Link");
            assert_eq!(children.len(), 1);
        }
        _ => panic!("expected App, got {:?}", term),
    }
}
