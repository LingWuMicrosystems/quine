---
status: planning
current_milestone: v0.2
current_phase: 04-expression-extraction
last_updated: 2026-06-03
---

## Current Position

Milestone: v0.2 Cost Model & Expression Extraction
Phase: 4 of 4 (Expression Extraction) — Ready to plan
Plan: Not started
Status: Phase 3 complete, ready to plan Phase 4
Last activity: 2026-06-03 — Phase 3 unified; 35 tests pass; git commit dec6f49

Progress:
- v0.2 Cost Model & Expression Extraction: [█████████░] 75%
- Phase 2: [██████████] 100%
- Phase 3: [██████████] 100%
- Phase 4: [░░░░░░░░░░] 0%

## Loop Position

Current loop state:
```
PLAN ──▶ APPLY ──▶ UNIFY
  ○        ○        ○     [Ready for Phase 4 planning]
```

## Session Continuity

Last session: 2026-06-03
Stopped at: Phase 3 complete and committed (dec6f49), ready to plan Phase 4
Next action: /paul:plan for Phase 4 — Expression Extraction
Resume file: .paul/ROADMAP.md
Resume context: Phase 3 delivered cost lattice, incremental cost computation, cost_select. Phase 4 uses this to extract lowest-cost expression.

## Decisions

| # | Decision | Plan | Date |
|---|----------|------|------|
| 1 | reverse_index only tracks eclass-typed value columns, not literal types | 01-01 | 2026-06-02 |
| 2 | reverse_index merging on both explicit union() and rebuild-time unions | 01-01 | 2026-06-02 |
| 3 | ActionCtx::reverse_index uses &mut reference pattern | 01-01 | 2026-06-02 |
| 4 | Cost syntax uses flat u64 per constructor: `cost TypeName.ConsName = <int>` | 02-01 | 2026-06-03 |
| 5 | CostDef stored in EngineContext.cost_models: Map<String, u64>, default 0 | 02-01 | 2026-06-03 |
| 6 | Dotted names parsed as single Pest variable, split via rsplit_once('.') in parser | 02-01 | 2026-06-03 |
