use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use quine_core::common::{RowIndex, Value};
use quine_core::related_egraph::{RelatedEGraph, TableId};
use quine_frontend::syntax::Atom;
use quine_frontend::term::Term;

use crate::dag::ExtractionDAG;
use crate::formulation::{atom_from_value, type_is_eclass};
use crate::relaxation::{
    find_cse_violations, pick_branching_eclass, solve_relaxation, CseDecision, FixedDecision,
    Solution,
};

/// A node in the Branch-and-Bound search tree.
///
/// Each node accumulates fixed decisions as the search descends.
/// Branching clones the node and adds one or more decisions to the
/// child's fixed map.
#[derive(Debug, Clone)]
pub struct BnBNode {
    /// Accumulated decisions from root to this node.
    pub fixed: BTreeMap<usize, FixedDecision>,
}

/// Statistics tracked during B&B search.
#[derive(Debug, Clone, Default)]
pub struct BnBStats {
    /// Number of B&B nodes explored (relaxations solved).
    pub nodes_explored: u64,
}

/// Recursive Branch-and-Bound with Combinatorial Relaxation (B&B-CR).
///
/// At each node:
/// 1. Solve the combinatorial relaxation (DAG shortest-path, drops CSE coupling).
/// 2. Bound: prune if relaxation cost ≥ best known cost.
/// 3. If no CSE violations: update best solution (feasible leaf).
/// 4. Otherwise: branch on the most-violated CSE eclass.
///
/// ## Branching structure
/// - **Branch A (NotShared):** The shared eclass is treated as NOT shared —
///   each parent independently pays its cost. This is the "break CSE" branch.
/// - **Branch B (OwnedBy):** For each parent enode referencing the shared
///   eclass, branch with the child owned by that parent. The owning parent
///   is also fixed to select the referencing enode (consistency).
///
/// ## Pruning
/// Branches are pruned when the relaxation bound ≥ the incumbent cost.
/// Since all costs are non-negative, the relaxation is always a lower bound.
pub fn branch_and_bound(
    dag: &ExtractionDAG,
    regraph: &RelatedEGraph,
    node: &BnBNode,
    best: &mut Solution,
    stats: &mut BnBStats,
) {
    stats.nodes_explored += 1;

    // 1. Solve combinatorial relaxation at this node.
    let relaxed = solve_relaxation(dag, regraph, &node.fixed);

    // 2. Bound: prune if relaxation doesn't improve on incumbent.
    if relaxed.cost >= best.cost {
        return;
    }

    // 3. Check feasibility: are there CSE violations?
    let violations = find_cse_violations(dag, &relaxed, &node.fixed);
    if violations.is_empty() {
        // Feasible solution found — update incumbent.
        best.enode_selection = relaxed.enode_selection;
        best.cost = relaxed.cost;
        return;
    }

    // 4. Branch: pick the eclass with most CSE violations.
    let eclass_idx = pick_branching_eclass(dag, &violations);

    // Find the CSE edge for this eclass (needed for parent_enodes list).
    let cse_edge = dag
        .cse_edges
        .iter()
        .find(|e| e.child_eclass == eclass_idx)
        .expect("violated eclass must have a CSE edge");

    // --- Branch A: NotShared (break CSE coupling) ---
    {
        let mut child = node.clone();
        child
            .fixed
            .entry(eclass_idx)
            .or_default()
            .cse = Some(CseDecision::NotShared);
        branch_and_bound(dag, regraph, &child, best, stats);
    }

    // --- Branch B: OwnedBy each parent ---
    for &(parent_idx, enode_idx) in &cse_edge.parent_enodes {
        let mut child = node.clone();
        // Merge: don't overwrite existing decisions on this eclass.
        child
            .fixed
            .entry(eclass_idx)
            .or_default()
            .cse = Some(CseDecision::OwnedBy(parent_idx));
        // Fix the parent to select the specific enode that references this child.
        // Also use merge to coexist with any existing CSE decision on the parent.
        let (tid, ridx) = dag.eclasses[parent_idx].enodes[enode_idx];
        child
            .fixed
            .entry(parent_idx)
            .or_default()
            .selected = Some((tid, ridx));
        branch_and_bound(dag, regraph, &child, best, stats);
    }
}

/// Materialize a `Term` from a `Solution` by walking the DAG.
///
/// Starting from the root eclass, follows the selected enode at each
/// eclass, recursing into eclass-typed children and emitting literals
/// for base-typed children via `atom_from_value`.
///
/// # Cycles
/// A visited set guards against cycles (should not occur for valid DAGs,
/// but e-graphs can contain self-referencing enodes). Cyclic references
/// are emitted as `Term::Literal(Atom::U64(canonical.0))`.
pub fn extract_solution_from_dag(
    dag: &ExtractionDAG,
    regraph: &RelatedEGraph,
    solution: &Solution,
) -> Term {
    // Build eclass_map on the fly from the already-indexed eclasses vector.
    let eclass_map: BTreeMap<Value, usize> = dag
        .eclasses
        .iter()
        .enumerate()
        .map(|(i, node)| (node.canonical, i))
        .collect();

    let mut visited: BTreeMap<usize, ()> = BTreeMap::new();
    build_term(
        dag,
        regraph,
        dag.root,
        &solution.enode_selection,
        &eclass_map,
        &mut visited,
    )
}

