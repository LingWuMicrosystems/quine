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

## Core Engine (`quine-core`)

- **EGraph / equality saturation:** [`related_egraph.rs`](quine-core/src/related_egraph.rs) — relation-aware e-graph combining congruence closure with Datalog-style rule evaluation
- **Rule engine:** [`rule.rs`](quine-core/src/rule.rs) — match-rewrite rules evaluated via semi-naïve evaluation
- **Relations / tables:** [`table.rs`](quine-core/src/table.rs) — relational tables backing the Datalog layer
- **Types:** [`types.rs`](quine-core/src/types.rs) — algebraic data types (`data Option = Some(value) \| None`)
- **Union-Find:** [`uf.rs`](quine-core/src/uf.rs) — union-find for equivalence classes
- **Reverse Index:** [`related_egraph.rs`](quine-core/src/related_egraph.rs) — `reverse_index` maps canonical eclass → set of `(table_id, row_index)` enode references, maintained through insert/union/rebuild. Query via `eclass_enodes(eclass)`.

## Key Decisions

| # | Decision | Rationale | Phase |
|---|----------|-----------|-------|
| 1 | reverse_index only tracks eclass-typed value columns (Id, named types), not literal types | Literal values don't participate in eclass unions | 01 |
| 2 | reverse_index merging on both explicit `union()` and rebuild-time key-dedup unions | Ensures complete coverage of all union paths | 01 |
| 3 | `ActionCtx::reverse_index` uses `&mut` reference (not owned) | Matches existing pattern of tables/union_find/pending_unions fields | 01 |
| 4 | Cost syntax uses flat `u64` per constructor: `cost TypeName.ConsName = <int>` | Simpler than expression-based costs; sufficient for cost-as-sum model | 02 |
| 5 | `EngineContext.cost_models: Map<String, u64>` stores costs, defaulting to 0 | Central location accessible by compilation and future analysis phases | 02 |
| 6 | Dotted names (`Option.Some`) parsed as single Pest variable, split at parser level | `.` is valid in Pest `variable_char`; splitting in parser avoids grammar complexity | 02 |

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

---
*Last updated: 2026-06-03 after Phase 02*
