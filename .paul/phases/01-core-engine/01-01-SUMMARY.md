---
phase: 01-core-engine
plan: 01
subsystem: core
tags: [rust, egraph, reverse-index, union-find, equality-saturation]
requires: []
provides:
  - reverse_index: Map<Value, Set<(TableId, RowIndex)>> on RelatedEGraph
  - eclass_enodes query method for reverse eclass → enode lookup
  - fresh_id public API for allocating eclass IDs
affects:
  - extraction (future)
  - pattern-matching (future)
  - debugging (future)
tech-stack:
  added: []
  patterns:
    - Reverse index maintained co-located with data mutation (insert/rebuild), not lazily recomputed
    - ActionCtx uses &mut references mirroring other context fields
key-files:
  created:
    - quine-core/tests/reverse_index.rs
  modified:
    - quine-core/src/related_egraph.rs
key-decisions:
  - "reverse_index tracks only eclass-typed value columns (Type::Name / BaseType::Id), not literal types (I64, F64, etc.)"
  - "reverse_index merging happens both on explicit union() and during rebuild's duplicate-row resolution"
  - "ActionCtx::reverse_index is a &mut reference, not owned — matches the existing pattern of tables/union_find/pending_unions"
patterns-established:
  - "Index maintenance: reverse_index is populated on insert (NewRow), merged on union, cleaned on rebuild (absorbed rows removed, canonical keys updated)"
duration: ~15min
started: 2026-06-02T16:30:00Z
completed: 2026-06-02T17:00:00Z
---

# Phase 01 Plan 01: Reverse Index & eclass_enodes Summary

**Implemented reverse_index (eclass → enode references map) on RelatedEGraph with full lifecycle maintenance across insert, union, and rebuild, plus eclass_enodes query method — verified with 10 BDD integration tests.**

## Performance

| Metric | Value |
|--------|-------|
| Duration | ~30min |
| Started | 2026-06-02 |
| Completed | 2026-06-02 |
| Tasks | 3 completed |
| Files modified | 1 |
| Files created | 1 |

## Acceptance Criteria Results

| Criterion | Status | Notes |
|-----------|--------|-------|
| AC-1: Single insert populates reverse_index | Pass | Verified by `ac1_single_insert_populates_reverse_index` |
| AC-2: Multiple rows same eclass aggregate | Pass | Verified by `ac2_multiple_rows_same_eclass_aggregate` |
| AC-3: Union merges reverse_index entries | Pass | Verified by `ac3_union_merges_reverse_index_entries` |
| AC-4: Rebuild removes merged rows | Pass | Verified by `ac4_rebuild_removes_merged_rows` |
| AC-5: Literal-typed tables not tracked | Pass | Verified by `ac5_literal_typed_tables_not_tracked` |
| AC-6: Canonical changes update keys | Pass | Verified by `ac6_canonical_changes_update_reverse_index_keys` |
| AC-7: eclass_enodes returns all enodes | Pass | Verified by `ac7_eclass_enodes_returns_all_enodes` |
| AC-8: eclass_enodes canonicalizes input | Pass | Verified by `ac8_eclass_enodes_canonicalizes_input` |
| AC-9: Unknown eclass returns empty | Pass | Verified by `ac9_eclass_enodes_unknown_eclass_returns_empty` |
| AC-10: Cross-table query works | Pass | Verified by `ac10_eclass_enodes_cross_table` |

## Accomplishments

- Added `reverse_index: Map<Value, Set<(TableId, RowIndex)>>` to `RelatedEGraph` — maintained automatically through insert, union, and rebuild
- Implemented `eclass_enodes(&self, eclass: Value) -> Set<(TableId, RowIndex)>` — canonicalizes input then queries the index
- Populated reverse_index on insert (NewRow) for eclass-typed value columns only; literal types (I64, etc.) excluded
- Merged reverse_index entries on union (both explicit `union()` calls and rebuild-time key deduplication unions)
- Removed absorbed rows from reverse_index during rebuild (key canonicalization causing duplicate rows)
- Updated reverse_index keys when canonical values change via union
- Added `fresh_id()` public method for allocating new eclass IDs registered with the union-find
- 10/10 BDD integration tests passing with Given/When/Then doc comments

## Files Created/Modified

| File | Change | Purpose |
|------|--------|---------|
| `quine-core/src/related_egraph.rs` | Modified (+80 lines) | reverse_index field, maintenance in insert/union/rebuild, eclass_enodes, fresh_id |
| `quine-core/tests/reverse_index.rs` | Created (303 lines) | 10 BDD integration tests (AC-1 through AC-10) |

## Decisions Made

| Decision | Rationale | Impact |
|----------|-----------|--------|
| reverse_index only tracks eclass-typed value columns | Literal values (I64, F64) don't participate in eclass unions; tracking them would add noise | Clean index, only meaningful entries |
| Merge reverse_index in `union()` method directly | Unions change canonicals; immediate merge ensures consistency without lazy resolution | No stale keys in reverse_index |
| Also merge in rebuild's None case | Rebuild can create unions (via duplicate row detection) that don't go through `union()` method | Complete coverage of all union paths |
| `ActionCtx::reverse_index` as `&mut` reference | Matches existing pattern of other ActionCtx fields (tables, union_find, pending_unions) | Consistent API, no ownership transfer |
| `fresh_id()` as public method | Tests and external callers need to create eclass IDs registered with union-find | Clean test API without exposing union_find internals |

## Deviations from Plan

### Summary

| Type | Count | Impact |
|------|-------|--------|
| Auto-fixed | 0 | N/A |
| Scope additions | 1 | Minor — `fresh_id()` public method added |
| Deferred | 0 | None |

**Total impact:** Essential addition (fresh_id) — required for testability, no scope creep.

### Scope Additions

**1. Public API: `fresh_id()` method added**
- **Reason:** Tests need to allocate eclass IDs registered with union-find. No public API existed; `ActionCtx::alloc_id` was private.
- **Impact:** Small — mirrors existing `ActionCtx::alloc_id` pattern, legitimate public API need.

### User-Directed Changes

**1. Test structure: 12 original tests → 10 BDD tests**
- **User instruction:** Use AC-1 through AC-10 BDD scenarios directly as tests with doc comments
- **Result:** 10 tests (one per AC) instead of original 12 scaffolded tests

## Issues Encountered

| Issue | Resolution |
|-------|------------|
| Missing `ActionCtx` construction site at different indentation level | Fixed with separate targeted edit |
| AC-5 (I64 table) — `RelatedEGraph::insert` calls `find(value)` which panics for encoded I64 values | Used `fresh_id()` to prime union_find, then inserted `Value(0)` into I64-typed table — type check (not value) determines tracking |

## Next Phase Readiness

**Ready:**
- `reverse_index` is maintained through all mutation paths (insert, union, rebuild)
- `eclass_enodes` provides efficient eclass → enode lookup
- 10 integration tests cover core behaviors and edge cases
- Both `no_std` and `std` (rayon) feature builds pass

**Concerns:**
- `RelatedEGraph::insert` calls `find(value)` unconditionally — panics for literal values not in union-find. This is a pre-existing issue, not introduced here, but limits table type flexibility.

**Blockers:**
- None

---
*Phase: 01-core-engine, Plan: 01*
*Completed: 2026-06-02*
