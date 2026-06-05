---
phase: 06-ilp-solver-design
plan: 01
subsystem: core,frontend,solver
tags: [rust, e-graph, ilp, branch-and-bound, combinatorial-relaxation, cse, extraction, design]

requires:
  - phase: 05-expression-extraction
    provides: evaluate_expr, materialize_cheapest, cost_select — current greedy extraction system
provides:
  - ILP-DESIGN-REPORT.md — comprehensive solver design blueprint for Phases 7-9
  - B&B-CR algorithm with full pseudocode
  - ILP formulation mapped to Quine types (Value, TableId, RowIndex)
  - Solver architecture: new quine-solver crate (no_std + alloc)
  - Two worked examples comparing ILP vs greedy extraction
affects:
  - phase: 07-ilp-solver-implementation
  - phase: 08-solver-integration
  - phase: 09-enhanced-extraction

tech-stack:
  added: []
  patterns:
    - Branch-and-Bound with Combinatorial Relaxation (B&B-CR): DAG shortest-path relaxation + branch on CSE ownership
    - ILP variables x_{e,n} and y_e mapped directly to Quine's Value, TableId, RowIndex
    - extract optimal <expr> DSL syntax for ILP extraction (greedy remains default for extract <expr>)
    - All library crates (quine-core, quine-frontend, quine-solver) share no_std + alloc stance

key-files:
  created:
    - .paul/phases/06-ilp-solver-design/ILP-DESIGN-REPORT.md
  modified:
    - .paul/STATE.md

key-decisions:
  - "Algorithm: Branch-and-Bound with Combinatorial Relaxation — exploits DAG structure, no floating-point, 400-600 lines"
  - "Crate: new quine-solver (no_std + alloc) — consistent with quine-core/quine-frontend"
  - "Syntax: extract optimal <expr> — natural DSL extension, no CLI flags"
  - "Zero external solver dependencies — all custom implementation"
  - "ILP extraction is opt-in; greedy extract <expr> remains default/unchanged"
  - "Extraction DAG built as snapshot from RelatedEGraph public APIs — no core changes needed"

patterns-established:
  - "Design phases produce a single comprehensive report, not scattered documents"
  - "Solver exploits problem structure (DAG, GUB, sparse CSE coupling) rather than general ILP"
  - "All integration through existing public APIs — no RelatedEGraph internal changes"

duration: ~1h
started: 2026-06-05T00:00:00+08:00
completed: 2026-06-05T01:00:00+08:00
---

# Phase 6 Plan 01: ILP Solver Design Report Summary

**Comprehensive ILP solver design: custom B&B-CR algorithm, full ILP formulation mapped to Quine types, solver architecture, and worked examples — providing the blueprint for Phases 7-9 implementation.**

## Performance

| Metric | Value |
|--------|-------|
| Duration | ~1h |
| Tasks | 3/3 completed |
| Files created | 2 (ILP-DESIGN-REPORT.md, 06-01-SUMMARY.md) |
| Source files modified | 0 (pure design phase) |
| User corrections applied | 2 (no_std + alloc, extract optimal syntax) |

## Acceptance Criteria Results

| Criterion | Status | Notes |
|-----------|--------|-------|
| AC-1: Custom solver algorithm designed | Pass | 5 approaches evaluated (B&B+Simplex, Greedy+Local Search, Lagrangian, B&B-CR, Min-Cost Flow); comparison table with 6 criteria; B&B-CR recommended with ~50 lines of pseudocode |
| AC-2: ILP formulation with Quine type mapping | Pass | Variables (x_{e,n}, y_e) mapped to Value/(TableId,RowIndex); 4 constraint types with exploitation notes; objective with CSE explanation; solution extraction pseudocode; complexity analysis |
| AC-3: Solver architecture specified | Pass | New quine-solver crate (no_std + alloc); 5-module structure; public API (ILPConfig, ILPResult, ilp_extract()); 6 integration points via existing public APIs; memory strategy |
| AC-4: no_std constraint addressed | Pass | quine-core unchanged; solver is no_std + alloc (uses alloc::collections::{BinaryHeap, BTreeMap}, alloc::vec::Vec); all library crates share no_std stance |
| AC-5: Worked examples comparing ILP vs greedy | Pass | Example 1: CSE double-counting (greedy 22 vs ILP 21, ~4.5% improvement); Example 2: Square optimization with CSE (greedy 38 vs ILP 37, ~2.6% improvement); both with concrete .quine DSL |

## Verification Results

```
git diff --stat: 4 files changed in .paul/ only — zero source code changes
Report structure: all 10 sections present with substantive content
Report is self-contained: readable without prior phase context
Zero external solver dependencies stated explicitly
Algorithm pseudocode: concrete enough to guide Phase 7 implementation
```

## Accomplishments

