# Combinators

Biparse's parser combinator ecosystem is primarily inspired by that of `chumsky`, which frontloads cognitive complexity onto the type system through trait implementations and dispatch. The actual runtime values of the parser combinators themselves tend to be lightweight.

Therefore, the intended usage pattern is that of a parser-producing function:
```rs,ignore
fn make_parser() -> impl Parser<...> { ... }
```

Refer to the [`Input Comparison`](./../input_comparison/input_comparison.md) chapter for further details and the exact form of the function signatures for each input type.