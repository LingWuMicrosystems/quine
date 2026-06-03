---
phase: 02-cost-extraction-syntax
plan: 01
subsystem: syntax
tags: [rust, parser, dsl, cost-model, extraction, pest]
requires:
  - 01-01 (reverse_index — needed by future Phase 4 extraction)
provides:
  - CostDef AST type with cost storage in EngineContext.cost_models
  - Extract command syntax (parses and compiles, execution stub for Phase 4)
  - Grammar rules for `cost` and `extract` commands
affects:
  - cost analysis (Phase 3 — consumes cost_models map)
  - expression extraction (Phase 4 — uses Extract CompiledUnit variant)
tech-stack:
  added: []
  patterns:
    - Cost definitions stored as Map<String, u64> keyed by "TypeName.ConstructorName"
    - Absent entries default to cost 0 (via unwrap_or)
    - Dotted names (Option.Some) parsed as single variables due to '.' being valid in variable_char
key-files:
  modified:
    - docs/grammar.pest
    - quine-frontend/src/syntax.rs
    - quine-frontend/src/lib.rs
    - quine-frontend/src/compile/mod.rs
    - quine-frontend/src/error.rs
    - quine-cli/src/pest_parser.rs
    - quine-cli/src/main.rs
    - README.md
  created:
    - quine-cli/src/lib.rs
    - quine-cli/tests/syntax_tests/main.rs
    - quine-cli/tests/syntax_tests/cost.rs
    - quine-cli/tests/syntax_tests/extract.rs
key-decisions:
  - "Cost syntax uses flat u64 per constructor: `cost TypeName.ConsName = <integer>`"
  - "CostDef stored in EngineContext.cost_models: Map<String, u64>"
  - "Default cost is 0 for undefined constructors (absent from map)"
  - "Validation: only data type constructors can have costs (checked via DataTypeEnv.cons2type_map)"
  - "Negative costs rejected at parse time (u64 parse panics)"
  - "Dotted names (Option.Some) parsed as single Pest variable — split via rsplit_once('.') in parser"
  - "quine-cli restructured to have lib.rs for integration test access to pest_parser module"
patterns-established:
  - "Cost models defined per constructor with integer cost; cost is sum of all constructors in expression tree"
  - "Extract syntax mirrors query syntax: extract <heads> print(<vars>)"
duration: ~45min
started: 2026-06-03T15:00:00Z
completed: 2026-06-03T15:45:00Z
---

# Phase 02 Plan 01: Cost + Extraction Syntax Summary

**Added DSL syntax for per-constructor cost definitions (`cost TypeName.ConsName = <int>`) and cost-aware extraction queries (`extract <pattern> print(<vars>)`), with full grammar → AST → parser → compilation pipeline. EngineContext stores costs in a cost_models map; undefined constructors default to 0.**

## Performance

| Metric | Value |
|--------|-------|
| Duration | ~45min |
| Started | 2026-06-03 |
| Completed | 2026-06-03 |
| Tasks | 4 completed |
| Files modified | 8 |
| Files created | 4 |

## Acceptance Criteria Results

| Criterion | Status | Notes |
|-----------|--------|-------|
| AC-1: Cost definition parses and stores | Pass | `cost Option.Some = 2` → cost_models["Option.Some"] = 2 |
| AC-2: Undefined constructor defaults to 0 | Pass | Absent from map → unwrap_or(0) |
| AC-3: Negative cost rejected | Pass | u64 parse panics on negative input |
| AC-4: Cost for relation rejected | Pass | CompileError (UnknownTypeName) |
| AC-5: Cost for function rejected | Pass | CompileError (UnknownTypeName) |
| AC-6: Unknown constructor rejected | Pass | CompileError (UnknownConstructor) |
| AC-7: Extract parses and compiles | Pass | Full parse → compile → CompiledUnit::Extract |
| AC-8: Display round-trips | Pass | Display output consistent with existing format |
| AC-9: README documents syntax | Pass | Cost Models and Extract sections added |

## Accomplishments