/// Recursively build a `Term` from the solution's enode selections.
fn build_term(
    dag: &ExtractionDAG,
    regraph: &RelatedEGraph,
    eclass_idx: usize,
    selection: &[Option<(TableId, RowIndex)>],
    eclass_map: &BTreeMap<Value, usize>,
    visited: &mut BTreeMap<usize, ()>,
) -> Term {
    // Cycle guard: if we've already visited this eclass, emit a raw Value.
    if visited.contains_key(&eclass_idx) {
        return Term::Literal(Atom::U64(dag.eclasses[eclass_idx].canonical.0));
    }
    visited.insert(eclass_idx, ());

    match selection[eclass_idx] {
        None => {
            // No enode selected — emit raw canonical Value.
            Term::Literal(Atom::U64(dag.eclasses[eclass_idx].canonical.0))
        }
        Some((tid, ridx)) => {
            let table = regraph.get_table(tid);
            let table_name = table.table_def.0.clone();
            let row = table.get_all_row(ridx);
            let mut children: Vec<Term> = Vec::new();

            for col in 0..table.arity() {
                let child_val = row.0[col];
                let child_ty = &table.table_def.1[col];
                if type_is_eclass(child_ty) {
                    let child_canon = regraph.find(child_val);
                    if let Some(&child_idx) = eclass_map.get(&child_canon) {
                        children.push(build_term(
                            dag,
                            regraph,
                            child_idx,
                            selection,
                            eclass_map,
                            visited,
                        ));
                    } else {
                        // Child eclass not in DAG (shouldn't happen for valid DAGs).
                        children.push(Term::Literal(Atom::U64(child_canon.0)));
                    }
                } else {
                    children.push(Term::Literal(atom_from_value(child_val, child_ty)));
                }
            }

            Term::App(table_name, children)
        }
    }
}

