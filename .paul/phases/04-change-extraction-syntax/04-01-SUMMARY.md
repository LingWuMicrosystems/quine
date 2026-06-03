---
phase: 04-change-extraction-syntax
plan: 01
subsystem: syntax
tags: [rust, parser, dsl, extraction, pest, grammar]
requires:
  - phase: 02-cost-extraction-syntax
    provides: Original extract grammar, Command::Extract, CompiledUnit::Extract, BDD tests
  - phase: 03-cost-analysis
    provides: Cost lattice and cost_select (used by Phase 5 extraction)
provides:
  - Simplified extract syntax: `extract <expr>` replacing `extract <pattern> print(<vars>)`
  - Command::Extract(Expr) instead of Command::Extract(Heads, Vec<String>)
  - CompiledUnit::Extract(Expr) — stores concrete expression, no query compilation
affects:
  - expression extraction (Phase 5 — will implement cost-aware extraction from Expr)
tech-stack:
  added: []
  patterns:
    - Extract no longer goes through heads2query compilation — Expr is stored directly
    - Expr Display uses s-expression format (space-separated, paren-wrapped nested calls)
key-files:
  modified:
    - docs/grammar.pest
    - quine-cli/src/pest_parser.rs
    - quine-frontend/src/syntax.rs
    - quine-frontend/src/compile/mod.rs
    - quine-frontend/src/lib.rs
    - quine-cli/tests/syntax_tests/extract.rs
    - README.md
key-decisions:
  - "extract syntax: `extract <expr>` where expr is a concrete value (FunctionCall or AtomOrVariable)"
  - "CompiledUnit::Extract holds Expr directly — no query compilation in Phase 4"
  - "Expression type-checking deferred to Phase 5"
duration: ~5min
started: 2026-06-03T19:20:00+08:00
completed: 2026-06-03T19:25:00+08:00
---

# Phase 4 Plan 01: Change Extraction Syntax Summary

**Changed extraction DSL syntax from `extract <pattern> print(<vars>)` to `extract <expr>` — concrete values instead of patterns.**

## Performance

| Metric | Value |
|--------|-------|
| Duration | ~5min |
| Tasks | 2/2 completed |
| Files modified | 7 |

## Acceptance Criteria Results

| Criterion | Status | Notes |
|-----------|--------|-------|
| AC-1: Parse `extract <expr>` with nested constructors | Pass | `extract Expr.Add(Expr.Const(0i32), Expr.Const(4i32))` → FunctionCall with 2 args |
| AC-2: Reject old `extract <pattern> print(<vars>)` | Pass | `extract expr(x) print(x)` → parse error |
| AC-3: Extract compiles to CompiledUnit::Extract(Expr) | Pass | Compiles and holds correct Expr structure |
| AC-4: Display round-trip | Pass | s-expression format: `extract (Expr.Add (Expr.Const 0i32) (Expr.Const 4i32))` |
| AC-5: Atom literals work | Pass | `extract 42i32` → AtomOrVariable::Atom(I32(42)) |
| AC-6: Variable name in extract | Pass | `extract x` → AtomOrVariable::Variable("x") |

## Verification Results

```
cargo test -p quine-core -p quine-frontend -p quine
  20 tests (quine CLI): all pass
   5 tests (quine-core lattice): all pass
  10 tests (quine-core reverse_index): all pass
Total: 35/35 passing, 0 failures, 0 regressions
```

## Accomplishments

- Grammar `extract_query` simplified from `"extract" ~ heads ~ "print" ~ "(" ~ variable ~ ("," ~ variable)* ~ ")"` to `"extract" ~ expr`
- Parser now calls `parse_expr()` instead of `parse_heads()` + collecting print variables
- `Command::Extract(Heads, Vec<String>)` → `Command::Extract(Expr)` — carries the concrete value
- `CompiledUnit::Extract(rule::Query, Vec<String>)` → `CompiledUnit::Extract(Expr)` — no query compilation needed
- Compilation path simplified: wraps Expr directly, no longer calls `heads2query`
- All 6 extract tests rewritten for new syntax; old `print(vars)` syntax correctly rejected
- README updated with new syntax examples

## Files Changed

| File | Change | Purpose |
|------|--------|---------|
| `docs/grammar.pest` | Modified | `extract_query` now `"extract" ~ expr` |
| `quine-cli/src/pest_parser.rs` | Modified | Parse `expr` instead of `heads` + variables |
| `quine-frontend/src/syntax.rs` | Modified | `Command::Extract(Expr)`, Display updated |
| `quine-frontend/src/compile/mod.rs` | Modified | Wrap Expr directly, no heads2query |
| `quine-frontend/src/lib.rs` | Modified | `CompiledUnit::Extract(Expr)`, `Expr` import added |
| `quine-cli/tests/syntax_tests/extract.rs` | Modified | 6 tests rewritten for new syntax |
| `README.md` | Modified | Extract section shows `extract <expr>` |

## Decisions Made

| Decision | Rationale | Impact |
|----------|-----------|--------|
| extract uses `expr` (concrete values), not `heads` (patterns) | Extraction needs a concrete expression to find in the e-graph, not pattern matching | Simplified grammar, parser, and compilation |
| No type-checking during extract compilation in Phase 4 | Deferred to Phase 5 when expression evaluation is implemented | `CompiledUnit::Extract(Expr)` is a simple wrapper |
| `CompiledUnit::Extract` remains a no-op | Extraction execution is Phase 5 scope | applay() just stores the Expr |

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

| Issue | Resolution |
|-------|------------|
| AC-6 test used `some_value` (with underscore) — rejected by parser | Underscores are not valid in `variable_char`; changed test to use `x` |

## Next Phase Readiness

**Ready:**
- New `extract <expr>` syntax fully parsed, compiled, and tested
- `CompiledUnit::Extract(Expr)` ready for Phase 5 to implement cost-aware extraction
- `Expr` type already supports nested constructor calls and literals

**Concerns:**
- Phase 5 will need to evaluate `Expr` against the e-graph (find eclass, use cost_select, materialize cheapest term)
- Expression type-checking (validating constructors exist) should be added in Phase 5 compilation

**Blockers:** None

---
*Phase: 04-change-extraction-syntax, Plan: 01*
*Completed: 2026-06-03*
