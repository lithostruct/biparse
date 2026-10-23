# Comparison between `Input` Types

The `biparse` parser combinator ecosystem specializes according to a suitable input type implementing the `Input` trait. While all input types have similar expressivity, choosing a suitable input type may help with subsequent writing and composition of combinators. This document describes some of the tradeoffs between existing `Input` types.

For a concrete example, refer to sibling files [`json_str.rs`](/tests/integration/json_str.rs) and [`json_logos.rs`](/tests/integration/json_logos.rs).

Note that you may define your own input types, so long as you have a suitable `Input` implementor and corresponding `Pattern` implementors for `just` combinators.

## Overview on `Input` and `Pattern` Traits

A type implementing `Input` describes a conceptually contiguous UTF-8 `str` input, that may be indexed into using either an associated cursor type or a byte offset. Note that storage and actualization of the string itself need not be contiguous; you may use interior mutability to provide `Cow<'_, str>` substrings as exposed by slicing operations. This input is borrowed immutably for the entire parse operation.

A type implementing `Pattern<I>` for some `I : Input` describes a predicate that checks whether an input at the specified location "matches" it. For example, `char` performs a character match against a `str` input, which may succeed if the location has the same `char` value or fail otherwise.

All `Pattern<I>` implementors may use `AllOf` and `AnyOf` to compose sequential and alternative matching.

## Input `str`

The textbook input type, natively supported.

### Usage
A producer of parser combinators expecting this input type should have the signature:
```rs
fn make_parser<'src, E: ParseEnvironment>(...) -> impl Parser<'src, str, A, E> {...}
```
where `A` is the desired AST payload type.

### Supported Patterns
`Pattern<str>` implementors include, where `K : Clone + Into<SyntaxKind>`:
- `(K, char)`
- `(K, &str)`
- `(K, String)`
- `(K, regex::Regex)` with default feature `regex`.
- `(K, regex_lite::Regex)` with default feature `regex`.

Note the explicit mentioning of `K` due to none of the `Pattern<str>` implementors having a distinct `SyntaxKind` associated value, which is required for automatic CST construction.

### Tradeoffs
`str`-based parsers offer a great deal of granularity, to both its advantage and detriment. Furthermore, `str`-based input preserves all characters unless explicitly consumed. Notably, this means that whitespace is preserved, which may be undesirable for whitespace-insensitive grammars.

### Variant `CachedStr<'source>`

Input type `CachedStr<'_>` is identical to `str`, but calculates the line start byte offsets eagerly and caches them for efficient diagnostic generation. Might be useful for wide speculative branching.

This type is mainly used internally for other input types. You should prefer `str` unless benchmarking shows that diagnostic generation contributes significantly to the performance of parsing.

A producer of parser combinators expecting this input type should have the signature:
```rs
fn make_parser<'parse, 'source, E: ParserEnvironment>(...) -> impl Parser<'parse, CachedStr<'source>, A, E>
where
    'source : 'parse,
{...}
```
where `A` is the desired AST payload type. 
Note the two distinct lifetimes: `'source` refers to the lifetime of the underlying `str` data that `CachedStr` borrows upon construction, and `'parse` represents the borrow on the input type `CachedStr<'source>` throughout a single parse.

## Input `LexerStream<'source, T>`

Integration with `logos` based lexing, based on `logos::Lexer`. Enabled by default feature `logos`.

### Usage
A producer of parser combinators expecting this input type should have the signature:
```rs
fn make_parser<'parse, 'source, E: ParserEnvironment>(...) -> impl Parser<'parse, LexerStream<'source, T>, A, E>
where
    'source : 'parse,
{...}
```
where `A` is the desired AST payload type. 
Note the two distinct lifetimes: `'source` refers to the lifetime of the underlying `str` data that `LexerStream` borrows upon construction, and `'parse` represents the borrow on the input type `LexerStream<'source, T>` throughout a single parse.

`T` implements the lexing grammar using the `logos` ecosystem, and must adhere to the following bounds:
```rs
impl<'source, T> Input for LexerStream<'source, T>
where
    T: Debug + Clone + 'static + Logos<'source, Source = str>,
{...}
```

### Supported Patterns
`Pattern<LexerStream<'source, T>>` implementors include:
```rs
(K, T) where
    K: Clone + Into<SyntaxKind>,
    T: Debug + Clone + Eq + Logos<'source, Source = str> + 'static,
```
```rs
T where
    T: Debug + Clone + Eq + Logos<'source, Source = str> + Into<SyntaxKind> + 'static,
```

### Tradeoffs

There are a few major limitations within `LexerStream` that does not cover the full feature set of `logos`:
- The source supported for lexing grammar `T` must be a UTF-8 string.
- The resulting type from lexing, `T`, must be `'static` and hence may *not* borrow from the underlying data. This notably excludes anything that takes a string reference `&str` into the underlying source.
- The natively supported `Pattern` types require `T : Eq`.

