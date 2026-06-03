---
status: complete
current_milestone: v0.2
current_phase: 05-expression-extraction
last_updated: 2026-06-03
---

## Current Position

Milestone: v0.2 Cost Model & Expression Extraction — **COMPLETE**
Phase: 5 of 5 (Expression Extraction) — Complete
Plan: 05-01 unified, loop closed
Status: v0.2 milestone complete — all 5 phases delivered
Last activity: 2026-06-03 — UNIFY complete, SUMMARY.md created, loop closed

Progress:
- v0.2 Cost Model & Expression Extraction: [██████████] 100%
- Phase 2: [██████████] 100%
- Phase 3: [██████████] 100%
- Phase 4: [██████████] 100%
- Phase 5: [██████████] 100%

## Loop Position

Current loop state:
```
PLAN ──▶ APPLY ──▶ UNIFY
  ✓        ✓        ✓     [Loop closed — milestone complete]
```

## Session Continuity

Last session: 2026-06-03
Stopped at: UNIFY complete — v0.2 milestone delivered, 41 tests pass
Next action: Plan next milestone (v0.3 or user-directed)
Resume file: .paul/phases/05-expression-extraction/05-01-SUMMARY.md

## Decisions

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
