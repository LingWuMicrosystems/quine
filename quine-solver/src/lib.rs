#![no_std]
extern crate alloc;

pub mod dag;
pub mod formulation;
pub mod relaxation;
pub mod solver;

pub use dag::ExtractionDAG;

use alloc::format;
use alloc::string::String;
use alloc::vec;
use quine_core::common::Value;
use quine_core::related_egraph::RelatedEGraph;
use quine_core::term::Term;

use crate::dag::build_extraction_dag;
use crate::relaxation::{solve_dag_shortest_path, Solution};
use crate::solver::{branch_and_bound, extract_solution_from_dag, BnBNode, BnBStats};

/// Configuration for the ILP extraction solver.
#[derive(Debug, Clone)]
pub struct ILPConfig {
    /// Maximum number of eclasses before falling back to greedy.
    pub max_eclasses: usize,
    /// Maximum number of CSE edges before switching to heuristic branching.
    pub max_cse_edges_warning: usize,
    /// Time limit for B&B search (None = no limit).
    pub time_limit_ms: Option<u64>,
}

impl Default for ILPConfig {
    fn default() -> Self {
        Self {
            max_eclasses: 500,
            max_cse_edges_warning: 50,
            time_limit_ms: Some(1000),
        }
    }
}

/// Result of ILP extraction.
#[derive(Debug, Clone)]
pub struct ILPResult {
    /// The extracted term. None if extraction failed.
    pub term: Option<Term>,
    /// Whether the global optimum was found.
    pub optimal: bool,
    /// Number of B&B nodes explored.
    pub nodes_explored: u64,
    /// The objective value (total cost).
    pub cost: u64,
    /// Number of CSE edges in the extraction DAG.
    pub cse_edge_count: usize,
    /// Warning message (e.g., CSE edge count exceeds threshold).
    pub warning: Option<String>,
}

/// Main entry point: extract the cheapest expression for `root_eclass`
/// using ILP-based Branch-and-Bound with Combinatorial Relaxation (B&B-CR).
///
/// ## Algorithm
/// 1. Build an `ExtractionDAG` from the e-graph reachable from `root_eclass`.
/// 2. If the DAG has no CSE edges: greedy DP is optimal — fast path.
/// 3. Otherwise: B&B-CR search for the globally optimal extraction.
///
/// ## Fallback
/// Returns a greedy (potentially suboptimal) result if the e-graph exceeds
/// `config.max_eclasses`. The `optimal` field is `false` in this case.
pub fn ilp_extract(
    regraph: &RelatedEGraph,
    root_eclass: Value,
    config: &ILPConfig,
) -> ILPResult {
    let dag = build_extraction_dag(regraph, root_eclass);
    let cse_edge_count = dag.cse_edges.len();

    // Build warning if CSE edges exceed threshold.
    let warning = if cse_edge_count > config.max_cse_edges_warning {
        Some(format!(
            "warning: {} CSE edges exceeds threshold ({}) — extraction may be slow",
            cse_edge_count, config.max_cse_edges_warning
        ))
    } else {
        None
    };

    // Convert time_limit_ms to node budget (~1000 nodes/ms heuristic for no_std).
    let max_nodes = config.time_limit_ms.map(|ms| ms * 1000);

    // Empty e-graph — nothing to extract.
    if dag.eclasses.is_empty() {
        return ILPResult {
            term: None,
            optimal: false,
            nodes_explored: 0,
            cost: 0,
            cse_edge_count,
            warning,
        };
    }

    // Fallback: e-graph too large for optimal search.
    if dag.eclasses.len() > config.max_eclasses {
        let solution = solve_dag_shortest_path(&dag, regraph);
        let term = extract_solution_from_dag(&dag, regraph, &solution);
        return ILPResult {
            term: Some(term),
            optimal: false,
            nodes_explored: 0,
            cost: solution.cost,
            cse_edge_count,
            warning,
        };
    }

    // No CSE edges: greedy DP is globally optimal — fast path.
    if dag.cse_edges.is_empty() {
        let solution = solve_dag_shortest_path(&dag, regraph);
        let term = extract_solution_from_dag(&dag, regraph, &solution);
        return ILPResult {
            term: Some(term),
            optimal: true,
            nodes_explored: 0,
            cost: solution.cost,
            cse_edge_count,
            warning,
        };
    }

    // B&B-CR: search for the global optimum.
    let mut best = Solution {
        enode_selection: vec![None; dag.eclasses.len()],
        cost: u64::MAX,
    };
    let root_node = BnBNode {
        fixed: alloc::collections::BTreeMap::new(),
    };
    let mut stats = BnBStats::default();

    branch_and_bound(&dag, regraph, &root_node, &mut best, &mut stats, max_nodes);

    let optimal = best.cost < u64::MAX;
    let term = if best.cost < u64::MAX {
        Some(extract_solution_from_dag(&dag, regraph, &best))
    } else {
        None
    };

    ILPResult {
        term,
        optimal,
        nodes_explored: stats.nodes_explored,
        cost: best.cost,
        cse_edge_count,
        warning,
    }
}
