---
phase: 11-core-simplification
plan: 04
plan_type: execute
autonomous: true
completed: 2026-06-13
type: Summary
about: quine
---

## Outcome

**PASS** — Decomposed `run_query` into a clear 3-stage pipeline.

## What Changed

### `quine-core/src/related_egraph.rs` — 641 → 674 lines (+33)

Three private functions extracted from `run_query`:

| Function | Lines | Purpose |
|----------|-------|---------|
| `scan_step_table()` | 359-398 (40) | Scan a query step's table; handles both plain scan and push-down constraint optimization |
| `join_step_rows()` | 400-444 (45) | Join two row sets: cross-product (no shared vars) or hash join |
| `filter_and_permute()` | 446-475 (30) | Filter rows by cross-constraints, permute into VarId column order |

### Refactored `run_query` — 140 lines → 50 lines (−64%)

Three clearly demarcated stages:
1. **Stage 1:** Initial scan (step 0)
2. **Stage 2:** Per-step loop — shared/new_cols computation → `scan_step_table` → `join_step_rows` → extend vars
3. **Stage 3:** `filter_and_permute`

### Import addition
- Added `ColumnIndex`, `CrossConstraint`, `ScanStep` to crate imports

## Verification

- **46/46 tests pass** (quine-core: 15, quine-frontend: 5, quine-solver: 26)
- Zero behavioral changes — pure internal refactor
- All extracted functions are private (`fn`, not `pub fn`)
- Public API: `run_query(&self, query: &Query, delta_table: Option<TableId>) -> Set<Row>` unchanged

## Decisions

None — this was a pure mechanical refactor with no design choices.

## Phase 11 Totals

| Plan | Change | Lines |
|------|--------|-------|
| 11-01 | CostTracker → cost.rs | 892 → 721 (−171) |
| 11-02 | ReverseIndex → reverse_index.rs | 721 → 689 (−32) |
| 11-03 | insert/union/fresh_id delegation | 689 → 641 (−48) |
| 11-04 | run_query pipeline decomposition | 641 → 674 (+33) |
| **Net** | | **892 → 674 (−218, −24%)** |

Note: 11-04 added lines (+33) because function signatures and doc comments exceed the inlined code savings. The value is in readability, not line count.
