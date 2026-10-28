# Composite Combinators

## Sequencing

Sequencing involves parsing constituent elements in sequential order. All of the parsers must succeed to be considered an overall success.

### `then`

Binary sequencing of two constituent parsers. A parser of `a.then(b)` has AST payload type `(A, B)`.

Note that the tuple payload type is not automatically flattened. Chained `then` parsers will produce right-nested 2-tuples eg. `a.then(b).then(c).then(d)` has payload type `(A, (B, (C, D)))`.

## Alternatives

Alternative parsing involves choosing between valid paths of parsing, where any of the parsers succeeding results in an overall success.

Alternatives are *ordered*; Biparse represents a Parsing Expression Grammar (PEG) rather than a Context-Free Grammar (CFG). 
Therefore, if `a` within `a.or(b)` succeeds first, the parsing path of `b` will not be explored. Hence, it is recommended to order alternatives from high to low specificity.

### `or`

Binary alternative between two constituent parsers. A parser of `a.or(b)` has AST payload type `Either<A, B>`.

The Either type is re-exported from the `either` crate, and represents one of two possible payload types. It can be thought of as a `Result` without the conceptual semantics of success/error.


### `or_same`

Syntactic sugar over `or` which flattens the Either value into one `A` payload value.

The constituent parsers `a` and `b` of `a.or_same(b)` must have the same AST payload type `A`.

## Repetition

Repetition involves parsing the same constructs multiple times, 

### `many`

Zero or more repetitions of the inner parser. A parser of `a.many()` has AST payload type `Vec<A>`.

### `some`

One or more repetitions of the inner parser. A parser of `a.some()` has AST payload type `Vec<A>`, which is guaranteed to be non-empty.

### `optional`

Zero or one repetitions of the inner parser. A parser of `a.optional()` has AST payload type `Option<A>`.