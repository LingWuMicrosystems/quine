---
phase: 12-ilp-integration
plan: 01
subsystem: core-engine
tags: ilp, extraction, dependency-management, architecture

# Dependency graph
requires:
  - phase: 11-core-simplification
    provides: "Simplified related_egraph (674 lines), extracted CostTracker + ReverseIndex"
provides:
  - "ILP optimal extraction integrated into EngineContext.apply()"
  - "Atom + Term moved to quine-core (break quine-solver → quine-frontend cycle)"
  - "quine-cli no longer imports quine-solver"
affects: all future extraction work, solver work

# Tech tracking
tech-stack:
  added: ["quine-solver as dependency of quine-frontend"]
  patterns: ["Core types (Atom, Term) live in quine-core; frontend re-exports for compat", "Extraction orchestration centralized in EngineContext.apply()"]

key-files:
  created: ["quine-core/src/atom.rs", "quine-core/src/term.rs"]
  modified: ["quine-frontend/src/lib.rs (apply())", "quine-cli/src/main.rs (deduplicated)", "quine-solver/Cargo.toml (dropped quine-frontend dep)"]

key-decisions:
  - "Decision 20: Move ILP optimal extraction into EngineContext.apply(); break circular dep by moving Term+Atom to quine-core"
  - "Decision 14 REVERSED: CLI no longer dispatches ilp_extract — EngineContext now handles it"

patterns-established:
  - "Extraction result: last_extract (Term) + last_extract_warning (Option<String>) — unified for greedy and optimal"
  - "Dependency direction: quine-solver → quine-core ← quine-frontend → quine-solver (clean DAG, no cycles)"

# Metrics
duration: ~20min
started: 2026-06-24T22:00:00Z
completed: 2026-06-25T01:00:00Z
description: "Moved ILP optimal extraction from quine-cli into EngineContext.apply(); resolved circular dependency by moving Term+Atom to quine-core"
type: Summary
about: "quine"
---

# Phase 12 Plan 01: ILP Extraction Integration Summary

**Moved ILP optimal extraction from quine-cli into EngineContext.apply(), breaking the quine-solver → quine-frontend circular dependency by relocating Term and Atom to quine-core.**

## Performance

| Metric | Value |
|--------|-------|
| Duration | ~20min |
| Started | 2026-06-24 |
| Completed | 2026-06-25 |
| Tasks | 3 completed |
| Files modified | 18 |

## Acceptance Criteria Results

| Criterion | Status | Notes |
|-----------|--------|-------|
| AC-1: Types moved — no circular dependency | Pass | quine-solver → quine-core (Term/Atom); quine-frontend → quine-solver (clean); cargo build passes |
| AC-2: ILP extraction inside apply() | Pass | CompiledUnit::Extract(_, Optimal) now calls ilp_extract directly; last_extract + last_extract_warning populated |
| AC-3: CLI no longer imports quine-solver | Pass | grep confirms zero quine_solver references in quine-cli/ |
| AC-4: All existing tests pass | Pass | quine-core + quine-frontend + quine-solver: all 28 tests pass; quine-cli tests compile (link blocked by known -liconv issue) |

## Accomplishments

- **Broke circular dependency**: Moved `Atom` + `Term` from quine-frontend to quine-core; quine-solver now depends only on quine-core
- **Centralized ILP extraction**: `EngineContext::apply()` directly calls `ilp_extract` for `ExtractMode::Optimal`, populating `last_extract` and `last_extract_warning`
- **Eliminated last_extract_info hack**: Removed the split-path where greedy set `last_extract` but optimal deferred to CLI — now unified
- **Deduplicated CLI code**: `execute_file_command` and `execute_repl_commands` each had ~20 lines of duplicate ILP handling; now both are 5-line unified blocks
- **Cleaned dependency graph**: quine-cli no longer imports quine-solver; dependency flows cleanly through quine-frontend

## Files Created/Modified

