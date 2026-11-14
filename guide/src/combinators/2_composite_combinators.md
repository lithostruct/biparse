# Composite Combinators

This section describes combinators formed via composition of smaller parsers.

## Sequencing

Sequencing involves parsing constituent elements in sequential order. All of the parsers must succeed to be considered an overall success.

### `then`

Binary sequencing of two constituent parsers. A parser of `a.then(b)` has AST payload type `(A, B)`.

Note that the tuple payload type is not flattened. Chained `then` parsers will produce left-nested 2-tuples eg. `a.then(b).then(c)` has payload type `((A, B), C)`.

### `ignore_then`, `then_ignore`

Combinators `ignore_then` and `then_ignore` are syntactic sugar over [`then`](#then), discarding the left and right AST value respectively, producing the other parser's AST value without tuple wrapping. This is commonly used for syntactic delimiters eg. parentheses, or whitespace.

## Alternatives

Alternative parsing involves choosing between valid paths of parsing, where any of the parsers succeeding results in an overall success.

Alternatives are *ordered*; if `a` within `a.or(b)` succeeds first, the parsing path of `b` will not be explored. Hence, it is recommended to order alternatives from high to low specificity.

### `or`

Binary alternative between two constituent parsers. A parser of `first.or(second)` has AST payload type `A`. Both parsers must produce an AST payload of type `A`.

### `either_or`

Syntactic sugar over `or`, distinguishing each parsing path as a sum type. A parser of `a.or(b)` has AST payload type `Either<A, B>`. `A` and `B` may be different types.

The Either type is re-exported from the [`either`](https://crates.io/crates/either) crate, and represents one of two possible payload types. It can be thought of as a `Result` without the conceptual semantics of success/error. The first parsing path from parser `a` is represented by `Either::Left(_)`, and the second parsing path from `b` is `Either::Right`.

Note that similarly to [`then`](#then), `a.either_or(b).either_or(c)` produces a left-nested `Either<Either<A, B>, C>` payload.

## Repetition

Repetition involves parsing the same constructs multiple times, 

### `many`

Zero or more repetitions of the inner parser. A parser of `a.many()` has AST payload type `Vec<A>`.

The inner parser *must* advance the input by some non-zero amount each iteration. Failure to do so is an invariant violation, and produces a `Fatal` failure.

### `some`

One or more repetitions of the inner parser. Syntactic sugar over [`many`](#many). A parser of `a.some()` has AST payload type `Vec<A>`, which is guaranteed to be non-empty.

The inner parser *must* advance the input by some non-zero amount each iteration. Failure to do so is an invariant violation, and produces a `Fatal` failure.

### `optional`

Zero or one repetitions of the inner parser. A parser of `a.optional()` has AST payload type `Option<A>`.

Unlike the [`many`](#many) and [`some`](#some) combinators, the inner parser may be non-advancing.