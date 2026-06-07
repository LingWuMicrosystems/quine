use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use quine_core::common::{RowIndex, Value};
use quine_core::related_egraph::{RelatedEGraph, TableId};
use quine_frontend::syntax::Atom;
use quine_frontend::term::Term;

use crate::dag::ExtractionDAG;
use crate::formulation::{atom_from_value, type_is_eclass};
use crate::relaxation::{
    find_cse_violations, pick_branching_eclass, solve_relaxation, FixedDecision, Solution,
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
    let violations = find_cse_violations(dag, &relaxed);
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
        child.fixed.insert(eclass_idx, FixedDecision::NotShared);
        branch_and_bound(dag, regraph, &child, best, stats);
    }

    // --- Branch B: OwnedBy each parent ---
    for &(parent_idx, enode_idx) in &cse_edge.parent_enodes {
        let mut child = node.clone();
        child
            .fixed
            .insert(eclass_idx, FixedDecision::OwnedBy(parent_idx));
        // Fix the parent to select the specific enode that references this child.
        let (tid, ridx) = dag.eclasses[parent_idx].enodes[enode_idx];
        child
            .fixed
            .insert(parent_idx, FixedDecision::Selected(tid, ridx));
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
