use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;
use quine_core::common::{RowIndex, Value};
use quine_core::related_egraph::{RelatedEGraph, TableId};

use crate::dag::ExtractionDAG;
use crate::formulation::{constructor_cost, type_is_eclass};

/// A fixed decision constraining the solver at a B&B node.
///
/// These accumulate as the B&B tree descends — each branch adds one
/// or more fixed decisions to its parent's map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixedDecision {
    /// Force a specific enode to be selected in this eclass.
    Selected(TableId, RowIndex),
    /// This eclass's cost is "owned" by `parent_idx` for CSE accounting.
    /// Other parent eclasses do not add this eclass's cost when computing
    /// their own enode costs — only the owning parent does.
    OwnedBy(usize),
    /// Break CSE coupling: each parent independently pays this eclass's
    /// cost. This is the relaxation default and corresponds to "the
    /// child is NOT shared — it appears once per parent in the expression."
    NotShared,
}

/// Result of solving the combinatorial relaxation at a B&B node.
///
/// The relaxation drops CSE coupling constraints, reducing the problem
/// to a DAG shortest-path solvable in O(|E|) time.
#[derive(Debug, Clone)]
pub struct RelaxedSolution {
    /// Selected enode for each eclass (indexed by eclass position in DAG).
    /// `None` if no enode could be selected (should not occur for valid DAGs).
    pub enode_selection: Vec<Option<(TableId, RowIndex)>>,
    /// Total cost from the root eclass (the relaxation bound).
    pub cost: u64,
}

/// Solve the combinatorial relaxation at a B&B node.
///
/// Drops all CSE coupling constraints — each eclass independently picks
/// its cheapest enode via DP from leaves to root. Fixed decisions from
/// the B&B node are respected (e.g., forced enode selections, CSE ownership).
///
/// The CSE adjustment: when computing enode cost for eclass `p`, for each
/// child eclass `c`:
/// - If `c` is `OwnedBy(owner)` where `owner != p`: add 0 (cost counted at owner)
/// - Otherwise: add `cost[c]` (default: each parent pays child cost)
///
/// This relaxation bound is the "greedy" cost — it counts shared eclasses
/// multiple times (once per selected parent). B&B branches to resolve
/// the overcount.
pub fn solve_relaxation(
    dag: &ExtractionDAG,
    regraph: &RelatedEGraph,
    fixed: &BTreeMap<usize, FixedDecision>,
) -> RelaxedSolution {
    let n = dag.eclasses.len();
    let mut cost = vec![u64::MAX; n];
    let mut enode = vec![None; n];

    // Build eclass_map on the fly for canonical Value → DAG index lookups.
    let eclass_map: BTreeMap<Value, usize> = dag
        .eclasses
        .iter()
        .enumerate()
        .map(|(i, node)| (node.canonical, i))
        .collect();

    // DP: leaves first (reverse BFS order — root first, leaves last).
    for eclass_idx in (0..n).rev() {
        if let Some(decision) = fixed.get(&eclass_idx) {
            let (c, e) = apply_fixed(dag, regraph, fixed, eclass_idx, decision, &cost, &eclass_map);
            cost[eclass_idx] = c;
            enode[eclass_idx] = e;
            continue;
        }

        // No fixed decision — pick cheapest enode with CSE-aware child costs.
        for &(tid, ridx) in &dag.eclasses[eclass_idx].enodes {
            let enode_cost = enode_cost_with_cse(
                dag, regraph, fixed, eclass_idx, tid, ridx, &cost, &eclass_map,
            );
            if enode_cost < cost[eclass_idx] {
                cost[eclass_idx] = enode_cost;
                enode[eclass_idx] = Some((tid, ridx));
            }
        }
    }

    RelaxedSolution {
        cost: cost[dag.root],
        enode_selection: enode,
    }
}

/// Apply a fixed decision for an eclass, returning its (cost, enode).
fn apply_fixed(
    dag: &ExtractionDAG,
    regraph: &RelatedEGraph,
    fixed: &BTreeMap<usize, FixedDecision>,
    eclass_idx: usize,
    decision: &FixedDecision,
    cost: &[u64],
    eclass_map: &BTreeMap<Value, usize>,
) -> (u64, Option<(TableId, RowIndex)>) {
    match decision {
        FixedDecision::Selected(tid, ridx) => {
            let tid = *tid;
            let ridx = *ridx;
            let c = enode_cost_with_cse(
                dag, regraph, fixed, eclass_idx, tid, ridx, cost, eclass_map,
            );
            (c, Some((tid, ridx)))
        }
        FixedDecision::OwnedBy(_) | FixedDecision::NotShared => {
            // Ownership affects parent-side accounting, not this eclass's
            // own selection. Pick cheapest enode normally.
            let mut best_c = u64::MAX;
            let mut best_e = None;
            for &(tid, ridx) in &dag.eclasses[eclass_idx].enodes {
                let c = enode_cost_with_cse(
                    dag, regraph, fixed, eclass_idx, tid, ridx, cost, eclass_map,
                );
                if c < best_c {
                    best_c = c;
                    best_e = Some((tid, ridx));
                }
            }
            (best_c, best_e)
        }
    }
}

