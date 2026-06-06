---
phase: 07-ilp-solver-implementation
plan: 01
subsystem: solver
tags: [rust, e-graph, ilp, branch-and-bound, extraction, dag, no_std]

requires:
  - phase: 06-ilp-solver-design
    provides: ILP-DESIGN-REPORT.md — B&B-CR algorithm design, ILP formulation, solver architecture
provides:
  - quine-solver crate (no_std + alloc) — public API types, ExtractionDAG builder, type predicates
  - Foundation for Plans 07-02 (solver algorithm) and 07-03 (tests)
affects:
  - phase: 07-ilp-solver-implementation (plans 02, 03)
  - phase: 08-solver-integration
  - phase: 09-enhanced-extraction

tech-stack:
  added: []
  patterns:
    - All library crates share #![no_std] + extern crate alloc stance
    - ExtractionDAG snapshots RelatedEGraph state via public APIs only — zero core changes
    - BTreeMap from alloc::collections replaces HashMap for no_std compatibility
    - BFS with Vec-as-queue pattern (avoids alloc::collections::VecDeque dependency)

key-files:
  created:
    - quine-solver/Cargo.toml
    - quine-solver/src/lib.rs
    - quine-solver/src/formulation.rs
    - quine-solver/src/dag.rs
  modified:
    - Cargo.toml (workspace members + workspace dependency)

key-decisions:
  - "ExtractionDAG stores eclasses in BFS order (root first, leaves last); solver iterates in reverse for bottom-up DP"
  - "Cycles handled via visited set, not panics — e-graphs can have self-referencing enodes (e.g., x+0 => x)"

patterns-established:
  - "BTreeMap for no_std maps (HashMap not in alloc)"
  - "Vec with head-index as BFS queue (no VecDeque dependency needed)"
  - "All solver integration through existing public RelatedEGraph APIs — no internal changes"

duration: ~15min
started: 2026-06-06T00:00:00+08:00
completed: 2026-06-06T00:15:00+08:00
description: "Created quine-solver crate with ILPConfig, ILPResult, ExtractionDAG builder, and type predicates — foundation for B&B-CR solver"
type: Summary
about: "quine"
---

# Phase 7 Plan 01: Crate Scaffold + Data Layer Summary

**Created the `quine-solver` crate (no_std + alloc) with public API types, type predicate helpers, and the ExtractionDAG builder that snapshots e-graph state for the ILP solver.**

## Performance

| Metric | Value |
|--------|-------|
| Duration | ~15min |
| Started | 2026-06-06 |
| Completed | 2026-06-06 |
| Tasks | 3 completed |
| Files created | 4 |
| Files modified | 1 |
| Lines of code | 290 (lib: 65, dag: 165, formulation: 52, Cargo.toml: 8) |

## Acceptance Criteria Results

| Criterion | Status | Notes |
|-----------|--------|-------|
| AC-1: Crate compiles as no_std + alloc | ✅ Pass | `cargo build -p quine-solver` passes; `#![no_std]` + `extern crate alloc` confirmed |
| AC-2: ExtractionDAG built from RelatedEGraph | ✅ Pass | BFS from root with eclass_map, enode enumeration, CSE detection (>1 parent) |
| AC-3: type_is_eclass matches existing logic | ✅ Pass | Uses `matches!(ty, Type::Name(_) \| Type::Base(BaseType::Id))` — identical to materialize_cheapest_inner |
| AC-4: Workspace builds with new member | ✅ Pass | All library crates (quine-core, quine-frontend, quine-solver) compile clean |
| AC-5: constructor_cost reads from cost_models | ✅ Pass | Delegates to `regraph.get_constructor_cost()` — returns 0 for unknown |

## Accomplishments

- Created `quine-solver` crate following the exact no_std + alloc pattern from quine-core and quine-frontend
- Defined `ILPConfig` (3 fields: max_eclasses, max_cse_edges_warning, time_limit_ms) with `Default` impl per design report §6.3
- Defined `ILPResult` (4 fields: term, optimal, nodes_explored, cost) — all solver outputs
- Implemented `build_extraction_dag()` — BFS traversal, topological ordering, CSE edge detection
- Implemented `type_is_eclass()`, `constructor_cost()`, `atom_from_value()` — shared helpers for DAG construction and solver
- Wired `quine-solver` into workspace (members + workspace.dependencies)
- Zero changes to `quine-core` or `quine-frontend` — all integration via public APIs

## Files Created/Modified

| File | Change | Purpose |
|------|--------|---------|
| `quine-solver/Cargo.toml` | Created | Crate manifest (depends on quine-core, quine-frontend) |
| `quine-solver/src/lib.rs` | Created | `#![no_std]`, ILPConfig, ILPResult, ilp_extract() stub |
| `quine-solver/src/formulation.rs` | Created | type_is_eclass(), constructor_cost(), atom_from_value() |
| `quine-solver/src/dag.rs` | Created | ExtractionDAG, EclassNode, CseEdge, build_extraction_dag() |
| `Cargo.toml` | Modified | Added quine-solver to workspace members and dependencies |

## Decisions Made

| Decision | Rationale | Impact |
|----------|-----------|--------|
| BTreeMap instead of HashMap for eclass_map | HashMap not available in alloc; BTreeMap is in alloc::collections | Slight perf difference (O(log n) vs O(1)), negligible for typical e-graph sizes |
| Vec with head-index as BFS queue | Avoids depending on alloc::collections::VecDeque | Simpler, zero additional dependencies |
| Cycles handled gracefully (visited set) not via panic | E-graphs can have self-referencing enodes (e.g., `x + 0 => x`) | Correct for real e-graph states; solver naturally ignores cyclic paths (non-negative costs) |

## Deviations from Plan

### Summary

| Type | Count | Impact |
|------|-------|--------|
| Auto-fixed | 1 | Essential fix — prevents panic on valid e-graph states |

**Total impact:** One spec correction. Plan assumed acyclic DAG; e-graphs can have cycles. Fix removes `assert_ne!` and uses visited set (already in place).

### Auto-fixed Issues

**1. Spec: Cycle handling in ExtractionDAG**
- **Found during:** User review of dag.rs
- **Issue:** PLAN assumed e-graph extraction DAG is always acyclic. In practice, rules like `x + 0 => x` create self-referencing enodes (an eclass containing an enode that references itself as a child).
- **Fix:** Removed `assert_ne!` for self-loops. The BFS visited set already handles cycles correctly (visited children are not re-enqueued). Self-loop edges are still recorded in the DAG for correctness. The solver naturally avoids cyclic paths since all costs are non-negative — cycles can only increase the objective.
- **Files:** `quine-solver/src/dag.rs` (removed assert, updated doc comments)
- **Verification:** `cargo build -p quine-solver` passes

## Issues Encountered

| Issue | Resolution |
|-------|------------|
| quine-cli linker error (`-liconv` not found) on workspace build | Pre-existing macOS SDK issue; library crates (quine-solver, quine-core, quine-frontend) all build clean. Verified with targeted `cargo build -p` commands. |

## Next Phase Readiness

**Ready:**
- `quine-solver` crate compiles and is wired into workspace
- `ExtractionDAG` builder ready for solver to consume
- All public API types defined — solver can implement against them
- `formulation.rs` helpers available for both DAG construction and solver

**Concerns:**
- `BaseType::Str` decoding in `atom_from_value` emits raw `Atom::U64` (no interner access from solver crate). This is acceptable for extraction output but may need interner wiring in Phase 8 if Str display is required.

**Blockers:**
- None

---
*Phase: 07-ilp-solver-implementation, Plan: 01*
*Completed: 2026-06-06*