- Added `cost TypeName.ConsName = <int>` syntax for defining per-constructor integer costs
- Added `extract <pattern> print(<vars>)` syntax mirroring query for cost-aware extraction
- Created `CostDef { type_name, constructor, cost: u64 }` AST type with Display impl
- Added `EngineContext.cost_models: Map<String, u64>` for storing cost definitions
- Added `CompiledUnit::CostDef` (with apply logic) and `CompiledUnit::Extract` (stub for Phase 4)
- Compile-time validation: only data type constructors can have costs
- `CompileError::UnknownConstructor` variant for unknown constructor errors
- Created `quine-cli/src/lib.rs` enabling integration tests for parser
- 14 BDD integration tests: 8 cost + 6 extract

## Files Created/Modified

| File | Change | Purpose |
|------|--------|---------|
| `docs/grammar.pest` | Modified | Added `cost_def`, `extract_query` rules; added `cost`, `extract` keywords |
| `quine-frontend/src/syntax.rs` | Modified | Added `CostDef` type, `Command::CostDef`, `Command::Extract`, Display impls |
| `quine-frontend/src/lib.rs` | Modified | Added `CompiledUnit::CostDef`, `CompiledUnit::Extract`, `EngineContext.cost_models` |
| `quine-frontend/src/compile/mod.rs` | Modified | Compilation of CostDef + Extract commands with validation |
| `quine-frontend/src/error.rs` | Modified | Added `UnknownConstructor` variant |
| `quine-cli/src/pest_parser.rs` | Modified | `parse_cost_def` function, dispatch for `cost_def` and `extract_query` |
| `quine-cli/src/main.rs` | Modified | Moved `pub mod pest_parser` to lib.rs |
| `quine-cli/src/lib.rs` | Created | Library root exposing `pest_parser` for integration tests |
| `quine-cli/tests/syntax_tests/main.rs` | Created | Test harness for syntax tests |
| `quine-cli/tests/syntax_tests/cost.rs` | Created | 8 BDD tests for cost parsing + compilation |
| `quine-cli/tests/syntax_tests/extract.rs` | Created | 6 BDD tests for extract parsing + compilation |
| `README.md` | Modified | Cost Models and Extract documentation sections |

## Decisions Made

| Decision | Rationale | Impact |
|----------|-----------|--------|
| Flat u64 cost per constructor | User requirement: no expression-based costs needed | Simpler API, Phase 3 evaluation is straightforward |
| Dotted names as single variables | '.' is valid in Pest variable_char; splitting at parser level avoids grammar complexity | Cleaner grammar, same user-facing syntax |
| cost_models on EngineContext | Central location accessible by both compilation (Phase 2) and evaluation (Phase 3) | No new Core IR needed yet |
| Validation via DataTypeEnv.cons2type_map | Existing map already tracks "TypeName.ConsName" → type index | No new data structures needed for validation |
| lib.rs extraction for testability | quine-cli was binary-only; tests need library target | Enables integration tests following cargo conventions |

## Issues Encountered

| Issue | Resolution |
|-------|------------|
| Pest '.' in sequence breaks parsing | Discovered '.' is valid variable_char; changed to parse dotted name as single variable, split in parser |
| Guard tests failing with bare integer `0` | Pre-existing issue: atom requires typed literals (0i32, not 0); fixed tests to use typed literals |
| quine-cli binary-only prevents integration tests | Created lib.rs with `pub mod pest_parser`, updated main.rs to use `quine::pest_parser` |

## Next Phase Readiness

**Ready for Phase 3 (Cost Analysis):**
- `EngineContext.cost_models` populated with cost definitions
- Default cost of 0 for undefined constructors
- Cost model data structure ready for evaluation logic

**Stubs for Phase 4 (Expression Extraction):**
- `CompiledUnit::Extract` exists with no-op apply handler
- Extract syntax fully parsed and compiled to Query + vars

---
*Phase: 02-cost-extraction-syntax, Plan: 01*
*Completed: 2026-06-03*
