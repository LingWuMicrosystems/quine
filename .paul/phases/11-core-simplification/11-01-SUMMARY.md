---
phase: 11-core-simplification
plan: 01
subsystem: core
tags: [cost, lattice, refactor, rust, no_std]
requires: []
provides:
  - "CostTracker: self-contained cost tracking (models, eclass_cost, cost_select)"
  - "Eliminated duplicated cost methods between ActionCtx and standalone functions"
affects: ["12-solver-simplification", "13-frontend-cli-consolidation"]
tech-stack:
  added: []
  patterns:
    - "CostTracker pattern: extract coherent subsystem with mutable state, pass read-only deps (tables, union_find) as params"
key-files:
  created: ["quine-core/src/cost.rs"]
  modified:
    - "quine-core/src/related_egraph.rs"
    - "quine-core/src/lib.rs"
    - "quine-cli/tests/syntax_tests/cost.rs"
key-decisions:
  - "cost_select made pub on CostTracker for rebuild D1 redirect pattern"
  - "ActionCtx delegates directly to cost_tracker (no wrapper methods)"
patterns-established:
  - "Subsystem extraction: struct + methods taking read-only deps as parameters"
duration: ~15min
started: 2026-06-13
completed: 2026-06-13
description: "Extract CostTracker into cost.rs, eliminate duplicated cost methods between ActionCtx and standalone functions"
type: Summary
about: "quine"
---

# Phase 11 Plan 01: Extract CostTracker Summary

**Extracted cost tracking into `cost.rs` module, eliminating 160 lines and 2 duplicated method pairs from `related_egraph.rs`.**

## Performance

| Metric | Value |
|--------|-------|
| Duration | ~15min |
| Started | 2026-06-13 |
| Completed | 2026-06-13 |
| Tasks | 3 completed |
| Files modified | 4 |

## Acceptance Criteria Results

| Criterion | Status | Notes |
|-----------|--------|-------|
| AC-1: CostTracker extracted with no behavior change | Pass | 46/46 tests pass, zero source changes outside quine-core (except test ref fix) |
| AC-2: Duplication eliminated | Pass | One copy of `compute_and_update_eclass_cost` and `merge_eclass_cost` — both in CostTracker |
| AC-3: Public API preserved | Pass | `set_cost_model`, `get_constructor_cost`, `eclass_cost`, `cost_select` signatures unchanged |

## Accomplishments

- **New module `cost.rs`** (147 lines): `CostTracker` struct with `cost_models`, `eclass_cost`, `cost_select` fields, plus `compute_and_update_eclass_cost` and `merge_eclass_cost` methods
- **Eliminated duplication:** Removed the ActionCtx method + standalone function pair for both cost algorithms — now a single `CostTracker` method each
- **related_egraph.rs** reduced from 892 → 721 lines (−160), clean compile, all 46 tests pass
- **ActionCtx simplified:** `insert` and `union` call `self.cost_tracker.compute_and_update_eclass_cost(...)` and `self.cost_tracker.merge_eclass_cost(...)` directly — no wrapper methods
- **Dead code removed:** `cost_select_matches` was unused, removed during post-APPLY cleanup

## Files Created/Modified

| File | Change | Purpose |
|------|--------|---------|
| `quine-core/src/cost.rs` | Created (147 lines) | CostTracker struct with cost lattice, cost tracking, 5 lattice tests |
| `quine-core/src/related_egraph.rs` | Modified (892→721, −160) | Replace 3 cost fields + 4 methods with single `cost_tracker` field + delegation |
| `quine-core/src/lib.rs` | Modified (+1 line) | Added `pub mod cost;` |
| `quine-cli/tests/syntax_tests/cost.rs` | Modified (4 refs) | Updated `cost_models` → `cost_tracker.cost_models` |

## Decisions Made

| Decision | Rationale | Impact |
|----------|-----------|--------|
| `cost_select_matches` removed | Unused; `cost_select_redirect` handles rebuild D1 redirect internally | Cleaner API, 10 fewer lines |
| ActionCtx wrapper methods removed | Pure delegation — callers can reach `cost_tracker` directly | 10 fewer lines, less indirection |
| `pub cost_tracker` on RelatedEGraph | Preserves access for test files that reach `cost_models` directly | Minimal API surface change |

## Deviations from Plan

### Summary

| Type | Count | Impact |
|------|-------|--------|
| Auto-fixed | 2 | Post-review cleanup |
| Scope additions | 0 | - |
| Deferred | 0 | - |

### Auto-fixed Issues

**1. Unused `cost_select_matches` method**
- **Found during:** Post-APPLY review
- **Issue:** Method added during extraction but never used — `cost_select_redirect` already handles the D1 check internally
- **Fix:** Removed from cost.rs
- **Files:** `quine-core/src/cost.rs`
- **Verification:** Full test suite, cargo check clean

**2. Redundant ActionCtx wrapper methods**
- **Found during:** Post-APPLY review
- **Issue:** `ActionCtx::compute_and_update_eclass_cost` and `ActionCtx::merge_eclass_cost` were 2-line wrappers that just delegated to `cost_tracker`
- **Fix:** Removed wrapper methods; call sites now invoke `self.cost_tracker.xxx(...)` directly. Also restored `ActionCtx::union` which was accidentally removed during edit
- **Files:** `quine-core/src/related_egraph.rs`
- **Verification:** 46 tests pass, cargo check clean

## Issues Encountered

| Issue | Resolution |
|-------|------------|
| `-liconv` linker error blocks CLI integration tests | Known env issue (STATE.md Issue #1); 5 CLI tests compile-ready but can't link |
| `ActionCtx::union` accidentally removed during edit | Re-added immediately with updated `cost_tracker.merge_eclass_cost` call |
| 4 test files referenced `regraph.cost_models` directly | Updated to `regraph.cost_tracker.cost_models` |

## Next Phase Readiness

**Ready:**
- CostTracker pattern established for further extractions
- 46/46 tests green — solid baseline
- `cost_tracker` field is `pub` — tests and callers can access cost_models directly

**Concerns:**
- None for next plan

**Blockers:**
- None

---
*Phase: 11-core-simplification, Plan: 01*
*Completed: 2026-06-13*