- **Algorithm evaluation and selection** — Evaluated 5 algorithmic approaches (B&B+Simplex, Greedy+Local Search, Lagrangian Relaxation, B&B-CR, Min-Cost Flow) against 6 criteria. Selected Branch-and-Bound with Combinatorial Relaxation — exploits the DAG structure, no floating-point, 400-600 lines estimated.
- **ILP formulation with Quine mapping** — Defined complete 0-1 ILP: decision variables (x_{e,n}, y_e) mapped to Value/(TableId,RowIndex), 4 constraint types with exploitation notes for the custom solver, objective function with CSE explanation, solution extraction pseudocode, and complexity analysis.
- **Solver architecture** — New quine-solver crate (no_std + alloc), 5-module structure, public API (ILPConfig, ILPResult, ilp_extract()), all integration through existing RelatedEGraph public APIs.
- **Worked examples** — Two concrete examples with .quine DSL, e-graph state, ILP variable assignments, and step-by-step B&B-CR solver walkthroughs demonstrating CSE cost savings.
- **Phase 7 handoff** — File list, module structure, AC suggestions, testing strategy recommendations — Phase 7 planner can start implementing immediately.

## Files Created/Modified

| File | Change | Purpose |
|------|--------|---------|
| `.paul/phases/06-ilp-solver-design/ILP-DESIGN-REPORT.md` | Created (~1415 lines) | Complete 10-section ILP solver design report |
| `.paul/STATE.md` | Modified | Loop position advanced: PLAN → APPLY → UNIFY; decisions recorded |
| `.paul/phases/06-ilp-solver-design/06-01-SUMMARY.md` | Created | This summary |

## Decisions Made

| Decision | Rationale | Impact |
|----------|-----------|--------|
| B&B-CR algorithm | Exploits DAG structure; relaxation is O(⎪E⎪) DAG shortest-path; no floating-point; branches on CSE ownership | Phase 7 implements ~500 lines, not 1000+ for simplex |
| quine-solver crate (no_std + alloc) | Consistent with quine-core/quine-frontend; alloc provides all needed collections (BinaryHeap, BTreeMap, Vec) | No std dependency anywhere in library crates |
| `extract optimal <expr>` DSL syntax | Natural extension of existing `extract <expr>`; no CLI flags; discoverable | Minimal parser change; backward compatible |
| Zero external solver deps | Project directive; problem is small and structured enough for custom solver | 400-600 lines of solver code, no Cargo.toml additions |
| Extraction DAG as snapshot | Solver operates on owned data, no shared state with e-graph | Safe for incremental re-extraction; no borrow-checker complexity |
| ILP opt-in, greedy default | Greedy is optimal for trees (no CSE); ILP only needed when CSE edges exist | Fast path for common case; ILP for optimality when it matters |

## Deviations from Plan

### Summary

| Type | Count | Impact |
|------|-------|--------|
| Design corrections (user feedback) | 2 | Improved alignment with project constraints |

### Design Correction 1: Solver crate is `no_std` + `alloc` (not `std`)

- **Original plan:** Separate `std` crate for solver
- **Correction:** Solver uses `#![no_std]` + `extern crate alloc`, like quine-core and quine-frontend. All needed collections (`BinaryHeap`, `BTreeMap`, `Vec`) are available in `alloc`.
- **Rationale:** User requirement — all library crates share the no_std stance.
- **Impact:** Updated Sections 1, 6.1, 6.5, 6.6, 7.3, 10.4 of the design report.

### Design Correction 2: `extract optimal <expr>` DSL syntax (not CLI flag)

- **Original plan:** `--ilp` CLI flag or `extract --optimal`
- **Correction:** `extract optimal <expr>` — inline DSL syntax, consistent with existing `extract <expr>`.
- **Rationale:** User requirement — simpler, no CLI configuration, natural extension of existing grammar.
- **Impact:** Updated Sections 1, 7.2, 7.3, 7.4, 7.5 of the design report.

## Issues Encountered

None — pure design phase, no code to build or test.

## Next Phase Readiness

**Ready:**
- Complete algorithm pseudocode for B&B-CR — Phase 7 can start implementing immediately
- Full ILP formulation with Quine type mappings — no design gaps
- Solver architecture with module structure and API sketch
- Testing strategy: exhaustive enumeration for small instances, property tests, no-CSE matches greedy
- Phase 7 AC suggestions included in Section 10.5

**Concerns:**
- B&B worst-case exponential in CSE edges — mitigation (time limit, size threshold, fallback to greedy) designed but needs implementation
- No external solver to validate against — must rely on exhaustive enumeration for small instances and property-based invariants
- `alloc::collections::BinaryHeap` availability confirmed — but needs verification in Phase 7 build

**Blockers:** None

---
*Phase: 06-ilp-solver-design, Plan: 01*
*Completed: 2026-06-05*
