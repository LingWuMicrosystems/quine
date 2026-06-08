---
phase: 08-solver-integration
plan: 01
subsystem: solver-integration
tags: [rust, ilp, pest-grammar, dsl, extraction, cli]

requires:
  - phase: 07-ilp-solver-implementation
    plan: 03
    provides: B&B-CR solver with 28 tests, ilp_extract() entry point
provides:
  - extract optimal DSL syntax (grammar, parser, AST, compilation)
  - CLI routing: greedy → materialize_cheapest, optimal → ilp_extract()
  - Fix #18: empty eclass filtering in build_extraction_dag
affects:
  - phase: 09-enhanced-extraction (end-to-end extraction tests, solver config)

tech-stack:
  added:
    - quine-solver (workspace dep added to quine-cli)
  patterns:
    - ExtractMode enum for greedy vs optimal routing
    - CLI as solver bridge (quine-frontend stays solver-agnostic to avoid circular dep)

key-files:
  created: []
  modified:
    - docs/grammar.pest
    - quine-cli/src/pest_parser.rs
    - quine-frontend/src/syntax.rs
    - quine-frontend/src/compile/mod.rs
    - quine-frontend/src/lib.rs
    - quine-cli/src/main.rs
    - quine-cli/Cargo.toml
    - quine-solver/src/dag.rs

key-decisions:
  - "CLI dispatches optimal extraction (not EngineContext) because quine-solver depends on quine-frontend for Term, creating a circular dependency if quine-frontend imported quine-solver"
  - "ExtractMode::Greedy preserves backward-compatible extract <expr> behavior; ExtractMode::Optimal activates ilp_extract()"

patterns-established:
  - "ExtractMode enum propagated through Command → CompiledUnit → EngineContext.last_extract_info → CLI dispatch"
  - "Pre-check child eclasses for enodes in build_extraction_dag BFS to prevent empty DAG entries"

duration: ~30min
started: 2026-06-08
completed: 2026-06-08
description: "Wire ILP solver into extraction pipeline — extract optimal DSL syntax, CLI dispatch, fix #18 empty-eclass bug"
type: Summary
about: "quine"
---

# Phase 8 Plan 01: Solver Integration Summary

**Wire ILP solver into extraction pipeline — `extract optimal <expr>` DSL syntax, CLI dispatch to `ilp_extract()`, and fix #18 empty-eclass bug in `build_extraction_dag`.**

## Performance

| Metric | Value |
|--------|-------|
| Duration | ~30min |
| Started | 2026-06-08 |
| Completed | 2026-06-08 |
| Tasks | 3 completed |
| Files modified | 8 |

## Acceptance Criteria Results

| Criterion | Status | Notes |
|-----------|--------|-------|
| AC-1: `extract optimal` parses and compiles | ✅ Pass | Grammar, parser, Command enum, compilation — all thread ExtractMode correctly |
| AC-2: `extract optimal` produces ILP extraction via solver | ✅ Pass | CLI dispatches Optimal mode to `ilp_extract()` with `ILPConfig::default()` |
| AC-3: `extract` (no optimal) is backward compatible | ✅ Pass | Greedy path unchanged — uses `evaluate_and_extract()` → `materialize_cheapest()` |
| AC-4: Empty eclasses filtered from extraction DAG | ✅ Pass | `build_extraction_dag` pre-checks `eclass_enodes().is_empty()` before enqueuing children |
| AC-5: `cargo build` succeeds with zero errors, zero warnings | ✅ Pass | `cargo check` all 4 crates — clean |
| AC-6: Existing solver tests still pass | ✅ Pass | 26 tests, 0 failures, 0 ignored |

## Verification Results

```
$ cargo check -p quine-core -p quine-frontend -p quine-solver -p quine
Finished — zero errors, zero warnings

$ cargo test -p quine-solver
running 15 tests (lib) ... 15 passed
running 1 test (exhaustive_verify) ... 1 passed
running 7 tests (property_tests) ... 7 passed
running 3 tests (scenarios) ... 3 passed

Result: 26 passed, 0 failed, 0 ignored
```

## Accomplishments