Furthermore, `logos` imposes the constraint that each token must be produced in a stateless manner. This means that lexing grammars must be globally unambiguous (eg. regex patterns `r"[a-zA-Z0-9_ ]+"` and `r"[ \t\r\n]+"` overlap, and hence must have a specified priority ordering or be rejected), whereas `str`-based parsing may disambugate between the two via control flow.

In exchange, `LexerStream` provides various correctness and ergonomic advantages:
- `Pattern`-matching constructs and lexing are vastly simplified due to `#[derive(Logos)]` syntax.
- Using `T` patterns means that a pattern-matching construct and its associated `SyntaxKind` can never be erroneously coupled.
- Trivia such as whitespace may be omitted or skipped, rather than explicitly consumed. Skipped trivia is preserved in the CST representation using a `SyntaxKind` value provided at construction, or the caller may assert that the lexing grammar is lossless via `LexerStream::assume_contiguous`.

This tends to lead to less noise within parser combinator definition. Take this example of a JSON parser for objects, describing the grammar rule:
> `object ::= '{' (pair members)? '}'`
```rs
// `str`-based implementation.
let object = (just((Kind::LeftBrace, '{')).then(optional_whitespace.clone()))
    .ignore_then(
        pair.clone()
            .then_ignore(optional_whitespace.clone())
            .then(members)
            .map(|(first, mut rest)| {
                rest.insert(0, first);
                rest
            })
            .optional_with_default()
    )
    .then_ignore(
        optional_whitespace
            .clone()
            .then(just((Kind::RightBrace, '}'))),
    )
    .map(Json::Object)
    .node(Kind::Object);
```
```rs
// `LexerStream`-based implementation.
let object = just(Kind::LeftBrace)
    .ignore_then(
        pair.clone()
            .then(members)
            .map(|(first, mut rest)| {
                rest.insert(0, first);
                rest
            })
            .optional_with_default()
    )
    .then_ignore(just(Kind::RightBrace))
    .map(Json::Object)
    .node(Kind::Object);
```

## Input `SkippingStr<'source>`

Trivia skipping for `str`-based input, typically whitespace. Trivia is skipped before and after successful pattern matching.

### Usage
A producer of parser combinators expecting this input type should have the signature:
```rs
fn make_parser<'parse, 'source, E: ParserEnvironment>(...) -> impl Parser<'parse, SkippingStr<'source>, A, E>
where
    'source : 'parse,
{...}
```
where `A` is the desired AST payload type. 
Note the two distinct lifetimes: `'source` refers to the lifetime of the underlying `str` data that `SkippingStr` borrows upon construction, and `'parse` represents the borrow on the input type `SkippingStr<'source>` throughout a single parse.

### Supported Patterns
All patterns supported by `str`.

### Tradeoffs
Naturally, this input type offers a fusion of the benefits of `str` and `LexerStream`:
- Familiar `str`-based pattern matching constructs eg. characters, string, regex patterns.
- Tokens need not be globally distinct, unlike `LexerStream`, and can be disambugated via control flow.
- Trivia is implicitly consumed and retained in the CST. This mitigates the biggest issue of raw `str` input.

However, due to the eager implicit consumption of trivia characters, this input type may have unexpected interactions with pattern matching containing trivia characters.

Essentially:
> Atomic patterns may consume trivia characters as part of their match, but cannot match leading trivia before the current cursor.

Examples, where whitespace is treated as trivia:
 - Pattern `"foo bar"` on input `"foo bar"` is an exact match.
 - Pattern `"foo "` on input `"foo bar"` matches with remainder `"bar"`.
 - Pattern `"foo"` on input `"foo bar"` matches with remainder `"bar"`. Trailing whitespace discarded after match.
 - Pattern `" foo"` on input `" foo"` fails to match because the leading whitespace on the input is discarded, leaving `"foo"` as the input.
   Patterns starting with leading trivia characters (`' '` in this case) *always* fails to match on any input value.
 - Pattern `Regex::new(r#""[^"]""#)` matches on an arbitrary string literal (including whitespace) enclosed by double quotes.
 - Non-atomic pattern `AllOf(["foo", " ", "bar"])` delegates to the inner patterns for matching,
   hence pattern `" "` will cause the match to fail on input `"foo bar"`.
   Note that `AllOf(["foo", " ", "bar"])` is **not** equivalent to `"foo bar"`,
   because the former is non-atomic and sequential, leading to trivia character consumption between each element matched.
 - Non-atomic pattern `AnyOf([...])` behaves as expected via delegation into the inner patterns;
   trivia consumption has no effect on speculative parsing.

Overall, this means that despite the relaxation on unambiguity requirements for lexing, `SkippingStr` may still be no more expressive than `LexerStream` compared to `str` regarding patterns containing trivia characters. This may be relevant for specific CST construction eg. separating the raw text of a string literal from the enclosing double quotes.

Prefer the other `Input` options when a more sophisticated trivia skipping mechanism or explicit matching on trivia characters is desired.