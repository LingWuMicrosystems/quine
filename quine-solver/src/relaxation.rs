use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;
use quine_core::common::{RowIndex, Value};
use quine_core::related_egraph::{RelatedEGraph, TableId};

use crate::dag::ExtractionDAG;
use crate::formulation::{constructor_cost, type_is_eclass};

/// CSE ownership decision for an eclass at a B&B node.
///
/// Controls how parent eclasses account for this child eclass's cost
/// when building the combinatorial relaxation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CseDecision {
    /// This eclass's cost is "owned" by `parent_idx` for CSE accounting.
    /// Other parent eclasses do not add this eclass's cost when computing
    /// their own enode costs — only the owning parent does.
    OwnedBy(usize),
    /// Break CSE coupling: each parent independently pays this eclass's
    /// cost. This is the relaxation default and corresponds to "the
    /// child is NOT shared — it appears once per parent in the expression."
    NotShared,
}

/// A fixed decision constraining the solver at a B&B node.
///
/// These accumulate as the B&B tree descends — each branch adds one
/// or more fixed decisions to its parent's map. `selected` and `cse`
/// are independent: an eclass can simultaneously be forced to select
/// a specific enode AND have CSE ownership assigned (nested CSE).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedDecision {
    /// Force a specific enode to be selected in this eclass.
    pub selected: Option<(TableId, RowIndex)>,
    /// CSE ownership decision for this eclass.
    pub cse: Option<CseDecision>,
}

