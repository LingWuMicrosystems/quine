# Quine

Relation-graph match-rewrite engine — e-graph equality saturation with datalog-style semi-naïve evaluation.

## Usage

```bash
quine                         # Start REPL
quine file.quine              # Execute a file as main entry point
quine dir/                    # Pre-scan directory, then REPL
quine dir/ --main name        # Pre-scan directory, execute named module as main
```

### Module loading (like rustc)

`load` statements in `.quine` files bring in declarations from other files:

```
load "irtype"           # bare name → looks for irtype.quine in same directory
load "./sub/extra.quine"  # path with / or . → resolved as-is
```

Resolution:
1. Bare name → check pre-scanned `module_map` (only populated when `quine dir/` is used).
2. Bare name not in map → look for `name.quine` relative to the loading file's directory (like `mod foo` → `foo.rs`).
3. Path-like (contains `.` or `/`) → resolve directly as a file path.

Loaded modules may only contain **pure declarations**: `data`, `relation`, `function`, `rule`, `cost`, and nested `load`. Side-effecting commands (`fact`, `run`, `query`, `extract`) are rejected in loaded modules — they belong in the main file only.

Duplicate loads (by canonical path) are silently skipped; circular loads are handled correctly.

### REPL

| Input | Description |
|-------|-------------|
| `exit`, `quit` | Exit REPL |
| `:load "file.quine"` | Load and execute a file (always reloads) |
| `data`, `rule`, `fact`, … | Any valid command, executed immediately |
| Multi-line input | Entered automatically when a statement is incomplete (unclosed `{`, `(`, etc.) |

## Syntax

### Types

```
data Option = Some(value) | None
```

### Tables (Relations)

```
relation edge(i32, i32)
relation node(i32)
```

### Functions

```
function add(i32, i32) -> i32 merge min
```

### Facts

```
fact set edge(1i32, 2i32)
fact set edge(2i32, 3i32)
```

### Rules

```
rule edge(x, y) { set path(x, y) }
rule edge(x, z), path(z, y) { set path(x, y) }
```

### Guards

```
rule edge(x, y), if x > 0 { set positive_edge(x, y) }
rule edge(x, y), if x != y { set distinct(x) }
```

### Unions

```
rule node(x), node(y), leteq x = y { union x with y }
```

### Query

```
query edge(_, y), if y > 0i32 print(y)
query path(x, _) print(x)
```

### Cost Models

```
cost Option.Some = 2
cost Option.None = 0
cost Expr.Add = 1
cost Expr.Mul = 2
```

Define integer costs for data type constructors. The cost of an expression is the sum of costs of all constructors in the tree. Constructors without a defined cost default to 0. Only data type constructors (`TypeName.ConstructorName`) can have costs; relations and functions cannot.

### Cost Analysis

Expression costs are computed incrementally during e-graph saturation using a lattice fixpoint:

```
Lattice: (u64, min, u64::MAX)
- Each eclass has a current minimum cost, starting at u64::MAX (unknown)
- Costs monotonically decrease as cheaper equivalent expressions are discovered
- Join operation = min (the cheaper of two equivalent expressions)
```

Costs are maintained eagerly at every insert and union operation. The cheapest enode for each eclass is tracked so extraction can select the lowest-cost expression.

### Extract

```
extract Expr.Add(Expr.Const(0i32), Expr.Const(4i32))
extract optimal Expr.Add(Expr.Const(0i32), Expr.Const(4i32))
```

Extract the lowest-cost expression equivalent to the given value from the e-graph. Provide a concrete expression (constructor calls with literal arguments), and the system uses defined cost models to find and return the cheapest equivalent form.

Two extraction modes:

| Mode | Syntax | Algorithm | Output |
|------|--------|-----------|--------|
| Greedy | `extract <expr>` | DP on cost_select | Cheapest per-eclass (may be suboptimal when sub-expressions are shared) |
| Optimal | `extract optimal <expr>` | B&B-CR ILP solver | Globally optimal, accounts for common sub-expression costs |

The `extract optimal` solver uses Branch-and-Bound with Combinatorial Relaxation (B&B-CR): drops CSE coupling to form a DAG shortest-path relaxation (lower bound), then branches on shared eclasses to find the global optimum.

Shared sub-expressions (eclasses referenced by multiple parent enodes) are automatically bound with `let` in the output to avoid expression duplication:

```
(let ([_t0 (+ a b)]) (f _t0 _t0))
```

### Run

```
run saturate
run repeat 10
```

Triggers e-graph saturation: applies all rules until no new facts are produced (or for a fixed number of iterations).

### Base Types

`bool`, `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, `f64`, `str`

## Build

```bash
nix develop
cargo build --release
```
