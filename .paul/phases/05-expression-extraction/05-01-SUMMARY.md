---
phase: 05-expression-extraction
plan: 01
subsystem: core,frontend,cli
tags: [rust, e-graph, cost-model, extraction, expression-evaluation, materialization]
requires:
  - phase: 03-cost-analysis
    provides: Cost lattice, cost_select, eclass_cost — incremental cost computation in RelatedEGraph
  - phase: 04-change-extraction-syntax
    provides: extract <expr> syntax, Command::Extract(Expr), CompiledUnit::Extract(Expr)
provides:
  - Expression evaluation: Expr → canonical eclass Value (recursive constructor lookup)
  - Cost-aware extraction: cost_select → cheapest enode → recursive materialization
  - End-to-end `extract <expr>` REPL command with cost-aware output
affects:
  - future optimization phases (extraction performance, caching)
tech-stack:
  added: []
  patterns:
    - evaluate_expr resolves FunctionCall recursively: evaluate args → build key Row → lookup in e-graph table
    - materialize_cheapest uses cost_select at each recursion level for globally-optimal extraction
    - Atom expressions short-circuit to Term::Literal (atoms aren't eclass IDs)
    - Constructor name resolution: direct table_types lookup → cons2type_map fallback
key-files:
  created:
    - quine-cli/tests/syntax_tests/extract_eval.rs (6 integration tests)
  modified:
    - quine-core/src/table.rs (Table::get_by_key)
    - quine-frontend/src/interner.rs (Interner::max_id)
    - quine-frontend/src/error.rs (VariableInExtract variant)
    - quine-frontend/src/compile/mod.rs (validate_extract_expr, Extract compile validation)
    - quine-frontend/src/lib.rs (evaluate_expr, materialize_cheapest, last_extract field, apply wiring)
    - quine-cli/src/main.rs (Extract handling in REPL + file paths)
key-decisions:
  - "Atom expressions in extract short-circuit to Term::Literal — they are not eclass references"
  - "evaluate_expr does not canonicalize atom values via union_find.find()"
  - "materialize_cheapest falls back to extract_inner (scan-based) when no cost_select entry exists"
  - "Constructor resolution: direct table_types.name_map lookup first, then cons2type_map for short names"
  - "Extract output goes through last_extract field on EngineContext (no_std compatible — printing is CLI responsibility)"
patterns-established:
  - "New extraction methods (evaluate_expr, materialize_cheapest) are additive — existing extract()/extract_inner() unchanged"
  - "CLI handles Extract specially (like Query), not through generic compile+apply path"
duration: ~30min
started: 2026-06-03T20:00:00+08:00
completed: 2026-06-03T20:30:00+08:00
---

# Phase 5 Plan 01: Expression Extraction Summary

**End-to-end cost-aware expression extraction: `extract <expr>` evaluates against e-graph, uses cost lattice to find cheapest equivalent, materializes and prints result.**

## Performance

| Metric | Value |
|--------|-------|
| Duration | ~30min |
| Tasks | 2/2 completed |
| Files modified | 6 |
| Files created | 1 |

## Acceptance Criteria Results

| Criterion | Status | Notes |
|-----------|--------|-------|
| AC-1: Simple constructor `extract Option.Some(42)` | Pass | Output: `(Option.Some 42)` |
| AC-2: Cost-aware returns cheapest equivalent | Pass | `extract T.A(1)` returns `(T.B 1)` when T.B cheaper |
| AC-3: Nested constructors resolve recursively | Pass | `evaluate_expr` resolves `Add(Const(1), Const(2))` through recursive evaluation |
| AC-4: Atom literal works | Pass | `extract 42u64` → `42` |
| AC-5: Unknown constructor errors | Pass | `extract NoSuch.Foo(1)` → compile error |
| AC-6: Variable in extract errors | Pass | `extract x` → VariableInExtract error |

## Verification Results

```
cargo test -p quine-core -p quine-frontend -p quine
  26 tests (quine CLI): all pass (+6 new extract_eval)
   5 tests (quine-core lattice): all pass
  10 tests (quine-core reverse_index): all pass
Total: 41/41 passing, 0 failures, 0 regressions
```

## Accomplishments

- **Expression evaluation** — `evaluate_expr` resolves `Expr` to canonical eclass `Value` by recursively evaluating constructor calls against e-graph tables. Handles `FunctionCall` (table lookup by key), `Atom` (literal value conversion), and `Variable` (error).
- **Cost-aware materialization** — `materialize_cheapest` uses `cost_select` at each recursion level to pick the cheapest equivalent enode, producing a globally-optimal `Term`.
- **Compile-time validation** — Extract expressions are validated: variables rejected, constructor names checked against `table_types` and `cons2type_map`.
- **CLI integration** — `Command::Extract` handled specially in both REPL and file execution paths, printing the result via `last_extract` field.
- **6 integration tests** — Covering simple constructor, cost-aware cheapest, nested resolution, atom literal, and both error cases.

## Files Changed

| File | Change | Purpose |
|------|--------|---------|
| `quine-core/src/table.rs` | Modified (+6) | `Table::get_by_key()` public key lookup |
| `quine-frontend/src/interner.rs` | Modified (+3) | `Interner::max_id()` for string interner iteration |
| `quine-frontend/src/error.rs` | Modified (+1) | `VariableInExtract` compile error variant |
| `quine-frontend/src/compile/mod.rs` | Modified (+35) | `validate_extract_expr`, Extract compile-time validation |
| `quine-frontend/src/lib.rs` | Modified (+136) | `evaluate_expr`, `materialize_cheapest`, `evaluate_and_extract`, `resolve_constructor_to_table`, `value_from_atom`, `last_extract` field, `apply()` wiring |
| `quine-cli/src/main.rs` | Modified (+25) | Extract handling in REPL and file execution |
| `quine-cli/tests/syntax_tests/extract_eval.rs` | Created (+162) | 6 BDD integration tests |

## Decisions Made

| Decision | Rationale | Impact |
|----------|-----------|--------|
| Atom expressions short-circuit to `Term::Literal` in `evaluate_and_extract` | Atoms are not eclass IDs — they're literal data values | Clean separation; no crash from invalid union-find index |
| `evaluate_expr` doesn't canonicalize atom values | Atom values may not be valid union-find IDs | Prevents index-out-of-bounds panic |
| `materialize_cheapest` falls back to `extract_inner` when no `cost_select` | Eclasses without cost info (no cost model defined) still work | Graceful degradation |
| Constructor name resolution: `table_types` → `cons2type_map` | Short constructor names (e.g., `Some`) need resolution to full table name (`Option.Some`) | Both fully-qualified and short names work |
| New methods are additive, existing `extract()`/`extract_inner()` unchanged | Query result printing uses separate path; shouldn't couple with cost-aware extraction | No regression risk for query output |

## Deviations from Plan

### Summary

| Type | Count | Impact |
|------|-------|--------|
| Scope adjusted | 1 | AC-3 tested via `evaluate_expr` directly instead of full extract DSL |

### AC-3 Adjustment

**AC-3 scope adjusted:** The plan specified testing `extract Expr.Add(Expr.Const(1), Expr.Const(2))` through the full DSL pipeline. However, the type system requires concrete base types in facts (e.g., `data Expr = Add(i32, i32)`), and recursive type references (`data Expr = Add(Expr, Expr)`) can't be expressed for facts with the current type system. The core behavior — recursive nested expression evaluation — is verified via `evaluate_expr` directly, testing that `Add(Const(1), Const(2))` correctly resolves through recursive `FunctionCall` evaluation.

## Issues Encountered

| Issue | Resolution |
|-------|------------|
| Atom `extract 42u64` crashed on `union_find.find(Value(42))` — index out of bounds | Short-circuit atoms in `evaluate_and_extract` to `Term::Literal` |
| `extract_vars` not found for `Expr` | Import `VarExtractor` trait in compile/mod.rs |
| `Interner` missing `max_id()` | Added `max_id()` method for string value lookup iteration |
| `parse_file` doesn't support bare `set`/`run` (needs `fact` prefix, `run saturate`) | Adjusted test DSL syntax to valid grammar |
| DSL type system doesn't support recursive types in facts (`Name` types vs `Base` types) | AC-3 tested via low-level `evaluate_expr` call instead of full extract pipeline |

## Next Phase Readiness

**Ready:**
- v0.2 milestone complete — all 5 phases delivered
- `extract <expr>` fully functional: parse → compile → evaluate → cost_select → materialize → print
- 41 tests passing, cost lattice stable, reverse_index stable

**Concerns:**
- String interner lookup in `value_from_atom` is O(n) — acceptable for current scale
- Recursive types (`data Expr = Add(Expr, Expr)`) not expressible for facts in DSL

**Blockers:** None

---
*Phase: 05-expression-extraction, Plan: 01*
*Completed: 2026-06-03*
