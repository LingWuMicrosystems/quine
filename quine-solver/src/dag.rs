use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use quine_core::common::{RowIndex, Value};
use quine_core::related_egraph::{RelatedEGraph, TableId};

use crate::formulation::type_is_eclass;

/// A single eclass node in the extraction DAG.
///
/// Each eclass contains one or more enodes (alternative ways to build
/// the same value). Enodes are stored as `(TableId, RowIndex)` pairs
/// for lookup into `RelatedEGraph` tables.
#[derive(Debug, Clone)]
pub struct EclassNode {
    pub canonical: Value,
    pub enodes: Vec<(TableId, RowIndex)>,
}

/// A CSE (common subexpression) edge: an eclass referenced by multiple
/// parent enodes in the extraction DAG.
///
/// These edges are the source of NP-hardness — without them, the
/// extraction problem is a simple DAG shortest-path.
#[derive(Debug, Clone)]
pub struct CseEdge {
    /// Index into `ExtractionDAG::eclasses` for the shared child.
    pub child_eclass: usize,
    /// `(eclass_index, enode_index)` pairs for each parent enode
    /// that references this child.
    pub parent_enodes: Vec<(usize, usize)>,
}

/// A snapshot of the e-graph reachable from a root eclass, organized
/// as a DAG for the ILP solver.
///
/// Eclasses are stored in BFS discovery order from the root (root first,
/// leaves last). For bottom-up DP, the solver iterates in reverse:
/// `for idx in (0..eclasses.len()).rev()`.
#[derive(Debug, Clone)]
pub struct ExtractionDAG {
    /// All eclasses reachable from the root, in BFS order (root first).
    pub eclasses: Vec<EclassNode>,
    /// Index into `eclasses` for the root node.
    pub root: usize,
    /// Eclasses referenced by more than one parent enode.
    pub cse_edges: Vec<CseEdge>,
}

/// Build an `ExtractionDAG` from a `RelatedEGraph` by traversing
/// reachable eclasses from `root_eclass`.
///
/// The traversal is BFS from the root. Eclasses are stored in BFS
/// discovery order (root first, leaves last). Reverse iteration gives
/// leaves-first order for bottom-up DP in the solver.
///
/// CSE edges are identified in a second pass: any child eclass
/// referenced by >1 parent enode is a CSE coupling point.
///
/// # Cycles
///
/// E-graphs may contain cycles (e.g., `x + 0 => x` creates an enode
/// that references its own eclass). The BFS handles this via the
/// visited set — a child already in the DAG is not re-enqueued.
/// Self-loop edges are still recorded for correctness. The solver
/// naturally ignores cyclic paths since all costs are non-negative
/// (cycles can only increase the objective).
pub fn build_extraction_dag(
    regraph: &RelatedEGraph,
    root_eclass: Value,
) -> ExtractionDAG {
    let root = regraph.find(root_eclass);

    // BFS state
    let mut eclasses: Vec<EclassNode> = Vec::new();
    let mut eclass_map: BTreeMap<Value, usize> = BTreeMap::new();

    // Queue for BFS. Use Vec with head index (cheaper than VecDeque
    // and avoids needing alloc::collections::VecDeque).
    let mut queue: Vec<Value> = Vec::new();

    // Visited set — BTreeMap is in alloc, HashMap is not.
    let mut visited: BTreeMap<Value, ()> = BTreeMap::new();

    // Track child → parent relationships for CSE detection.
    // Key: canonical child Value. Value: list of (parent_eclass_index, enode_index).
    let mut child_parents: BTreeMap<Value, Vec<(usize, usize)>> = BTreeMap::new();

    // Initialize with root
    queue.push(root);
    visited.insert(root, ());

    let mut head: usize = 0;

    // --- Pass 1: BFS to build eclass list ---

    while head < queue.len() {
        let eclass_val = queue[head];
        head += 1;

        let idx = eclasses.len();
        eclass_map.insert(eclass_val, idx);

        // Gather all enodes for this eclass
        let enodes: Vec<(TableId, RowIndex)> =
            regraph.eclass_enodes(eclass_val).into_iter().collect();

        // Enumerate children, record parent→child edges, enqueue new eclasses
        for (enode_i, &(tid, ridx)) in enodes.iter().enumerate() {
            let table = regraph.get_table(tid);
            let row = table.get_all_row(ridx);

            for col in 0..table.arity() {
                let col_type = &table.table_def.1[col];
                if type_is_eclass(col_type) {
                    let child_val = row.0[col];
                    let child_canon = regraph.find(child_val);

                    // Record parent → child for CSE detection.
                    // Dedup: skip if this (parent, enode) pair already
                    // recorded (e.g., `Add(A, A)` references A twice from
                    // the same enode — not a CSE edge, just repeated child).
                    // Note: e-graphs may contain cycles (e.g., `x + 0 => x`
                    // puts an enode referencing its own eclass). Self-loops
                    // are recorded as edges but the visited set prevents
                    // re-enqueueing. The solver naturally avoids cyclic
                    // paths: all costs are non-negative, so cycles never
                    // improve the objective.
                    let parents = child_parents.entry(child_canon).or_default();
                    if !parents.contains(&(idx, enode_i)) {
                        parents.push((idx, enode_i));
                    }

                    // Enqueue if not yet visited (prevents infinite loops
                    // on cycles; child is already in the DAG)
                    if !visited.contains_key(&child_canon) {
                        visited.insert(child_canon, ());
                        queue.push(child_canon);
                    }
                }
            }
        }

        eclasses.push(EclassNode {
            canonical: eclass_val,
            enodes,
        });
    }

    // --- Pass 2: Identify CSE edges ---

    let mut cse_edges: Vec<CseEdge> = Vec::new();
    for (child_val, parents) in &child_parents {
        if parents.len() > 1 {
            if let Some(&child_idx) = eclass_map.get(child_val) {
                cse_edges.push(CseEdge {
                    child_eclass: child_idx,
                    parent_enodes: parents.clone(),
                });
            }
        }
    }

    ExtractionDAG {
        eclasses,
        root: eclass_map[&root],
        cse_edges,
    }
}
