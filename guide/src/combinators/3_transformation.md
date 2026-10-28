# AST and CST Transformation

## AST Transformation

### `map`

### `into_owned`

### `try_map`

## CST Transformation

A green CST forest is automatically accumulated during parsing. Without interference, it will end up parsing as a flat forest, but transforming it along the syntax rules of your grammar takes little effort. To maintain correspondence invariants, CST transformation is more limited and pre-defined compared to AST transformation.

### `node`

### `associate`

### `recover`