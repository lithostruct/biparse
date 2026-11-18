# Repetition

Repetition involves parsing the same constructs multiple times.

## `repeat_by`
*Docs.rs documentation: [`ParserExt::repeat_by`], [`combinator::Repetition::new`].*

Some number of repetitions of the inner parser, using the bounds described by a [`range : RangeBounds<usize>`](std::ops::RangeBounds) argument.

`range.start_bound()` represents the minimum number of repetitions (inclusive) required for an overall success. Failure to reach this number results in a [`ParseOutcome::Speculative`] failure.  
`range.end_bound()` represents the maximum number of repetitions (inclusive) admitted by this repetition construct. The combinator will not attempt to parse more repetitions beyond this bound.  
An unbounded start range eg. `..` is automatically normalized with a start bound of [`usize::MIN`] ie. `0..`.

Eg. a parser of `a.repeat_by(n..=m)` produces a [`Vec<A>`](std::vec::Vec), whose length is bounded by the range `n..=m`. The bound on the output type applies for any [`RangeBounds<usize>`](std::ops::RangeBounds) implementor, including those with unbounded start ranges.

The range passed *must* be contain at least one `usize` element upon normalization. Construction of this combinator panics otherwise.

The inner parser *must* advance the input by some non-zero amount each iteration. Failure to do so is a parser invariant violation, and produces a [`ParseOutcome::Fatal`] failure upon parse.

## `repeat`
*Docs.rs documentation: [`ParserExt::repeat`].*

Exactly `n` repetitions of the inner parser. Syntax sugar over [`.repeat_by(n..=n)`](#repeat_by).  
A parser of `a.repeat(n)` has AST payload type [`Vec<A>`](std::vec::Vec), which is guaranteed to contain `n` elements.

This parser inherits the invariant of [`repeat_by`](#repeat_by) that the inner parser must be advancing, otherwise a `Fatal` failure is produced.

## `many`
*Docs.rs documentation: [`ParserExt::many`].*

Zero or more repetitions of the inner parser. Syntax sugar over [`.repeat_by(0..)`](#repeat_by).  
A parser of `a.many()` has AST payload type [`Vec<A>`](std::vec::Vec).

This parser inherits the invariant of [`repeat_by`](#repeat_by) that the inner parser must be advancing, otherwise a `Fatal` failure is produced.

## `some`
*Docs.rs documentation: [`ParserExt::some`].*

One or more repetitions of the inner parser.  Syntax sugar over [`.repeat_by(1..)`](#repeat_by).  
A parser of `a.some()` has AST payload type [`Vec<A>`](std::vec::Vec), which is guaranteed to be non-empty.

This parser inherits the invariant of [`repeat_by`](#repeat_by) that the inner parser must be advancing, otherwise a `Fatal` failure is produced.

## `optional`
*Docs.rs documentation: [`ParserExt::optional`].*

Zero or one repetitions of the inner parser. A parser of `a.optional()` has AST payload type [`Option<A>`](std::option::Option).

Unlike [`repeat_by`](#repeat_by)-derived combinators, the inner parser is allowed to be non-advancing. This is because `a.optional()` is actually syntactic sugar over `a.either_or(success()).map(Either::left)`. See the documentation on [`either_or`](./22_alternatives.md#either_or) for further details.

Note that `a.repeat_by(0..=1)` differs from `a.optional()` in that it is forbidden to be non-advancing, producing a [`ParseOutcome::Fatal`] failure otherwise.