# Input Types

The Biparse parser combinator ecosystem specializes according to a suitable input type implementing the `Input` trait. While all input types have similar expressivity, choosing a suitable input type may help with subsequent writing and composition of combinators. This section describes some of the tradeoffs between existing `Input` types.

Note that you may add your own input types! So long as you have a suitable `Input` implementor and corresponding `Pattern` implementors for `just` combinators.

## Trait Overview 

### `Input`

A type implementing `Input` describes a conceptually contiguous UTF-8 `str` input, that may be indexed into using either an associated cursor type `Input::Cursor` or a byte offset. Note that storage and actualization of the string itself need not be contiguous; you may use interior mutability to provide `Cow<'_, str>` substrings as exposed by slicing operations. An input value is borrowed immutably for an entire parse operation.

### `Pattern`

A type implementing `Pattern<'src, I>` for some `I : Input` describes a predicate that checks whether an input, borrowed for lifetime `'src`, at the specified location "matches" it. For example, `char` performs a character match against a `str` input, which may succeed if the location has the same `char` value or fail otherwise, returning a `&str` borrowing from the input.

All `Pattern` implementors must specify a `rowan::SyntaxKind` value that a match corresponds to. This is used for total CST construction.

A universal pattern exists in the form of `Matches`: all `I : Input` implementors may use `Matches<impl Into<SyntaxKind>, impl Fn(&I::Token<'_>) -> bool>` as an out-the-box supported pattern. This fills the role of the boolean predicate for whether a token should be accepted as a match.

All `Pattern<'_, I>` implementors may use `AllOf` and `AnyOf` to compose sequential and alternative matching.

## Note on Lifetime-Bound Inputs

Due to the constructed parser object capturing the lifetime of the input in its `I: Input` field, it *must* be deallocated before the input object itself.

This also means that following Rust's default drop order, the input object should be constructed *before* the parser object. This is particularly relevant of `str`-derived input types [`CachedStr`](./1_input_str.md#variant-cachedstrsource) and [`SkippingStr`](./3_input_skippingstr.md), which are constructed from `&str` references.

Hence, rather counterintuitively, this won't compile:

```rust,ignore
use biparse::{
    prelude::*,
    input::skipping_str::SkippingStr,
};
use std::rc::Rc;

struct Ast {
    // ...
}
```
```rust,ignore
fn parse(text: &str) {
    let parser : Rc<dyn Parser<'_, SkippingStr<'_>, Ast, DefaultParseEnvironment>> = /* ... */ ;
    let input = SkippingStr::whitespace(text, SyntaxKind(0));

    parser.parse_all(&input);
    // input dropped before parser!
}
```

But this will:

```rust,ignore
fn parse(text: &str) {
    // reversed order:
    let input = SkippingStr::whitespace(text, SyntaxKind(0));
    let parser : Rc<dyn Parser<'_, SkippingStr<'_>, Ast, DefaultParseEnvironment>> = /* ... */ ;

    parser.parse_all(&input);
    // parser dropped, then input dropped. Fine.
}
```