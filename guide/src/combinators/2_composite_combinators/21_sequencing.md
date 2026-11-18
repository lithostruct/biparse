# Sequencing

Sequencing involves parsing constituent elements in sequential order.

All of the parsers must succeed to be considered an overall success, and Biparse typically implements short-circuiting semantics. In other words, early failure results in the entire collection failing.

## `then`
*Docs.rs documentation: [ParserExt::then], [`combinator::Then`].*

Binary sequencing of two constituent parsers. A parser of `a.then(b)` has AST payload type [`(A, B)`](tuple).

Note that the tuple payload type is not flattened. Chained `then` parsers will produce left-nested 2-tuples eg. `a.then(b).then(c)` has payload type `((A, B), C)`.

## `ignore_then`, `then_ignore`
*Docs.rs documentation: [`ParserExt::ignore_then`], [`ParserExt::then_ignore`].*

Combinators `ignore_then` and `then_ignore` are syntactic sugar over [`then`](#then), discarding the left and right AST value respectively, producing the other parser's AST value without tuple wrapping.

This is commonly used for syntactic delimiters eg. parentheses, or whitespace.