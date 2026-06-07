// ============================================================================
// AC-4: Exhaustive brute-force verification on small e-graphs
// ============================================================================

use quine_core::common::Value;
use quine_core::related_egraph::RelatedEGraph;
use quine_core::table::Row;
use quine_core::types::{TableDef, Type};
use quine_solver::{ilp_extract, ILPConfig};
use quine_solver::dag::build_extraction_dag;

/// Build a connected chain: root → eclass1 → ... → leaf.
/// Internal nodes use "Link" (arity 1, 1 eclass child + value).
/// Leaf nodes use "Leaf" (arity 0, no children, just value).
fn make_chain(n: usize, k: usize) -> (RelatedEGraph, Value) {
    let mut eg = RelatedEGraph::default();

    // "Link" table: 1 eclass child + value column
    eg.add_table(TableDef(
        "Link".into(),
        Box::new([
            Type::Name("Expr".into()),  // child: eclass
            Type::Name("Expr".into()),  // value
        ]),
        None,
    ));
    // "Leaf" table: 0 children + value column
    eg.add_table(TableDef(
        "Leaf".into(),
        Box::new([Type::Name("Expr".into())]), // value only
        None,
    ));
    eg.set_cost_model("Link".into(), 10);
    eg.set_cost_model("Leaf".into(), 10);

    assert!(n >= 1);
    let values: Vec<Value> = (0..n).map(|_| eg.fresh_id()).collect();

    for i in 0..n {
        if i + 1 < n {
            // Internal: Link enodes pointing to next eclass
            for _ in 0..k {
                eg.insert(0, Row(smallvec::smallvec![values[i + 1]]), values[i]);
            }
        } else {
            // Leaf: no children, arity 0
            for _ in 0..k {
                eg.insert(1, Row(smallvec::smallvec![]), values[i]);
            }
        }
    }

    (eg, values[0])
}

/// Brute-force minimum on the DAG from build_extraction_dag.
fn brute_force_minimum(eg: &RelatedEGraph, root: Value) -> u64 {
    let dag = build_extraction_dag(eg, root);
    let n = dag.eclasses.len();
    if n == 0 {
        return 0;
    }

    let counts: Vec<usize> = dag.eclasses.iter().map(|e| e.enodes.len()).collect();
    for (i, &c) in counts.iter().enumerate() {
        assert!(c > 0, "eclass {} has 0 enodes", i);
    }

    let total: usize = counts.iter().product();
    if total > 100_000 {
        panic!("too many combinations ({})", total);
    }

    let mut best = u64::MAX;
    let mut indices: Vec<usize> = vec![0; n];

    loop {
        let mut cost: u64 = 0;
        for ei in 0..n {
            let (tid, _ridx) = dag.eclasses[ei].enodes[indices[ei]];
            let table = eg.get_table(tid);
            cost += eg.get_constructor_cost(&table.table_def.0);
        }
        if cost < best {
            best = cost;
        }

        let mut carry = 1;
        for i in 0..n {
            indices[i] += carry;
            if indices[i] >= counts[i] {
                indices[i] = 0;
                carry = 1;
            } else {
                carry = 0;
                break;
            }
        }
        if carry == 1 {
            break;
        }
    }

    best
}

// ============================================================================
// Tests
// ============================================================================

/// Exhaustive brute-force verification: ILP cost matches true minimum on small chain e-graphs.
///
/// Given: chain e-graphs of 1..5 eclasses × 1..2 enodes each (all constructor cost = 10),
///        with no CSE edges — tree structure where DP is globally optimal
/// When:  brute-force enumeration checks every possible enode selection against ilp_extract
/// Then:  ILP cost = true minimum (= n × 10) for all 10 instances; optimal = true for all
#[test]
fn test_exhaustive_small_chains() {
    let config = ILPConfig::default();
    // Chain e-graphs: n eclasses, k enodes each. No CSE → tree.
    for n in 1..=5 {
        for k in 1..=2 {
            let (eg, root) = make_chain(n, k);
            let result = ilp_extract(&eg, root, &config);
            let brute = brute_force_minimum(&eg, root);

            assert!(result.optimal, "n={}, k={}: should be optimal", n, k);
            assert_eq!(
                result.cost, brute,
                "n={}, k={}: ILP={}, brute={}",
                n, k, result.cost, brute
            );
            assert_eq!(result.cost, (n * 10) as u64, "n={}, k={}: wrong cost", n, k);
        }
    }
}
