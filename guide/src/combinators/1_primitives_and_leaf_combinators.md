# Primitives and Leaf Combinators

The leaves or terminals upon which [composite combinators](./2_composite_combinators/index.md) build upon.

## Note on `As<...>` Wrapping
*Docs.rs documentation: [`combinator::As`].*

Type inference tends to have a hard time inferring the correct trait implementation for leaf combinators due to their wide applicability. Hence, the transparent `As<'src, I, A, E, P>` wrapper is often used when constructing leaf combinators through the construction functions (eg. `success()` rather than the `Success` type), which fixes the inner value to a specific `Parser<'src, I, A, E>` implementation.

## `success`
*Docs.rs documentation: [`combinator::success`], [`combinator::Success`].*

Unconditionally succeeds, returning the unit value `()`.

## `fail`
*Docs.rs documentation: [`combinator::fail`], [`combinator::Fail`].*

Unconditionally fails, returning a speculative failure.

The output type of this parser is unconstrained, and must be specified upon construction. It can be any type, up to and including the never type [`!`](primitive@never).

## `fail_with`
*Docs.rs documentation: [`combinator::fail_with`], [`combinator::Fail`].*

Identical to [`fail`](#fail), but the user can specify a message to transmit upon failure.

## `end`
*Docs.rs documentation: [`combinator::end`], [`combinator::End`].*

Succeeds if at the end of input, returning the unit value `()`. Otherwise, returns a speculative failure.

## `just`
*Docs.rs documentation: [`combinator::just`], [`combinator::Just`].*

Succeeds upon a pattern match by a compatible [`Pattern`] implementor, returning a corresponding [`Pattern::Payload<'src>`]. Otherwise, returns a speculative failure.

For example, a regex pattern matching against a [`str`] input returns a `&str` substring into the borrowed input on success.

Refer to the [`Input Types`](../input_comparison/index.md) chapter for more details on specific `Input` and `Pattern` implementors.