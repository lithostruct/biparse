# Primitives and Leaf Combinators

## Note on `AsParser<...>` Wrapping

Type inference tends to have a hard time inferring the correct trait implementation for leaf combinators due to their wide applicability, particularly all primitive combinators which work on *any* input and parse environment. Hence, the transparent `AsParser<'src, I, A, E, P>` wrapper is often used when construcing leaf combinators through the construction functions (eg. `success()` rather than the `Success` type), which fixes the inner value to a specific `Parser` implementation.

### `success`

Unconditionally succeeds, returning the unit value `()`.

### `fail`

Unconditionally fails, returning a speculative failure.

This parser is considered to have an AST payload type of `()`.

### `end`

Succeeds if at the end of input, returning the unit value `()`.

## `just`

Succeeds upon a pattern match by a compatible `Pattern` implementor, returning a corresponding `Pattern::Payload<'src>`. For example, a regex pattern matching against a `str` input returns a `&str` substring into the borrowed input on success.

Refer to [`Input Comparison`] for more details on specific `Input` and `Pattern` implementors.