# Combinators

Biparse's parser combinator ecosystem is primarily inspired by that of `chumsky`, which frontloads cognitive complexity onto the type system through trait implementations and dispatch. The actual runtime values of the parser combinators themselves tend to be lightweight.

Therefore, the intended usage pattern is that of a parser-producing function:
```rs,ignore
fn make_parser() -> impl Parser<...> { ... }
```

Refer to the [`Input Comparison`](./../input_comparison/input_comparison.md) chapter for further details and the exact form of the function signatures for each input type.

## Note on Method Chaining and Nesting

Most non-leaf combinators are constructed via `ParserExt` method chaining, and is the preferred method of construction over direct usage of the corresponding combinator types. This means that following Rust's evaluation rules, method chaining is left-associative.

This is most obviously seen in chaining of `then` sequencing parsers and `either_or` alternative parsers, which produce left-nested data structures:
- `a.then(b).then(c)` is equivalent to `(a.then(b)).then(c)`, producing a payload of type `((A, B), C)`.
- `a.then(b.then(c))` produces a payload of type `(A, (B, C))`. Isomorphic but not identical to the other two forms.