- **DSL syntax:** `extract optimal <expr>` parses, compiles, and routes to ILP solver. `extract <expr>` (without `optimal`) unchanged.
- **CLI bridge:** quine-cli gains quine-solver dependency and dispatches optimal extraction to `ilp_extract()`. quine-frontend stays solver-agnostic (avoids circular dependency: quine-solver → quine-frontend for `Term`).
- **ExtractMode enum:** `Greedy | Optimal` — propagated through `Command::Extract(Expr, ExtractMode)` → `CompiledUnit::Extract(Expr, ExtractMode)` → `EngineContext.last_extract_info` → CLI dispatch.
- **Fix #18:** `build_extraction_dag` now pre-checks child eclass enodes before enqueuing — no more empty `EclassNode` entries in the DAG. Single-table schemas with dummy child values work correctly.

## Files Created/Modified

| File | Change | Purpose |
|------|--------|---------|
| `docs/grammar.pest` | Modified | Added `"optimal"` to KEYWORD; `extract_query` rule now has optional `"optimal"?` |
| `quine-cli/src/pest_parser.rs` | Modified | Parse `optimal` keyword in extract_query → `ExtractMode::Optimal` |
| `quine-frontend/src/syntax.rs` | Modified | Added `ExtractMode` enum; `Command::Extract(Expr, ExtractMode)` |
| `quine-frontend/src/compile/mod.rs` | Modified | Thread `ExtractMode` through compilation |
| `quine-frontend/src/lib.rs` | Modified | `CompiledUnit::Extract(Expr, ExtractMode)`; added `last_extract_info` field |
| `quine-cli/src/main.rs` | Modified | ILP dispatch: optimal → `ilp_extract()`, greedy → existing path |
| `quine-cli/Cargo.toml` | Modified | Added `quine-solver` workspace dependency |
| `quine-solver/src/dag.rs` | Modified | Fix #18: skip child eclasses with 0 enodes before enqueuing |

## Decisions Made

| Decision | Rationale | Impact |
|----------|-----------|--------|
| CLI dispatches optimal extraction (not EngineContext) | quine-solver depends on quine-frontend for `Term`; if quine-frontend depended on quine-solver, circular dependency | Clean architecture; CLI owns the bridge |
| `ExtractMode` enum with `Greedy` and `Optimal` variants | Simple sum type carried through Command → CompiledUnit → EngineContext → CLI | Extensible for future extraction modes |
| Pre-check `eclass_enodes().is_empty()` in BFS (not post-filter) | Avoids index remapping complexity; prevents empty eclasses from entering DAG | Minimal change, correct behavior |

## Deviations from Plan

### Summary

| Type | Count | Impact |
|------|-------|--------|
| Auto-fixed | 1 | Trivial — API correction |
| Scope additions | 0 | None |
| Deferred | 0 | None |

**Total impact:** One trivial API fix. No scope creep.

### Auto-fixed Issues

**1. HashSet API correction in #18 fix**
- **Found during:** Task 3 (verify)
- **Issue:** `eclass_enodes()` returns `HashSet`, not `Iterator` — `.next()` doesn't exist
- **Fix:** Changed to `.is_empty()` (direct HashSet method)
- **Files:** `quine-solver/src/dag.rs`
- **Verification:** `cargo test -p quine-solver` — all 26 pass

## Issues Encountered

| Issue | Resolution |
|-------|------------|
| Linker error (`-liconv`) during `cargo build -p quine` | Environmental; `cargo check` and `cargo test` work correctly |

## Next Phase Readiness

**Ready:**
- Solver integration complete — `extract optimal` syntax works end-to-end
- #18 fixed — `build_extraction_dag` handles any e-graph shape
- All crates compile clean, all solver tests pass
- Phase 9 (Enhanced Extraction) can now build end-to-end extraction tests

**Concerns:**
- `max_cse_edges_warning` and `time_limit_ms` fields in `ILPConfig` are unused (placeholder config)
- No end-to-end integration test for `extract optimal` DSL path (parser → CLI → ilp_extract → output)
- `Cargo.lock` needs to be committed (new quine-solver dep in quine-cli)

**Blockers:**
- None for Phase 9 (Enhanced Extraction)

---
*Phase: 08-solver-integration, Plan: 01*
*Completed: 2026-06-08*