// ============================================================================
// Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::BTreeMap;
    use alloc::vec;
    use quine_core::common::Value;
    use quine_core::related_egraph::RelatedEGraph;
    use quine_core::table::Row;
    use quine_core::types::{TableDef, Type};
    use crate::dag::{CseEdge, EclassNode, ExtractionDAG};

    /// Create a minimal e-graph with an "Op" table.
    fn make_eg(child_types: &[Type], cost: u64) -> RelatedEGraph {
        let mut eg = RelatedEGraph::default();
        let mut all: Vec<Type> = child_types.to_vec();
        all.push(Type::Name("Expr".into()));
        eg.add_table(TableDef("Op".into(), all.into_boxed_slice(), None));
        eg.set_cost_model("Op".into(), cost);
        eg
    }

    /// Build a CSE DAG (like design report §8.1): A=0, B=1, C=2, D=3 (BFS order).
    fn make_cse_dag() -> (RelatedEGraph, ExtractionDAG) {
        let types = [Type::Name("Expr".into()), Type::Name("Expr".into())];
        let mut eg = make_eg(&types, 5);
        let a = eg.fresh_id();
        let b = eg.fresh_id();
        let c = eg.fresh_id();
        let d = eg.fresh_id();
        let z: Vec<Value> = (0..5).map(|_| eg.fresh_id()).collect();

        // A: root, children = B, C
        eg.insert(0, Row(smallvec::smallvec![b, c]), a);
        // B: child = D
        eg.insert(0, Row(smallvec::smallvec![d, z[0]]), b);
        // C: child = D
        eg.insert(0, Row(smallvec::smallvec![d, z[1]]), c);
        // D: leaf (2 enodes)
        eg.insert(0, Row(smallvec::smallvec![z[2], z[3]]), d);
        eg.insert(0, Row(smallvec::smallvec![z[4], z[4]]), d);

        let dag = ExtractionDAG {
            eclasses: vec![
                EclassNode { canonical: a, enodes: vec![(0, RowIndex(0))] },
                EclassNode { canonical: b, enodes: vec![(0, RowIndex(1))] },
                EclassNode { canonical: c, enodes: vec![(0, RowIndex(2))] },
                EclassNode { canonical: d, enodes: vec![(0, RowIndex(3)), (0, RowIndex(4))] },
            ],
            root: 0,
            cse_edges: vec![CseEdge {
                child_eclass: 3,
                parent_enodes: vec![(1, 0), (2, 0)],
            }],
        };
        (eg, dag)
    }

    // ------------------------------------------------------------------
    // AC-2: branch_and_bound
    // ------------------------------------------------------------------

    /// Tree DAG (no CSE) — B&B finds optimum in a single root node.
    ///
    /// Given: a 2-eclass tree DAG with no CSE edges
    /// When:  branch_and_bound is called starting from a fresh BnBNode
    /// Then:  best cost = 10 (constructor costs only), nodes_explored = 1 (no branching needed)
    #[test]
    fn test_branch_and_bound_tree() {
        // Tree DAG (no CSE) — B&B finds optimum in 1 node.
        let types = [Type::Name("Expr".into())];
        let mut eg = make_eg(&types, 5);
        let r = eg.fresh_id();
        let c = eg.fresh_id();
        let z = eg.fresh_id();
        eg.insert(0, Row(smallvec::smallvec![z]), c);
        eg.insert(0, Row(smallvec::smallvec![c]), r);

        let dag = ExtractionDAG {
            eclasses: vec![
                EclassNode { canonical: r, enodes: vec![(0, RowIndex(1))] },
                EclassNode { canonical: c, enodes: vec![(0, RowIndex(0))] },
            ],
            root: 0,
            cse_edges: vec![],
        };

        let mut best = Solution { enode_selection: vec![None; 2], cost: u64::MAX };
        let mut stats = BnBStats::default();
        let node = BnBNode { fixed: BTreeMap::new() };
        branch_and_bound(&dag, &eg, &node, &mut best, &mut stats);

        assert_eq!(best.cost, 10);
        assert_eq!(stats.nodes_explored, 1);
    }

    /// CSE DAG — B&B branches and finds optimum lower than greedy relaxation.
    ///
    /// Given: a DAG with 1 CSE edge (D shared by B and C)
    /// When:  branch_and_bound explores branches (NotShared, OwnedBy)
    /// Then:  best cost < 25 (relaxation), nodes_explored > 1 (multiple branches explored)
    #[test]
    fn test_branch_and_bound_single_cse() {
        // CSE DAG — B&B finds optimum with cost lower than greedy relaxation.
        let (eg, dag) = make_cse_dag();

        let n = dag.eclasses.len();
        let mut best = Solution { enode_selection: vec![None; n], cost: u64::MAX };
        let mut stats = BnBStats::default();
        let node = BnBNode { fixed: BTreeMap::new() };
        branch_and_bound(&dag, &eg, &node, &mut best, &mut stats);

        // Relaxation (greedy) cost without CSE adjustment: 25.
        // Optimal with CSE: 20 or less (one branch with OwnedBy).
        assert!(best.cost < 25, "expected optimal < 25, got {}", best.cost);
        assert!(stats.nodes_explored > 1, "should explore multiple B&B nodes");
        assert!(best.cost < u64::MAX);
    }

    /// Bound pruning: B&B skips nodes where relaxation ≥ incumbent.
    ///
    /// Given: a CSE DAG and an incumbent cost already set to relaxation value (25)
    /// When:  branch_and_bound evaluates the root node
    /// Then:  relaxation cost (25) ≥ best cost (25) → pruned immediately, nodes_explored = 1
    #[test]
    fn test_branch_and_bound_pruning() {
        // With an incumbent already at relaxation cost, B&B should prune immediately.
        let (eg, dag) = make_cse_dag();

        let n = dag.eclasses.len();
        // Set best to relaxation cost (25) as if a leaf already found.
        let mut best = Solution { enode_selection: vec![None; n], cost: 25 };
        let mut stats = BnBStats::default();
        let node = BnBNode { fixed: BTreeMap::new() };
        branch_and_bound(&dag, &eg, &node, &mut best, &mut stats);

        // The root relaxation has cost 25. Since best.cost = 25,
        // `relaxed.cost >= best.cost` → prune at root → 1 node only.
        assert_eq!(stats.nodes_explored, 1);
    }

    // ------------------------------------------------------------------
    // AC-2: extract_solution_from_dag
    // ------------------------------------------------------------------

    /// Materializes a valid Term from a Solution by walking selected enodes.
    ///
    /// Given: a 2-eclass tree DAG and a Solution selecting enodes for each eclass
    /// When:  extract_solution_from_dag builds the Term tree
    /// Then:  returns Term::App("Op", [child_term]) with correct structure
    #[test]
    fn test_extract_solution_from_dag() {
        let types = [Type::Name("Expr".into())];
        let mut eg = make_eg(&types, 5);
        let r = eg.fresh_id();
        let c = eg.fresh_id();
        let z = eg.fresh_id();
        eg.insert(0, Row(smallvec::smallvec![z]), c);
        eg.insert(0, Row(smallvec::smallvec![c]), r);

        let dag = ExtractionDAG {
            eclasses: vec![
                EclassNode { canonical: r, enodes: vec![(0, RowIndex(1))] },
                EclassNode { canonical: c, enodes: vec![(0, RowIndex(0))] },
            ],
            root: 0,
            cse_edges: vec![],
        };

        let solution = Solution {
            enode_selection: vec![Some((0, RowIndex(1))), Some((0, RowIndex(0)))],
            cost: 10,
        };
        let term = extract_solution_from_dag(&dag, &eg, &solution);
        match &term {
            Term::App(name, children) => {
                assert_eq!(name, "Op");
                assert_eq!(children.len(), 1);
            }
            _ => panic!("expected Term::App, got {:?}", term),
        }
    }
}
