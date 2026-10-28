# Comparison between `Input` Types

The Biparse parser combinator ecosystem specializes according to a suitable input type implementing the `Input` trait. While all input types have similar expressivity, choosing a suitable input type may help with subsequent writing and composition of combinators. This document describes some of the tradeoffs between existing `Input` types.

For a concrete example, refer to sibling files [`json_str.rs`](/tests/integration/json_str.rs) and [`json_logos.rs`](/tests/integration/json_logos.rs).

Note that you may define your own input types, so long as you have a suitable `Input` implementor and corresponding `Pattern` implementors for `just` combinators.

## Overview on `Input` and `Pattern` Traits

A type implementing `Input` describes a conceptually contiguous UTF-8 `str` input, that may be indexed into using either an associated cursor type `Input::Cursor` or a byte offset. Note that storage and actualization of the string itself need not be contiguous; you may use interior mutability to provide `Cow<'_, str>` substrings as exposed by slicing operations. This input is borrowed immutably for the entire parse operation.

A type implementing `Pattern<'src, I>` for some `I : Input` describes a predicate that checks whether an input, borrowed for lifetime `'src`, at the specified location "matches" it. For example, `char` performs a character match against a `str` input, which may succeed if the location has the same `char` value or fail otherwise, returning a `&str` borrowing from the input.

All `I : Input` implementors may use `Matches<impl Into<SyntaxKind>, impl Fn(&I::Token<'_>) -> bool>` as a universal `Pattern<'_, I>` implementor. This fills the role of the boolean predicate for whether a token should be accepted as a match.

All `Pattern<'_, I>` implementors may use `AllOf` and `AnyOf` to compose sequential and alternative matching. 