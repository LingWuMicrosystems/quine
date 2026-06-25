# Roadmap

## Milestone: v0.1 — Core Engine ✅

| # | Phase | Status | Completed |
|---|-------|--------|-----------|
| 1 | Core Engine | ✅ Complete | 2026-06-02 |

**Plans:** 1/1 complete
**Summary:** Implemented reverse_index (eclass → enode references map) with full lifecycle maintenance and eclass_enodes query method. 10 BDD integration tests passing.

---

## Milestone: v0.2 — Cost Model & Expression Extraction ✅

Status: ✅ Complete
Phases: 4 of 4 complete

| # | Phase | Plans | Status | Completed |
|---|-------|-------|--------|-----------|
| 2 | Cost + Extraction Syntax | [02-01](.paul/phases/02-cost-extraction-syntax/02-01-SUMMARY.md) | ✅ Complete | 2026-06-03 |
| 3 | Cost Analysis | [03-01](.paul/phases/03-cost-analysis/03-01-SUMMARY.md) | ✅ Complete | 2026-06-03 |
| 4 | Change Extraction Syntax | [04-01](.paul/phases/04-change-extraction-syntax/04-01-SUMMARY.md) | ✅ Complete | 2026-06-03 |
| 5 | Expression Extraction | [05-01](.paul/phases/05-expression-extraction/05-01-SUMMARY.md) | ✅ Complete | 2026-06-03 |

### Phase 2: Cost + Extraction Syntax ✅

Focus: DSL syntax for defining cost models and extraction queries
Plans: 1/1 complete — `cost TypeName.ConsName = <int>` syntax, EngineContext.cost_models storage, 14 BDD tests

### Phase 3: Cost Analysis ✅

Focus: Cost model evaluation and analysis of expression costs within the e-graph
Plans: 1/1 complete — cost lattice `(u64, min, u64::MAX)` integrated into RelatedEGraph; incremental computation at 5 mutation points; cost_select tracking; 6 BDD tests

### Phase 4: Change Extraction Syntax ✅

Focus: Replace `extract <pattern> print(<vars>)` with `extract <expr>` — concrete values instead of patterns
Plans: 1/1 complete — changed grammar, parser, AST, compilation, tests, README

### Phase 5: Expression Extraction ✅

Focus: Cost-aware expression extraction — `extract <expr>` evaluates against e-graph, uses cost lattice to find cheapest equivalent, materializes and prints result
Plans: 1/1 complete — [05-01](.paul/phases/05-expression-extraction/05-01-SUMMARY.md) implemented evaluate_expr, materialize_cheapest, CLI integration, 6 integration tests

---

## Milestone: v0.3 — ILP Solver Enhanced Extraction ✅

Status: ✅ Complete
Phases: 4 of 4 complete

| # | Phase | Plans | Status | Completed |
|---|-------|-------|--------|-----------|
| 6 | ILP Solver Design Report | [06-01](.paul/phases/06-ilp-solver-design/06-01-SUMMARY.md) | ✅ Complete | 2026-06-05 |
| 7 | Solver Implementation | [07-01](.paul/phases/07-ilp-solver-implementation/07-01-SUMMARY.md), [07-02](.paul/phases/07-ilp-solver-implementation/07-02-SUMMARY.md), [07-03](.paul/phases/07-ilp-solver-implementation/07-03-SUMMARY.md) | ✅ Complete | 2026-06-07 |
| 8 | Solver Integration | [08-01](.paul/phases/08-solver-integration/08-01-SUMMARY.md) | ✅ Complete | 2026-06-08 |
| 9 | Enhanced Extraction | [09-01](.paul/phases/09-enhanced-extraction/09-01-SUMMARY.md) | ✅ Complete | 2026-06-08 |

### Phase 6: ILP Solver Design Report ✅

Focus: Architecture and design document for the built-in ILP solver
Plans: 1/1 complete — [06-01](.paul/phases/06-ilp-solver-design/06-01-SUMMARY.md) — comprehensive design report: B&B-CR algorithm, ILP formulation mapped to Quine types, solver architecture (new quine-solver crate, no_std + alloc), `extract optimal <expr>` DSL syntax, 2 worked examples

