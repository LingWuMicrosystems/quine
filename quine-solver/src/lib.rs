#![no_std]
extern crate alloc;

pub mod dag;
pub mod formulation;

pub use dag::ExtractionDAG;

use quine_core::common::Value;
use quine_core::related_egraph::RelatedEGraph;
use quine_frontend::term::Term;

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
}

/// Main entry point: extract the cheapest expression for `root_eclass`
/// using ILP-based optimization.
///
/// Falls back to greedy extraction if the e-graph exceeds `config.max_eclasses`
/// or if the solver times out.
///
/// STUB: Returns empty result. Full implementation in 07-02.
pub fn ilp_extract(
    _regraph: &RelatedEGraph,
    _root_eclass: Value,
    _config: &ILPConfig,
) -> ILPResult {
    ILPResult {
        term: None,
        optimal: false,
        nodes_explored: 0,
        cost: 0,
    }
}