impl Default for FixedDecision {
    fn default() -> Self {
        FixedDecision {
            selected: None,
            cse: None,
        }
    }
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
    if let Some((tid, ridx)) = decision.selected {
        // Forced enode selection — compute its cost with CSE adjustments.
        let c = enode_cost_with_cse(
            dag, regraph, fixed, eclass_idx, tid, ridx, cost, eclass_map,
        );
        (c, Some((tid, ridx)))
    } else {
        // No forced selection. CSE ownership (if any) affects parent-side
        // accounting, not this eclass's own selection. Pick cheapest enode.
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
                let child_cost = match fixed.get(&child_idx).and_then(|d| d.cse.as_ref()) {
                    Some(CseDecision::OwnedBy(owner)) if *owner != parent_idx => 0,
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
///
/// Skips CSE edges whose child eclass already has a fixed CSE decision
/// (NotShared or OwnedBy) — those are already resolved by branching.
pub fn find_cse_violations(
    dag: &ExtractionDAG,
    relaxed: &RelaxedSolution,
    fixed: &BTreeMap<usize, FixedDecision>,
) -> Vec<(usize, usize)> {
    let mut violations = Vec::new();

    for (ei, edge) in dag.cse_edges.iter().enumerate() {
        // Skip if child eclass already has a CSE decision — resolved by branching.
        if fixed
            .get(&edge.child_eclass)
            .and_then(|d| d.cse.as_ref())
            .is_some()
        {
            continue;
        }

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

// ============================================================================
// Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::BTreeMap;
    use quine_core::common::{RowIndex, Value};
    use quine_core::related_egraph::RelatedEGraph;
    use quine_core::table::Row;
    use quine_core::types::{TableDef, Type};
    use crate::dag::{CseEdge, EclassNode, ExtractionDAG};

    /// Create a minimal RelatedEGraph with an "Op" table.
    /// `child_types` lists the types of child columns (key columns).
    /// A value column of eclass type is appended automatically.
    fn make_egraph(child_types: &[Type], cost: u64) -> RelatedEGraph {
        let mut eg = RelatedEGraph::default();
        let mut all_types: Vec<Type> = child_types.to_vec();
        all_types.push(Type::Name("Expr".into())); // value column (eclass-typed)
        eg.add_table(TableDef("Op".into(), all_types.into_boxed_slice(), None));
        eg.set_cost_model("Op".into(), cost);
        eg
    }

    /// Build a 2-eclass tree DAG: root → child (no CSE edges).
    /// Table "Op" has 1 child column. Constructor cost = 5.
    fn make_tree_dag(eg: &mut RelatedEGraph) -> (ExtractionDAG, Value, Value) {
        let root_val = eg.fresh_id();
        let child_val = eg.fresh_id();
        let dummy = eg.fresh_id();

        // Child enode: leaf (dummy child not in DAG — no child cost added)
        eg.insert(0, Row(smallvec::smallvec![dummy]), child_val);
        // Root enode: child = child_val (in DAG → child cost added)
        eg.insert(0, Row(smallvec::smallvec![child_val]), root_val);

        let dag = ExtractionDAG {
            eclasses: vec![
                EclassNode { canonical: root_val, enodes: vec![(0, RowIndex(1))] },
                EclassNode { canonical: child_val, enodes: vec![(0, RowIndex(0))] },
            ],
            root: 0,
            cse_edges: vec![],
        };
        (dag, root_val, child_val)
    }

    /// Build a 4-eclass DAG with 1 CSE edge (design report §8.1 shape):
    ///   A=root=index 0, B=index 1, C=index 2, D=index 3 (leaf)
    ///   Both B and C reference D → CSE edge on D.
    ///   BFS order: root first, leaves last — solver reverses for bottom-up DP.
    ///   All enodes in one table "Op" with 2 child columns (arity 2).
    fn make_cse_dag() -> (RelatedEGraph, ExtractionDAG) {
        let child_types = [
            Type::Name("Expr".into()),
            Type::Name("Expr".into()),
        ];
        let mut eg = make_egraph(&child_types, 5);

        let a_val = eg.fresh_id(); // root
        let b_val = eg.fresh_id();
        let c_val = eg.fresh_id();
        let d_val = eg.fresh_id(); // shared leaf
        let z0 = eg.fresh_id(); // distinct dummy values
        let z1 = eg.fresh_id();
        let z2 = eg.fresh_id();
        let z3 = eg.fresh_id();
        let z4 = eg.fresh_id();

        // Insert in BFS order (root first); RowIndex order matches enode order.
        // A: root enode (children = B, C)
        eg.insert(0, Row(smallvec::smallvec![b_val, c_val]), a_val); // RowIndex(0)
        // B: child0 = D, child1 = z2
        eg.insert(0, Row(smallvec::smallvec![d_val, z2]), b_val); // RowIndex(1)
        // C: child0 = D, child1 = z3
        eg.insert(0, Row(smallvec::smallvec![d_val, z3]), c_val); // RowIndex(2)
        // D: leaf (2 alternatives, children not in DAG)
        eg.insert(0, Row(smallvec::smallvec![z0, z1]), d_val); // RowIndex(3)
        eg.insert(0, Row(smallvec::smallvec![z4, z4]), d_val); // RowIndex(4)

        let dag = ExtractionDAG {
            eclasses: vec![
                // BFS order: root first (idx 0), leaves last (idx 3)
                EclassNode { canonical: a_val, enodes: vec![(0, RowIndex(0))] },       // idx 0: root A
                EclassNode { canonical: b_val, enodes: vec![(0, RowIndex(1))] },       // idx 1: B
                EclassNode { canonical: c_val, enodes: vec![(0, RowIndex(2))] },       // idx 2: C
                EclassNode { canonical: d_val, enodes: vec![(0, RowIndex(3)), (0, RowIndex(4))] }, // idx 3: leaf D
            ],
            root: 0,
            cse_edges: vec![CseEdge {
                child_eclass: 3, // D is at index 3
                parent_enodes: vec![(1, 0), (2, 0)], // B (idx 1, enode 0), C (idx 2, enode 0)
            }],
        };
        (eg, dag)
    }

    // ------------------------------------------------------------------
    // AC-1: solve_relaxation
    // ------------------------------------------------------------------

    /// 2-eclass tree DAG: root → child. No CSE — relaxation equals optimum.
    ///
    /// Given: a 2-eclass tree DAG where root enode references child eclass
    /// When:  solve_relaxation is called with no fixed decisions
    /// Then:  each eclass picks its cheapest enode, root cost = child cost + constructor
    #[test]
    fn test_solve_relaxation_tree() {
        let child_types = [Type::Name("Expr".into())];
        let mut eg = make_egraph(&child_types, 5);
        let (dag, _root, _child) = make_tree_dag(&mut eg);
        let relaxed = solve_relaxation(&dag, &eg, &BTreeMap::new());
        // child cost = 5 (constructor only, child not in DAG)
        // root cost = 5 + child_cost = 10
        assert_eq!(relaxed.cost, 10);
        assert_eq!(relaxed.enode_selection[0], Some((0, RowIndex(1))));
        assert_eq!(relaxed.enode_selection[1], Some((0, RowIndex(0))));
    }

    /// CSE DAG with child OwnedBy a specific parent — non-owner pays 0.
    ///
    /// Given: a DAG where eclass D is shared by B and C (1 CSE edge)
    /// When:  D is fixed as OwnedBy(B) — B owns D's cost
    /// Then:  B pays D's cost (10), C pays 0 for D (5), root cost = 20
    #[test]
    fn test_solve_relaxation_ownedby() {
        let (eg, dag) = make_cse_dag();

        // BFS order: A=0, B=1, C=2, D=3.
        // Fix D (index 3) as OwnedBy B (parent index 1)
        let mut fixed = BTreeMap::new();
        fixed.insert(3, FixedDecision { selected: None, cse: Some(CseDecision::OwnedBy(1)) });

        let relaxed = solve_relaxation(&dag, &eg, &fixed);

        // D picks cheapest of 2 enodes (both cost 5): D cost = 5
        // B: cost = 5 + cost[D] = 5+5=10 (B is owner, pays D's cost)
        // C: cost = 5 + 0 = 5 (not owner, D's cost is 0)
        // A: cost = 5 + cost[B] + cost[C] = 5+10+5 = 20
        assert_eq!(relaxed.cost, 20);
    }

    /// CSE DAG with child NotShared — all parents pay child cost independently.
    ///
    /// Given: a DAG where eclass D is shared by B and C (1 CSE edge)
    /// When:  D is fixed as NotShared (CSE coupling broken)
    /// Then:  B and C each pay D's cost independently, root cost = 25
    #[test]
    fn test_solve_relaxation_notshared() {
        let (eg, dag) = make_cse_dag();

        // BFS order: A=0, B=1, C=2, D=3.
        // Fix D (index 3) as NotShared
        let mut fixed = BTreeMap::new();
        fixed.insert(3, FixedDecision { selected: None, cse: Some(CseDecision::NotShared) });

        let relaxed = solve_relaxation(&dag, &eg, &fixed);

        // D: cost=5 (picks cheapest)
        // B: cost = 5 + cost[D] = 5+5=10
        // C: cost = 5 + cost[D] = 5+5=10
        // A: cost = 5 + 10 + 10 = 25
        assert_eq!(relaxed.cost, 25);
    }

    /// Fixed decision Selected forces a specific enode in an eclass.
    ///
    /// Given: a 2-eclass tree DAG
    /// When:  root eclass is fixed to Selected(0, RowIndex(1))
    /// Then:  relaxation selects the forced enode for root regardless of cost
    #[test]
    fn test_solve_relaxation_selected() {
        let child_types = [Type::Name("Expr".into())];
        let mut eg = make_egraph(&child_types, 5);
        let (dag, _root_val, _child_val) = make_tree_dag(&mut eg);

        // Fix root to select its enode (RowIndex(1))
        let mut fixed = BTreeMap::new();
        fixed.insert(0, FixedDecision {
            selected: Some((0, RowIndex(1))),
            cse: None,
        });

        let relaxed = solve_relaxation(&dag, &eg, &fixed);
        assert_eq!(relaxed.enode_selection[0], Some((0, RowIndex(1))));
    }

    // ------------------------------------------------------------------
    // AC-1: find_cse_violations
    // ------------------------------------------------------------------

    /// CSE violation detected when >1 parent selects enode referencing shared child.
    ///
    /// Given: a DAG with CSE edge where D is referenced by both B and C
    /// When:  relaxation selects B's and C's enodes pointing to D
    /// Then:  find_cse_violations returns the CSE edge with 2+ selected parents
    #[test]
    fn test_find_cse_violations_found() {
        let (eg, dag) = make_cse_dag();

        // Relaxation without fixed decisions: D is selected, both B and C
        // reference D via selected enodes → violation.
        let relaxed = solve_relaxation(&dag, &eg, &BTreeMap::new());
        let violations = find_cse_violations(&dag, &relaxed, &BTreeMap::new());
        assert!(!violations.is_empty());
        assert_eq!(violations[0].0, 0); // edge index
        assert!(violations[0].1 > 1); // 2+ parents selected
    }

    /// No CSE violations in a tree-structured DAG (no sharing).
    ///
    /// Given: a tree DAG with no CSE edges
    /// When:  find_cse_violations checks all CSE edges
    /// Then:  violations list is empty
    #[test]
    fn test_find_cse_violations_none_when_tree() {
        // Tree DAG has no CSE edges → no violations possible.
        let child_types = [Type::Name("Expr".into())];
        let mut eg = make_egraph(&child_types, 5);
        let (tree_dag, _, _) = make_tree_dag(&mut eg);
        let relaxed = solve_relaxation(&tree_dag, &eg, &BTreeMap::new());
        let violations = find_cse_violations(&tree_dag, &relaxed, &BTreeMap::new());
        assert!(violations.is_empty());
    }

    // ------------------------------------------------------------------
    // AC-1: pick_branching_eclass
    // ------------------------------------------------------------------

    /// Branching heuristic picks the eclass with the most selected parents.
    ///
    /// Given: a CSE DAG with one violated edge (D, 2 parents)
    /// When:  pick_branching_eclass is called with the violations list
    /// Then:  returns the child_eclass index of the most-violated CSE edge (D)
    #[test]
    fn test_pick_branching_eclass() {
        let (eg, dag) = make_cse_dag();
        let relaxed = solve_relaxation(&dag, &eg, &BTreeMap::new());
        let violations = find_cse_violations(&dag, &relaxed, &BTreeMap::new());

        let picked = pick_branching_eclass(&dag, &violations);
        // Should pick the child_eclass of the violated CSE edge (D = index 3)
        assert_eq!(picked, 3);
    }

    // ------------------------------------------------------------------
    // AC-1: solve_dag_shortest_path
    // ------------------------------------------------------------------

    /// Fast path: tree DAG without CSE yields optimal solution via DP alone.
    ///
    /// Given: a tree DAG (no CSE edges)
    /// When:  solve_dag_shortest_path is called
    /// Then:  returns a Solution with cost = relaxation optimum (10 = 5+5)
    #[test]
    fn test_solve_dag_shortest_path_tree() {
        let child_types = [Type::Name("Expr".into())];
        let mut eg = make_egraph(&child_types, 5);
        let (dag, _, _) = make_tree_dag(&mut eg);
        let solution = solve_dag_shortest_path(&dag, &eg);
        assert_eq!(solution.cost, 10);
        assert_eq!(solution.enode_selection.len(), 2);
    }

    // ------------------------------------------------------------------
    // CSE table sanity check
    // ------------------------------------------------------------------

    /// Single eclass in 2-child table — relaxation picks its only enode.
    ///
    /// Given: a DAG with 1 eclass, 1 enode in a 2-child table (arity=2)
    /// When:  solve_relaxation is called
    /// Then:  the single enode is selected, cost equals constructor cost (5)
    #[test]
    fn test_cse_table_single_eclass() {
        let child_types = [Type::Name("Expr".into()), Type::Name("Expr".into())];
        let mut eg = make_egraph(&child_types, 5);
        let dummy = eg.fresh_id();
        let val = eg.fresh_id();
        eg.insert(0, Row(smallvec::smallvec![dummy, dummy]), val);

        let dag = ExtractionDAG {
            eclasses: vec![
                EclassNode { canonical: val, enodes: vec![(0, RowIndex(0))] },
            ],
            root: 0,
            cse_edges: vec![],
        };
        let relaxed = solve_relaxation(&dag, &eg, &BTreeMap::new());
        assert_eq!(relaxed.cost, 5);
        assert_eq!(relaxed.enode_selection[0], Some((0, RowIndex(0))));
    }

    /// Two-eclass chain in 2-child table — bottom-up DP works correctly.
    ///
    /// Given: a 2-eclass chain DAG (root → child) with arity-2 table
    /// When:  solve_relaxation processes in reverse BFS order
    /// Then:  leaf cost = 5, root cost = 5 + leaf_cost = 10
    #[test]
    fn test_cse_table_two_eclass_chain() {
        // Two eclasses in a chain: root → child (both in 2-child table).
        let child_types = [Type::Name("Expr".into()), Type::Name("Expr".into())];
        let mut eg = make_egraph(&child_types, 5);
        let d0 = eg.fresh_id();
        let child_val = eg.fresh_id();
        let root_val = eg.fresh_id();
        // Child (leaf — children not in DAG)
        eg.insert(0, Row(smallvec::smallvec![d0, d0]), child_val);
        // Root (child = child_val, other child not in DAG)
        eg.insert(0, Row(smallvec::smallvec![child_val, d0]), root_val);

        let dag = ExtractionDAG {
            eclasses: vec![
                EclassNode { canonical: root_val, enodes: vec![(0, RowIndex(1))] },
                EclassNode { canonical: child_val, enodes: vec![(0, RowIndex(0))] },
            ],
            root: 0,
            cse_edges: vec![],
        };
        let relaxed = solve_relaxation(&dag, &eg, &BTreeMap::new());
        // child cost = 5, root cost = 5 + child_cost = 10
        assert_eq!(relaxed.cost, 10);
    }

    // ------------------------------------------------------------------
    // #16 regression: FixedDecision merge (selected + cse coexist)
    // ------------------------------------------------------------------

    /// FixedDecision struct allows Selected + CseDecision to coexist (#16 fix).
    ///
    /// Given: a FixedDecision with both selected and cse fields set
    /// When:  the struct is constructed with both Some values
    /// Then:  both fields retain their values (no mutual exclusion)
    #[test]
    fn test_fixeddecision_merge() {
        // Verify struct allows selected + cse simultaneously
        let fd = FixedDecision {
            selected: Some((0, RowIndex(0))),
            cse: Some(CseDecision::OwnedBy(1)),
        };
        assert_eq!(fd.selected, Some((0, RowIndex(0))));
        assert_eq!(fd.cse, Some(CseDecision::OwnedBy(1)));

        // Default: both None
        let fd2 = FixedDecision::default();
        assert_eq!(fd2.selected, None);
        assert_eq!(fd2.cse, None);
    }
}