### Phase 7: Solver Implementation ✅

Focus: Build the ILP solver core engine
Plans: 3/3 complete — [07-01](.paul/phases/07-ilp-solver-implementation/07-01-SUMMARY.md) (crate scaffold + data layer), [07-02](.paul/phases/07-ilp-solver-implementation/07-02-SUMMARY.md) (B&B-CR solver algorithm), [07-03](.paul/phases/07-ilp-solver-implementation/07-03-SUMMARY.md) (28 tests covering exhaustive verification, property invariants, and design report scenarios)

### Phase 8: Solver Integration

Focus: Wire the ILP solver into the e-graph and extraction pipeline
Plans: TBD (defined during /paul:plan)

### Phase 9: Enhanced Extraction ✅

Focus: Wire ILPConfig fields, add integration tests
Plans: 1/1 complete — [09-01](.paul/phases/09-enhanced-extraction/09-01-SUMMARY.md) wired time_limit_ms→B&B node budget, max_cse_edges_warning→user warning, 5 extract optimal integration tests; fuzz testing deferred

---

## Milestone: v0.4 — Extraction Ergonomics ✅

Status: ✅ Complete
Phases: 1 of 1 complete

| # | Phase | Plans | Status | Completed |
|---|-------|-------|--------|-----------|
| 10 | Term::Let Extraction | [10-01](.paul/phases/10-term-let/10-01-SUMMARY.md) | ✅ Complete | 2026-06-08 |

### Phase 10: Term::Let Extraction ✅

Focus: Let-binding for shared sub-expressions — `Term::Let` enables extraction output to bind multiply-referenced nodes to a name, eliminating duplication in printed expressions
Plans: 1/1 complete — [10-01](.paul/phases/10-term-let/10-01-SUMMARY.md) added Term::Let + Term::Var variants, two-pass let-aware extraction (reference counting + build), flat `(let ([name val] ...) body)` display, 5 new unit tests, 5 integration tests

---

## Milestone: v0.5 — Refactor & Simplify 🚧

Status: ✅ Complete
Phases: 3 of 3 complete

| # | Phase | Plans | Status | Completed |
|---|-------|-------|--------|-----------|
| 11 | Core Engine Simplification | [11-01](.paul/phases/11-core-simplification/11-01-PLAN.md), [11-02](.paul/phases/11-core-simplification/11-02-PLAN.md), [11-03](.paul/phases/11-core-simplification/11-03-PLAN.md), [11-04](.paul/phases/11-core-simplification/11-04-PLAN.md) | ✅ Complete | 2026-06-13 |
| 12 | ILP Integration | [12-01](.paul/phases/12-ilp-integration/12-01-SUMMARY.md) | ✅ Complete | 2026-06-25 |

### Phase 11: Core Engine Simplification ✅

Focus: Split `related_egraph.rs` (892 lines), tighten APIs, and remove dead paths
Plans: 4/4 complete — [11-01](.paul/phases/11-core-simplification/11-01-SUMMARY.md) (CostTracker → cost.rs, −160 lines), [11-02](.paul/phases/11-core-simplification/11-02-SUMMARY.md) (ReverseIndex → reverse_index.rs, −32 lines), [11-03](.paul/phases/11-core-simplification/11-03-SUMMARY.md) (insert/union/fresh_id delegation, −48 lines), [11-04](.paul/phases/11-core-simplification/11-04-SUMMARY.md) (run_query pipeline: scan_step_table + join_step_rows + filter_and_permute, 140→50 lines). Total: 892 → 674 (−218, −24%)

### Phase 12: ILP Integration ✅

Focus: Move ILP optimal extraction from `quine-cli` into `EngineContext.apply()`; break circular dependency (`quine-solver → quine-frontend` for Term/Atom) by moving `Term` and `Atom` to `quine-core`
Plans: 1/1 complete — [12-01](.paul/phases/12-ilp-integration/12-01-SUMMARY.md) moved Atom+Term → quine-core, wired ilp_extract into apply(), deduplicated CLI, removed quine-solver from CLI deps

---
*Last updated: 2026-06-25 — v0.5 milestone complete*
