# Quine

Relation-graph match-rewrite engine — e-graph equality saturation with datalog-style semi-naïve evaluation.

## Usage

```bash
quine                    # Start REPL
quine file.quine         # Execute a file
```

### REPL Meta-commands

| Command | Description |
|---------|-------------|
| `:exit`, `:quit`, `:q` | Exit REPL |
| `:load "file.quine"` | Load and execute a file |

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
fact set edge(1, 2)
fact set edge(2, 3)
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
extract 42i32
```

Extract the lowest-cost expression equivalent to the given value from the e-graph. Provide a concrete expression (constructor calls with literal arguments), and the system uses defined cost models to find and return the cheapest equivalent form.

### Run

```
run
```

Triggers e-graph saturation: applies all rules until no new facts are produced.

### Base Types

`bool`, `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, `f64`, `str`

## Build

```bash
cargo build --release
```
