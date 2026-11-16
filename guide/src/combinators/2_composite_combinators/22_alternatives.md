

## Alternatives

Alternative parsing involves choosing between valid paths of parsing, where any of the parsers succeeding results in an overall success.

Alternatives are *ordered*; if `a` within `a.or(b)` succeeds first, the parsing path of `b` will not be explored. Hence, it is recommended to order alternatives from high to low specificity.

### `or`

Binary alternative between two constituent parsers. A parser of `first.or(second)` has AST payload type `A`. Both parsers must produce an AST payload of type `A`.

### `either_or`

Syntactic sugar over `or`, distinguishing each parsing path as a sum type. A parser of `a.either_or(b)` has AST payload type `Either<A, B>`. `A` and `B` may be different types.

The Either type is re-exported from the [`either`] crate, and represents one of two possible payload types. It can be thought of as a `Result` without the conceptual semantics of success/error. The first parsing path from parser `a` is represented by [`Either::Left`](either::Either::Left), and the second parsing path from `b` is [`Either::Right`](either::Either::Right).

Note that similarly to [`then`](#then), `a.either_or(b).either_or(c)` produces a left-nested `Either<Either<A, B>, C>` payload.
