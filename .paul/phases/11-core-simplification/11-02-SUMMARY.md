---
phase: 11-core-simplification
plan: 02
subsystem: core
tags: [reverse_index, eclass_enodes, refactor, rust, no_std]
requires: []
provides:
  - "ReverseIndex: self-contained tracking of eclass → enode references"
  - "Eliminated duplicated insert/merge patterns between ActionCtx and RelatedEGraph"
affects: ["12-solver-simplification", "13-frontend-cli-consolidation"]
tech-stack:
  added: []
  patterns:
    - "ReverseIndex pattern: extract coherent Map-based subsystem with insert/merge/remove/get methods"
key-files:
  created: ["quine-core/src/reverse_index.rs"]
  modified:
    - "quine-core/src/related_egraph.rs"
    - "quine-core/src/lib.rs"
key-decisions: []
patterns-established:
  - "Subsystem extraction: struct + methods (consistent with CostTracker from 11-01)"
duration: ~10min
started: 2026-06-13
completed: 2026-06-13
description: "Extract ReverseIndex into reverse_index.rs, eliminate duplicated insert/merge patterns between ActionCtx and RelatedEGraph"
type: Summary
about: "quine"
---

# Phase 11 Plan 02: Extract ReverseIndex Summary

**Extracted reverse_index tracking into `reverse_index.rs` module, eliminating duplicated insert (2→1) and merge (3→1) patterns.**

## Performance

| Metric | Value |
|--------|-------|
| Duration | ~10min |
| Started | 2026-06-13 |
| Completed | 2026-06-13 |
| Tasks | 3 completed |
| Files modified | 3 |

## Acceptance Criteria Results

| Criterion | Status | Notes |
|-----------|--------|-------|
| AC-1: ReverseIndex extracted with no behavior change | Pass | 46/46 tests pass, zero source changes outside quine-core |
| AC-2: Duplicated patterns eliminated | Pass | insert: 2 copies → 1 method. merge: 3 copies → 1 method. remove: inline → 1 method. |
| AC-3: Public API preserved | Pass | `eclass_enodes` signature unchanged for quine-solver callers |

## Accomplishments

- **New module `reverse_index.rs`** (65 lines): `ReverseIndex` struct wrapping `Map<Value, Set<(TableId, RowIndex)>>` with `insert`, `merge`, `remove`, `get` methods
- **Eliminated pattern duplication:** Insert was in ActionCtx::insert and RelatedEGraph::insert — now one `ReverseIndex::insert`. Merge was in ActionCtx::union, RelatedEGraph::union, and RelatedEGraph::rebuild — now one `ReverseIndex::merge`
- **related_egraph.rs** reduced from 721 → 689 lines (−32), clean compile, all 46 tests pass
- **ActionCtx and RelatedEGraph** both call the same ReverseIndex methods — no duplicated Map manipulation
- **eclass_enodes** simplified: `self.reverse_index.get(&canonical).cloned().unwrap_or_default()` → `self.reverse_index.get(canonical)`

## Files Created/Modified

| File | Change | Purpose |
|------|--------|---------|
| `quine-core/src/reverse_index.rs` | Created (65 lines) | ReverseIndex struct with insert, merge, remove, get methods |
| `quine-core/src/related_egraph.rs` | Modified (721→689, −32) | Replace `Map<Value, Set<...>>` with `ReverseIndex`; 12 call sites updated |
| `quine-core/src/lib.rs` | Modified (+1 line) | Added `pub mod reverse_index;` |

## Decisions Made

None — followed plan as specified, no architectural choices emerged.

## Deviations from Plan

### Summary

| Type | Count | Impact |
|------|-------|--------|
| Auto-fixed | 0 | - |
| Scope additions | 0 | - |
| Deferred | 0 | - |

**Total impact:** Plan executed exactly as written — pure mechanical extraction, no surprises.

### Line reduction estimate variance

Plan estimated ~60 lines reduced from related_egraph.rs; actual was 32. The reverse_index patterns (insert, merge) are individually smaller (2-4 lines each) than the cost algorithms extracted in 11-01 (12-30 lines each). The value is in eliminating duplication and encapsulating Map operations behind a typed API, not raw line count.

## Issues Encountered

| Issue | Resolution |
|-------|------------|
| `-liconv` linker error blocks CLI binary tests | Known env issue (STATE.md Issue #1); quine-core, quine-frontend, quine-solver all pass |
| Two identical merge patterns triggered replace_all | Used `replace_all: true` — both sites needed identical replacement |

## Next Phase Readiness

**Ready:**
- CostTracker and ReverseIndex patterns established — proven extraction approach for 11-03
- 46/46 tests green — solid baseline
- related_egraph.rs at 689 lines — still room for 11-03 (remaining duplication + API tightening)

**Concerns:**
- None for next plan

**Blockers:**
- None

---
*Phase: 11-core-simplification, Plan: 02*
*Completed: 2026-06-13*
