# Repetition

Repetition involves parsing the same constructs multiple times.

## `repeat_by`

Some number of repetitions of the inner parser, using the bounds described by a [`range : RangeBounds<usize>`](std::ops::RangeBounds) argument.

`range.start_bound()` represents the minimum number of repetitions (inclusive) required for an overall success. Failure to reach this number results in a [`Speculative` failure](../../parse_output.md#speculative).  
`range.end_bound()` represents the maximum number of repetitions (inclusive) admitted by this repetition construct. The combinator will not attempt to parse more repetitions beyond this bound.

Eg. a parser of `a.repeat_by(n..=m)` produces a [`Vec<A>`](std::vec::Vec), whose length is bounded by the range `n..=m`. The range passed *must* be non-empty, otherwise this combinator panics upon construction.

The inner parser *must* advance the input by some non-zero amount each iteration. Failure to do so is an invariant violation, and produces a [`Fatal` failure](../../parse_output.md#fatal).

## `repeat`

Exactly `n` repetitions of the inner parser. Syntax sugar over [`.repeat_by(n..=n)`](#repeat_by).  
A parser of `a.repeat(n)` has AST payload type [`Vec<A>`](std::vec::Vec), which is guaranteed to contain `n` elements.

This parser inherits the invariant of [`repeat_by`](#repeat_by) that the inner parser must be advancing, otherwise a `Fatal` failure is produced.

## `many`

Zero or more repetitions of the inner parser. Syntax sugar over [`.repeat_by(0..)`](#repeat_by).  
A parser of `a.many()` has AST payload type [`Vec<A>`](std::vec::Vec).

This parser inherits the invariant of [`repeat_by`](#repeat_by) that the inner parser must be advancing, otherwise a `Fatal` failure is produced.

## `some`

One or more repetitions of the inner parser.  Syntax sugar over [`.repeat_by(1..)`](#repeat_by).  
A parser of `a.some()` has AST payload type [`Vec<A>`](std::vec::Vec), which is guaranteed to be non-empty.

This parser inherits the invariant of [`repeat_by`](#repeat_by) that the inner parser must be advancing, otherwise a `Fatal` failure is produced.

## `optional`

Zero or one repetitions of the inner parser. A parser of `a.optional()` has AST payload type [`Option<A>`](std::option::Option).

Unlike [`repeat_by`](#repeat_by)-derived combinators, the inner parser is allowed to be non-advancing. This is because `a.optional()` is actually syntactic sugar over `a.either_or(success()).map(Either::left)`. See the documentation on [`either_or`](./22_alternatives.md#either_or) for further details.

Note that `a.repeat_by(0..=1)` differs from `a.optional()` in that it is forbidden to be non-advancing, producing a [`Fatal` failure](../../parse_output.md#fatal) otherwise.