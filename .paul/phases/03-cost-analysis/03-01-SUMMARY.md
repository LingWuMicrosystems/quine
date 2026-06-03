---
phase: 03-cost-analysis
plan: 01
subsystem: core-engine
tags: [lattice, cost-model, equality-saturation, incremental-computation]

requires:
  - phase: 02-cost-extraction-syntax
    provides: cost DSL syntax, CostDef parsing, EngineContext.cost_models storage
provides:
  - cost lattice (u64, min, u64::MAX) integrated into RelatedEGraph
  - incremental cost maintenance at 5 mutation points
  - EngineContext cost delegation to RelatedEGraph
  - BDD integration tests for cost analysis
affects: [04-expression-extraction]

tech-stack:
  added: [smallvec (quine-cli dev-dependency)]
  patterns: [incremental fixpoint maintenance, free-function code sharing between structs]

key-files:
  created:
    - quine-cli/tests/syntax_tests/cost_analysis.rs
  modified:
    - quine-core/src/related_egraph.rs
    - quine-frontend/src/lib.rs
    - quine-cli/tests/syntax_tests/main.rs
    - quine-cli/tests/syntax_tests/cost.rs
    - quine-cli/Cargo.toml
    - README.md

key-decisions:
  - "Cost lattice: (u64, min, u64::MAX) with saturating_add; costs decrease monotonically"
  - "Cost computation factored into free functions shared between ActionCtx and RelatedEGraph"
  - "ActionCtx::union performs eager cost merge (not deferred to rebuild)"
  - "cost_select redirect during rebuild uses find(new) to target absorbed row's canonical"

patterns-established:
  - "Eager incremental cost maintenance: costs computed at insert, merged at union, redirected at rebuild absorption"
  - "Free-function refactoring pattern: when two struct impls need identical logic, factor into module-level fn"

duration: ~15min
started: 2026-06-03
completed: 2026-06-03
---

# Phase 3 Plan 1: Cost Analysis Summary

**Cost lattice (u64, min, u64::MAX) integrated into RelatedEGraph with incremental maintenance at all 5 mutation points.**

## Performance

| Metric | Value |
|--------|-------|
| Duration | ~15min |
| Tasks | 5 completed |
| Files modified | 6 |

## Acceptance Criteria Results

| Criterion | Status | Notes |
|-----------|--------|-------|
| AC-1: Cost lattice definition | Pass | `(u64, min, u64::MAX)` documented; 5 lattice unit tests |
| AC-2: Cost computed on insert | Pass | Tested in ac1, ac3 tree cost summation |
| AC-3: Cost merged on union | Pass | ac2: min(5, 1) = 1 after union |
| AC-4: cost_select redirected during rebuild | Pass | ac6: absorbed RowIndex(1) → surviving RowIndex(0) |
| AC-5: Undefined constructor defaults to 0 | Pass | ac4: undefined Add = 0, total = 20 from children only |
| AC-6: Unknown child propagates u64::MAX | Pass | Implicit: saturating_add tested in lattice unit tests |

## Accomplishments

- Cost lattice `(u64, ⊑, ⊥, ⊤, ⊔)` where `a ⊑ b iff a ≥ b`, `⊔ = min`, `⊥ = u64::MAX` — fully documented and tested
- Incremental cost computation integrated into all 5 mutation points: ActionCtx::insert, ActionCtx::union, RelatedEGraph::insert, RelatedEGraph::union, RelatedEGraph::rebuild
- cost_select tracks cheapest enode per eclass and redirects correctly when enodes are absorbed during rebuild
- ActionCtx gains cost_models, eclass_cost, cost_select fields for eager cost updates during rule evaluation
- cost_models moved from EngineContext to RelatedEGraph; EngineContext provides delegation accessors

## Task Commits

All tasks delivered in a single atomic unit (per PAUL APPLY convention).

| Task | Description |
|------|-------------|
| Task 1: Cost data structures | Added fields + accessors + lattice docs + 5 unit tests to RelatedEGraph |
| Task 2: Incremental computation | Cost logic in all 5 mutation points; free fn refactoring |
| Task 3: EngineContext update | Removed cost_models; added delegation accessors; fixed cost tests |
| Task 4: BDD tests | 6 cost_analysis tests + smallvec dev-dep |
| Task 5: README | Cost Analysis section with lattice description |

## Files Created/Modified

| File | Change | Purpose |
|------|--------|---------|
| `quine-core/src/related_egraph.rs` | Modified (+130 lines) | Lattice fields, accessors, cost computation in 5 mutation points, free fns, unit tests |
| `quine-frontend/src/lib.rs` | Modified | Removed cost_models, add CostDef delegation, add accessor methods |
| `quine-cli/tests/syntax_tests/cost_analysis.rs` | Created (+280 lines) | 6 BDD tests for cost analysis |
| `quine-cli/tests/syntax_tests/main.rs` | Modified (+1 line) | `mod cost_analysis;` |
| `quine-cli/tests/syntax_tests/cost.rs` | Modified (3 lines) | `ctx.cost_models` → `ctx.regraph.cost_models` |
| `quine-cli/Cargo.toml` | Modified (+3 lines) | smallvec dev-dependency |
| `README.md` | Modified (+12 lines) | Cost Analysis section |

## Decisions Made

| Decision | Rationale | Impact |
|----------|-----------|--------|
| Free function refactoring for cost logic | ActionCtx and RelatedEGraph share identical cost computation; duplication would be brittle | Single source of truth for compute_and_update_eclass_cost and merge_eclass_cost_into |
| BDD tests use RelatedEGraph directly | Plan allowed direct e-graph manipulation; simpler than constructing Datalog for every scenario | Tests exercise cost logic directly without parsing overhead |
| cost_select D1 redirect targets find(new) | The absorbed row's result canonical is find(new); this is the key in cost_select | Correct redirection before merge_eclass_cost_into removes the child entry |

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

| Issue | Resolution |
|-------|------------|
| Test table definitions missing value column (arity mismatch) | Added Id-typed value column to all table definitions |
| AC-6 child eclasses had no costs (MAX propagated) | Seeded child eclasses with cost-0 enodes via Seed.S table |
| AC-6 test expectations inverted (absorbed vs surviving) | Fixed: R0 is surviving, R1 is absorbed; assert surviving R0 |

## Next Phase Readiness

**Ready:**
- eclass_cost and cost_select are populated for all eclasses with known costs
- cost_select provides (TableId, RowIndex) → cheap enode lookup for extraction
- Constructor cost lookup via `get_constructor_cost` / `constructor_cost`

**Concerns:**
- No worklist propagation: when a child's cost decreases, parent costs are not re-propagated. This is correct for bottom-up insertion (children exist before parents) but Phase 4 may need it for cyclic/forward-reference scenarios.
- ActionCtx cost fields add borrowing complexity; future refactors may need to manage the growing field list

**Blockers:** None

---
*Phase: 03-cost-analysis, Plan: 01*
*Completed: 2026-06-03*
