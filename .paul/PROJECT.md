# Quine

**Relation-graph match-rewrite engine** — e-graph equality saturation with Datalog-style semi-naïve evaluation.

- **Language:** Rust (edition 2024)
- **Repository:** [LingWuMicrosystems/quine](https://github.com/LingWuMicrosystems/quine)
- **License:** see [LICENSE](../LICENSE)

## Architecture

Rust workspace with three crates:

| Crate | Purpose |
|-------|---------|
| `quine-core` | Core engine: e-graph (equality saturation), relations, rules, types, union-find |
| `quine-cli` | CLI + REPL for `.quine` files |
| `quine-frontend` | Parser / frontend for the Quine DSL |
| `quine-solver` | ILP solver: B&B-CR optimal extraction (no_std + alloc) |

## Core Engine (`quine-core`)

- **EGraph / equality saturation:** [`related_egraph.rs`](quine-core/src/related_egraph.rs) — relation-aware e-graph combining congruence closure with Datalog-style rule evaluation. `run_query` uses a 3-stage pipeline: scan → join → filter/permute.
- **Rule engine:** [`rule.rs`](quine-core/src/rule.rs) — match-rewrite rules evaluated via semi-naïve evaluation
- **Relations / tables:** [`table.rs`](quine-core/src/table.rs) — relational tables backing the Datalog layer
- **Types:** [`types.rs`](quine-core/src/types.rs) — algebraic data types (`data Option = Some(value) \| None`)
- **Union-Find:** [`uf.rs`](quine-core/src/uf.rs) — union-find for equivalence classes
- **Reverse Index:** [`reverse_index.rs`](quine-core/src/reverse_index.rs) — `ReverseIndex` struct maps canonical eclass → set of `(table_id, row_index)` enode references. Maintained through `insert`/`merge`/`remove` lifecycle. Query via `eclass_enodes(eclass)`.
- **Cost Analysis:** [`cost.rs`](quine-core/src/cost.rs) — `CostTracker` struct with incremental cost computation via lattice fixpoint `(u64, min, u64::MAX)`. `eclass_cost` tracks minimum cost per eclass; `cost_select` maps eclass → cheapest enode. Costs maintained eagerly at insert/union/rebuild. Cost models stored as `Map<String, u64>`.

## Key Decisions

| # | Decision | Rationale | Phase |
|---|----------|-----------|-------|
| 1 | reverse_index only tracks eclass-typed value columns (Id, named types), not literal types | Literal values don't participate in eclass unions | 01 |
| 2 | reverse_index merging on both explicit `union()` and rebuild-time key-dedup unions | Ensures complete coverage of all union paths | 01 |
| 3 | `ActionCtx::reverse_index` uses `&mut` reference (not owned) | Matches existing pattern of tables/union_find/pending_unions fields | 01 |
| 4 | Cost syntax uses flat `u64` per constructor: `cost TypeName.ConsName = <int>` | Simpler than expression-based costs; sufficient for cost-as-sum model | 02 |
| 5 | `EngineContext.cost_models: Map<String, u64>` stores costs, defaulting to 0 | Central location accessible by compilation and future analysis phases | 02 |
| 6 | Dotted names (`Option.Some`) parsed as single Pest variable, split at parser level | `.` is valid in Pest `variable_char`; splitting in parser avoids grammar complexity | 02 |
| 7 | Cost lattice `(u64, min, u64::MAX)` with `saturating_add` — costs decrease monotonically | Models cost as a fixpoint; cheaper equivalent expressions lower eclass cost; saturating_add propagates unknown (⊥) | 03 |
| 8 | ActionCtx::union performs eager cost merge (not deferred to rebuild) | Consistency with reverse_index merging pattern; ensures cost state stays synchronized across all union paths | 03 |
| 9 | cost_models moved from EngineContext to RelatedEGraph | Costs are an e-graph concern; RelatedEGraph owns both the data and the computation | 03 |
| 10 | Extract syntax changed to `extract <expr>` (concrete values, not patterns) | Extraction needs a concrete expression to find in the e-graph; pattern matching unnecessary | 04 |
| 11 | ILP solver algorithm: Branch-and-Bound with Combinatorial Relaxation (B&B-CR) | Exploits DAG structure; relaxation is O(|E|) DAG shortest-path; no floating-point; branches on CSE ownership | 06 |
| 12 | ILP solver crate: new quine-solver (no_std + alloc); all library crates share no_std stance | Consistent with quine-core/quine-frontend; alloc provides BinaryHeap, BTreeMap, Vec | 06 |
| 13 | ILP extraction syntax: `extract optimal <expr>` — inline DSL, not CLI flag | Natural extension of existing `extract <expr>`; discoverable; no configuration needed | 06 |
| 14 | CLI dispatches optimal extraction to ilp_extract (not EngineContext) | Avoids circular dependency: quine-solver → quine-frontend for Term | 08 |
| 15 | build_extraction_dag pre-checks eclass_enodes().is_empty() before enqueuing children | Prevents empty EclassNode entries in DAG (fix #18); simplest approach without index remapping | 08 |
| 16 | Term::Let uses Vec<(String, Term)> + Box<Term> — flat binding list, no nesting | User feedback: nested lets are unreadable; &mut Vec threading accumulates all shared bindings into single Let | 10 |
| 17 | Display format for Let: `(let ([name val] ...) body)` — square bracket pairs | User preference; distinct from standard S-expression `((name val) ...)` parens for visual clarity | 10 |
| 18 | Two-pass extraction: count_eclass_refs (DFS) → build_term_with_lets (build with Var) | Ensures shared eclasses detected before building; matches materialize_cheapest path (cost_select → find_defining_row) | 10 |
| 19 | Move Atom + Term to quine-core; break circular dep (quine-solver → quine-frontend) | Atom/Term are core data types used by solver, frontend, CLI; enables quine-frontend → quine-solver cleanly | 12 |
| 20 | ILP extraction in EngineContext.apply() (reverses Decision 14) | Original constraint (circular dep) eliminated by Term/Atom move; apply() now directly orchestrates both greedy and optimal extraction | 12 |

## DSL Syntax

```
data Option = Some(value) | None
relation edge(i32, i32)
function add(i32, i32) -> i32 merge min
fact set edge(1, 2)
rule edge(x, y) { set path(x, y) }
```

## Key Dependencies

- `rustc-hash` — fast hashing for e-graph internals
- `smallvec` — small-vector optimization
- `rayon` (optional) — parallel e-graph rebuilding

## Phase History

| Phase | Status | Completed |
|-------|--------|-----------|
| 01 — Core Engine (reverse_index, eclass_enodes) | ✅ Complete | 2026-06-02 |
| 02 — Cost + Extraction Syntax | ✅ Complete | 2026-06-03 |
| 03 — Cost Analysis (lattice, incremental computation) | ✅ Complete | 2026-06-03 |
| 04 — Change Extraction Syntax (extract \<expr\>) | ✅ Complete | 2026-06-03 |
| 05 — Expression Extraction (cost-aware evaluation) | ✅ Complete | 2026-06-03 |
| 06 — ILP Solver Design Report | ✅ Complete | 2026-06-05 |
| 07 — Solver Implementation | ✅ Complete | 2026-06-07 |
| 08 — Solver Integration | ✅ Complete | 2026-06-08 |
| 09 — Enhanced Extraction | ✅ Complete | 2026-06-08 |
| 10 — Term::Let Extraction | ✅ Complete | 2026-06-08 |

| 11 | Core Engine Simplification | ✅ Complete | 2026-06-13 |
| 12 | ILP Integration | ✅ Complete | 2026-06-25 |

---
*Last updated: 2026-06-25 after Phase 12 (v0.5 complete)*
