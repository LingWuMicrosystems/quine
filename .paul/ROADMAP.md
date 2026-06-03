# Roadmap

## Milestone: v0.1 — Core Engine ✅

| # | Phase | Status | Completed |
|---|-------|--------|-----------|
| 1 | Core Engine | ✅ Complete | 2026-06-02 |

**Plans:** 1/1 complete
**Summary:** Implemented reverse_index (eclass → enode references map) with full lifecycle maintenance and eclass_enodes query method. 10 BDD integration tests passing.

---

## Current Milestone

**v0.2 Cost Model & Expression Extraction**
Status: 🚧 In Progress
Phases: 3 of 4 complete

| # | Phase | Plans | Status | Completed |
|---|-------|-------|--------|-----------|
| 2 | Cost + Extraction Syntax | [02-01](.paul/phases/02-cost-extraction-syntax/02-01-SUMMARY.md) | ✅ Complete | 2026-06-03 |
| 3 | Cost Analysis | [03-01](.paul/phases/03-cost-analysis/03-01-SUMMARY.md) | ✅ Complete | 2026-06-03 |
| 4 | Change Extraction Syntax | [04-01](.paul/phases/04-change-extraction-syntax/04-01-PLAN.md) | ✅ Complete | 2026-06-03 |
| 5 | Expression Extraction | TBD | Not started | - |

### Phase 2: Cost + Extraction Syntax

Focus: DSL syntax for defining cost models and extraction queries
Plans: 1/1 complete — `cost TypeName.ConsName = <int>` and `extract <pattern> print(<vars>)` syntax, EngineContext.cost_models storage, 14 BDD tests

### Phase 3: Cost Analysis ✅

Focus: Cost model evaluation and analysis of expression costs within the e-graph
Plans: 1/1 complete — cost lattice `(u64, min, u64::MAX)` integrated into RelatedEGraph; incremental computation at 5 mutation points; cost_select tracking; 6 BDD tests

### Phase 4: Change Extraction Syntax ✅

Focus: Replace `extract <pattern> print(<vars>)` with `extract <expr>` — concrete values instead of patterns
Plans: 1/1 complete — [04-01](.paul/phases/04-change-extraction-syntax/04-01-SUMMARY.md) changed grammar, parser, AST, compilation, tests, README

### Phase 5: Expression Extraction 🔵

Focus: Extract the lowest-cost equivalent expression for a given expression from the e-graph
Plans: TBD (defined during /paul:plan)
Status: Not started

---
*Last updated: 2026-06-03 (Phase 4 complete — extract <expr> syntax)*
