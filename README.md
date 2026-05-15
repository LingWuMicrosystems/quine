# Quine

Relation-graph match-rewrite engine — e-graph equality saturation with datalog-style semi-naïve evaluation.

## Usage

```bash
quine                    # Start REPL
quine file.quine         # Execute a file
quine -e "source"        # Execute inline source, then enter REPL
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
function add(i32, i32) -> i32
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
query edge(_, y), if y > 0
query path(x, _)
```

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
