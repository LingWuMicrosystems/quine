---
status: in_progress
current_milestone: v0.5
current_phase: 11-core-simplification
last_updated: 2026-06-13
---

## Current Position

Milestone: v0.5 Refactor & Simplify — 🚧 In Progress
Phase: 11 of 3 (Core Engine Simplification) — Plan 11-02 closed
Plan: 11-02 closed
Status: Loop closed, ready for next PLAN
Last activity: 2026-06-13 — Closed 11-02 loop (UNIFY)

Progress:
- v0.5 Refactor & Simplify: [████░░░░░░] 33%
- Phase 11: [██████████] 67% (Core Engine Simplification — 2 of ~3 plans)
- Phase 12: [░░░░░░░░░░] 0% (Solver Simplification)
- Phase 13: [░░░░░░░░░░] 0% (Frontend & CLI Consolidation)

## Loop Position

Current loop state:
```
PLAN ──▶ APPLY ──▶ UNIFY
  ✓        ✓        ✓     [Loop complete — ready for next PLAN]
```

## Session Continuity

Last session: 2026-06-13
Stopped at: 11-02 loop closed, session paused
Next action: /paul:resume → /paul:plan for Phase 11 plan 03
Resume file: .paul/HANDOFF-2026-06-13.md
Resume context:
- 11-01 extracted CostTracker into cost.rs, 11-02 extracted ReverseIndex into reverse_index.rs
- related_egraph.rs trimmed 892→689 lines across both plans
- Two extraction patterns established: struct + methods taking read-only deps as params
- 46/46 tests pass; 11-03 ready: remaining duplication + API tightening

## Accumulated Context

### Decisions

| # | Decision | Plan | Date |
|---|----------|------|------|
| 1 | reverse_index only tracks eclass-typed value columns, not literal types | 01-01 | 2026-06-02 |
| 2 | reverse_index merging on both explicit union() and rebuild-time unions | 01-01 | 2026-06-02 |
| 3 | ActionCtx::reverse_index uses &mut reference pattern | 01-01 | 2026-06-02 |
| 4 | Cost syntax uses flat u64 per constructor: `cost TypeName.ConsName = <int>` | 02-01 | 2026-06-03 |
| 5 | CostDef stored in EngineContext.cost_models: Map<String, u64>, default 0 | 02-01 | 2026-06-03 |
| 6 | Dotted names parsed as single Pest variable, split via rsplit_once('.') in parser | 02-01 | 2026-06-03 |
| 7 | Added Phase 4: Change Extraction Syntax; original Phase 4 (Expression Extraction) renumbered to 5 | Phase 3 | 2026-06-03 |
| 8 | Atom expressions in extract short-circuit to Term::Literal (not eclass IDs) | 05-01 | 2026-06-03 |
| 9 | evaluate_expr does not canonicalize atom values via union_find.find() | 05-01 | 2026-06-03 |
| 10 | materialize_cheapest falls back to extract_inner when no cost_select entry | 05-01 | 2026-06-03 |
| 11 | Constructor name resolution: table_types.name_map → cons2type_map fallback | 05-01 | 2026-06-03 |
| 12 | ILP solver algorithm: Branch-and-Bound with Combinatorial Relaxation (B&B-CR) | 06-01 | 2026-06-05 |
| 13 | Solver crate: new quine-solver (no_std + alloc); consistent with quine-core/quine-frontend | 06-01 | 2026-06-05 |
| 14 | ILP extraction via `extract optimal <expr>` DSL syntax; greedy remains default for `extract <expr>` | 06-01 | 2026-06-05 |
| 15 | No external solver dependencies — all custom implementation | 06-01 | 2026-06-05 |
| 16 | FixedDecision refactored enum→struct with selected + cse fields + entry API merge | 07-02 fix | 2026-06-07 |
| 17 | find_cse_violations accepts &fixed param; skips edges with CSE decision to prevent infinite recursion | 07-03 | 2026-06-07 |
| 18 | BDD Given/When/Then doc comments on all 26 test functions | 07-03 | 2026-06-07 |
| 19 | Created milestone v0.4 Extraction Ergonomics — single phase: Term::Let | Phase 10 | 2026-06-08 |

### Git State
Last commit: f4455e6
Branch: main
Uncommitted: none

### Known Issues

| # | Issue | Status | Fix in |
|---|-------|--------|--------|
| 1 | `-liconv` linker error prevents quine binary from linking on this machine | Known env issue | N/A |
| 2 | Fuzz testing (random DAG + brute-force) not yet implemented | Deferred | Future phase |