/// Compute the cost of a specific enode in a specific eclass, applying
/// CSE ownership adjustments for child eclass costs.
///
/// For each child eclass `c` of the enode:
/// - If `c` is `OwnedBy(owner)` and `owner != parent_eclass`: child cost
///   is NOT added (it is accounted for at the owning parent).
/// - Otherwise: child cost IS added (default behavior).
fn enode_cost_with_cse(
    _dag: &ExtractionDAG,
    regraph: &RelatedEGraph,
    fixed: &BTreeMap<usize, FixedDecision>,
    parent_idx: usize,
    tid: TableId,
    ridx: RowIndex,
    cost: &[u64],
    eclass_map: &BTreeMap<Value, usize>,
) -> u64 {
    let table = regraph.get_table(tid);
    let mut total = constructor_cost(regraph, &table.table_def.0);
    let row = table.get_all_row(ridx);

    for col in 0..table.arity() {
        let col_type = &table.table_def.1[col];
        if type_is_eclass(col_type) {
            let child_val = row.0[col];
            let child_canon = regraph.find(child_val);
            if let Some(&child_idx) = eclass_map.get(&child_canon) {
                // CSE adjustment: if child is OwnedBy a DIFFERENT parent,
                // this parent doesn't pay the child's cost.
                let child_cost = match fixed.get(&child_idx) {
                    Some(FixedDecision::OwnedBy(owner)) if *owner != parent_idx => 0,
                    _ => cost[child_idx],
                };
                total = total.saturating_add(child_cost);
            }
        }
    }

    total
}

/// Find eclasses where the relaxation double-counts CSE costs.
///
/// Returns `(cse_edge_index, selected_parent_count)` for each CSE edge
/// where more than one parent enode is selected in the relaxed solution.
/// These are the branching candidates — the relaxation overcounts these
/// eclasses' costs, so the B&B must resolve them.
pub fn find_cse_violations(
    dag: &ExtractionDAG,
    relaxed: &RelaxedSolution,
) -> Vec<(usize, usize)> {
    let mut violations = Vec::new();

    for (ei, edge) in dag.cse_edges.iter().enumerate() {
        let selected_count = edge
            .parent_enodes
            .iter()
            .filter(|&&(pi, ni)| {
                relaxed.enode_selection[pi]
                    .map_or(false, |(tid, ridx)| {
                        dag.eclasses[pi].enodes[ni] == (tid, ridx)
                    })
            })
            .count();

        if selected_count > 1 {
            violations.push((ei, selected_count));
        }
    }

    violations
}

/// Pick the best eclass to branch on from the violation list.
///
/// Heuristic: choose the eclass with the most selected parents (largest
/// CSE overcount). Ties are broken by eclass depth (deeper first — higher
/// index in BFS order means further from root).
///
/// # Panics
/// Panics if `violations` is empty (caller should check first).
pub fn pick_branching_eclass(
    dag: &ExtractionDAG,
    violations: &[(usize, usize)],
) -> usize {
    let &(ei, _) = violations
        .iter()
        .max_by_key(|&&(ei, count)| {
            // Primary: most selected parents. Tiebreak: deeper eclass first
            // (higher child_eclass index = later in BFS = deeper).
            (count, dag.cse_edges[ei].child_eclass)
        })
        .expect("pick_branching_eclass called with empty violations");
    dag.cse_edges[ei].child_eclass
}

/// Result of a B&B search — the selected enodes and total cost.
#[derive(Debug, Clone)]
pub struct Solution {
    /// Selected enode for each eclass (indexed by eclass position in DAG).
    pub enode_selection: Vec<Option<(TableId, RowIndex)>>,
    /// Total optimal cost.
    pub cost: u64,
}

/// Fast path: solve extraction when there are no CSE edges.
///
/// Without CSE coupling, the extraction problem is a pure DAG shortest-path
/// and the greedy DP is globally optimal. No B&B needed.
pub fn solve_dag_shortest_path(
    dag: &ExtractionDAG,
    regraph: &RelatedEGraph,
) -> Solution {
    let relaxed = solve_relaxation(dag, regraph, &BTreeMap::new());
    Solution {
        enode_selection: relaxed.enode_selection,
        cost: relaxed.cost,
    }
}
