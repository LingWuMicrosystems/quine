---
phase: 11-core-simplification
plan: 03
subsystem: core
tags: [refactor, deduplication, delegation, action_ctx, rust, no_std]
requires: []
provides:
  - "action_ctx() helper: single ActionCtx construction point for RelatedEGraph"
  - "Eliminated insert/union/fresh_id duplication via delegation to ActionCtx"
affects: ["12-solver-simplification", "13-frontend-cli-consolidation"]
tech-stack:
  added: []
  patterns:
    - "Delegation pattern: RelatedEGraph public methods delegate to ActionCtx via action_ctx() helper"
key-files:
  created: []
  modified:
    - "quine-core/src/related_egraph.rs"
key-decisions: []
patterns-established:
  - "action_ctx() delegation: RelatedEGraph methods delegate to ActionCtx for mutation; ActionCtx is single source of truth"
duration: ~10min
started: 2026-06-13
completed: 2026-06-13
description: "Eliminate remaining insert/union duplication via delegation to ActionCtx, deduplicate ActionCtx construction, tighten public API"
type: Summary
about: "quine"
---

# Phase 11 Plan 03: Eliminate Duplication & Tighten API Summary

**Eliminated remaining insert/union/fresh_id duplication via delegation to ActionCtx. Added `action_ctx()` helper. related_egraph.rs: 689 → 641 lines (−48).**

## Performance

| Metric | Value |
|--------|-------|
| Duration | ~10min |
| Started | 2026-06-13 |
| Completed | 2026-06-13 |
| Tasks | 3 completed |
| Files modified | 1 |

## Acceptance Criteria Results

| Criterion | Status | Notes |
|-----------|--------|-------|
| AC-1: insert/union duplication eliminated | Pass | `RelatedEGraph::insert` and `union` are 3-line delegations to `ActionCtx` |
| AC-2: ActionCtx construction deduplicated | Pass | `action_ctx()` helper added; one justified exception in `run_semi_naive` (borrow checker) |
| AC-3: alloc_id/fresh_id deduplicated | Pass | `fresh_id` delegates: `self.action_ctx().alloc_id()` |
| AC-4: All tests pass, zero behavior change | Pass | 46/46 tests pass, 0 warnings |
| AC-5: Public API tightened | Pass | `action_ctx()` is private; `ActionCtx` not leaked outside module |

## Accomplishments

- **Added `action_ctx()` helper** (private): constructs `ActionCtx` from `RelatedEGraph` fields — 1 canonical construction site
- **Eliminated insert duplication:** `RelatedEGraph::insert` (was ~51 lines) → 3-line delegation to `ctx.insert()`. `ActionCtx::insert` is the sole implementation.
- **Eliminated union duplication:** `RelatedEGraph::union` (was ~10 lines) → 3-line delegation to `ctx.union()`. `ActionCtx::union` is the sole implementation.
- **Eliminated fresh_id duplication:** `RelatedEGraph::fresh_id` → delegates to `self.action_ctx().alloc_id()`.
- **related_egraph.rs** reduced from 689 → 641 lines (−48). Phase 11 total: 892 → 641 (−251 lines).
- **ActionCtx is single source of truth** for all mutation logic — insert, union, and alloc_id live exclusively on ActionCtx.
- **46/46 tests pass**, 0 failures, 0 warnings. Zero changes outside `quine-core/src/related_egraph.rs`.

## Files Created/Modified

| File | Change | Purpose |
|------|--------|---------|
| `quine-core/src/related_egraph.rs` | Modified (689→641, −48) | Added `action_ctx()`; `insert`/`union`/`fresh_id` delegate; `apply_action` uses helper |

## Decisions Made

None — followed plan as specified. One borrow-checker adaptation documented below.

## Deviations from Plan

### Summary

| Type | Count | Impact |
|------|-------|--------|
| Auto-fixed | 1 | Minimal — justified technical exception |
| Scope additions | 0 | - |
| Deferred | 0 | - |

**Total impact:** One justified technical exception — no scope creep.

### Auto-fixed Issues

**1. run_semi_naive keeps inline ActionCtx construction**
- **Found during:** Task 1 (action_ctx helper)
- **Issue:** `action_ctx()` takes `&mut self` which conflicts with `action` reference borrowing `self.ruleset` immutably. Rust's borrow checker cannot see that the borrows are disjoint when routed through a method call.
- **Fix:** Kept inline `ActionCtx { ... }` construction in `run_semi_naive` with explanatory comment. The original code already used field-level disjoint borrows for this exact reason. All other call sites (`apply_action`, `insert`, `union`, `fresh_id`) use `action_ctx()` successfully.
- **Files:** `quine-core/src/related_egraph.rs` (run_semi_naive loop, lines ~262-272)
- **Verification:** `cargo check` clean, 46/46 tests pass

## Issues Encountered

| Issue | Resolution |
|-------|------------|
| `-liconv` linker error blocks CLI binary tests | Known env issue (STATE.md Issue #1); all 3 library crates + solver pass 46/46 |
| Borrow checker: `action_ctx()` conflicts with `action` ref in `run_semi_naive` | Kept inline construction with comment — Rust cannot see disjoint field borrows through method calls |

## Next Phase Readiness

**Ready:**
- Phase 11 complete: related_egraph.rs reduced from 892 → 641 lines (−28%)
- Delegation pattern established for Phase 12 (Solver Simplification) reference
- 46/46 tests green — solid baseline for Phase 12

**Concerns:**
- None for Phase 12

**Blockers:**
- None

---
*Phase: 11-core-simplification, Plan: 03*
*Completed: 2026-06-13*