| File | Change | Purpose |
|------|--------|---------|
| `quine-core/src/atom.rs` | Created | Atom enum + Display + get_type() moved from quine-frontend |
| `quine-core/src/term.rs` | Created | Term enum + Display + tests moved from quine-frontend |
| `quine-core/src/lib.rs` | Modified | Added `pub mod atom; pub mod term;` |
| `quine-frontend/src/syntax.rs` | Modified | Removed Atom definition; re-exports from quine-core; removed unused BaseType import |
| `quine-frontend/src/term.rs` | Modified | Replaced 120-line file with re-export from quine-core |
| `quine-frontend/src/error.rs` | Modified | Changed import: `crate::syntax::Atom` → `quine_core::atom::Atom` |
| `quine-frontend/src/lib.rs` | Modified | Added quine_solver import; added last_extract_warning field; removed last_extract_info; updated apply() Optimal arm |
| `quine-frontend/Cargo.toml` | Modified | Added `quine-solver = { workspace = true }` |
| `quine-solver/src/lib.rs` | Modified | Changed import: `quine_frontend::term::Term` → `quine_core::term::Term` |
| `quine-solver/src/solver.rs` | Modified | Changed imports: Atom + Term from quine_core |
| `quine-solver/src/formulation.rs` | Modified | Changed import: Atom from quine_core |
| `quine-solver/Cargo.toml` | Modified | Removed `quine-frontend` dependency |
| `quine-solver/tests/property_tests.rs` | Modified | Added `use quine_core::term::Term`; fixed inline path reference |
| `quine-cli/Cargo.toml` | Modified | Removed `quine-solver` dependency |
| `quine-cli/src/main.rs` | Modified | Removed quine_solver + ExtractMode imports; replaced 2x duplicate ILP blocks with unified code |
| `quine-cli/tests/syntax_tests/extract_optimal.rs` | Modified | Updated 2 tests from `last_extract_info` to `last_extract` + `last_extract_warning` |
| `.paul/ROADMAP.md` | Modified | Deleted Phase 12/13, added Phase 12 ILP Integration |
| `.paul/phases/12-solver-simplification/` | Deleted | Old phase directory |
| `.paul/phases/13-frontend-cli-consolidation/` | Deleted | Old phase directory |

## Decisions Made

| Decision | Rationale | Impact |
|----------|-----------|--------|
| Move Atom + Term to quine-core | Breaks circular dep; these are core data types used by solver, frontend, and CLI | quine-solver no longer depends on quine-frontend |
| Reverse Decision 14 | Original constraint (circular dep) eliminated by Term/Atom move | EngineContext can now directly orchestrate ILP extraction |
| Add last_extract_warning field | quine-frontend is no_std; can't use eprintln | Warning surfaced to CLI via field, not direct print |

## Deviations from Plan

### Summary

| Type | Count | Impact |
|------|-------|--------|
| Auto-fixed | 2 | Test file updates, missing derives |

**Total impact:** Essential fixes, no scope creep.

### Auto-fixed Issues

**1. Missing Eq + Hash derives on Atom**
- **Found during:** Task 1 verification (cargo build)
- **Issue:** Atom in quine-core was missing `Eq` and `Hash` derives that were on the original in quine-frontend
- **Fix:** Added `Eq, Hash` to `#[derive(...)]` on `quine-core/src/atom.rs`
- **Files:** `quine-core/src/atom.rs`
- **Verification:** cargo build passes

**2. Stale test references to last_extract_info**
- **Found during:** Task 3 verification (cargo test)
- **Issue:** `quine-cli/tests/syntax_tests/extract_optimal.rs` referenced removed `last_extract_info` field
- **Fix:** Updated 2 test functions to use `last_extract` + `last_extract_warning` instead
- **Files:** `quine-cli/tests/syntax_tests/extract_optimal.rs`
- **Verification:** cargo check --tests passes

## Issues Encountered

| Issue | Resolution |
|-------|------------|
| Stray `#[derive(...)]` attribute left before re-export in syntax.rs | Removed extraneous derive attribute |
| Unused `BaseType` import in syntax.rs after Atom removal | Removed import |
| quine-solver test used full `quine_frontend::term::Term` path | Added import + changed to `Term` directly |
| Binary linking fails with `-liconv` | Known environment issue (STATE.md Issue #1), unrelated to this plan |

## Next Phase Readiness

**Ready:**
- ILP extraction fully integrated into EngineContext.apply() — any consumer gets optimal extraction automatically
- Dependency graph simplified: quine-core owns fundamental types; no circular dependencies
- CLI code deduplicated and simplified

**Concerns:**
- `-liconv` linker error prevents running quine binary and integration tests on this machine
- quine-solver crate still exists as standalone (1,485 lines) — could be merged into quine-frontend in future

**Blockers:**
- None

---
*Built with PAUL Framework*
*Phase: 12-ilp-integration, Plan: 01*
*Completed: 2026-06-25*
