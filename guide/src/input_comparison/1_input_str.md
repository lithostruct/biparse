# Input `str`

The textbook input type, natively supported.

## Usage
A producer of parser combinators expecting this input type should have the signature:
```rust,ignore
fn make_parser<'src, E: ParseEnvironment>(...) -> impl Parser<'src, str, A, E> {...}
```
where `A` is the desired AST payload type.

## Supported Patterns
[`Pattern<str>`] implementors include, where `K : Clone + Into<rowan::SyntaxKind>`:
- [`(K, char)`](char)
- [`(K, &str)`](str)
- [`(K, String)`](String)
- [`(K, regex::Regex)`](regex::Regex) with default feature `regex`.
- [`(K, regex_lite::Regex)`](regex_lite::Regex) with default feature `regex`.

Note the explicit mentioning of `K` due to none of the [`Pattern<str>`] implementors having a distinct [`rowan::SyntaxKind`] associated value, which is required for automatic CST construction.

## Tradeoffs
[`str`]-based parsers offer a great deal of granularity, to both its advantage and detriment. Furthermore, [`str`]-based input preserves all characters unless explicitly consumed. Notably, this means that whitespace is preserved, which may be undesirable for whitespace-insensitive grammars.

## Variant: `CachedStr<'source>`
*Docs.rs documentation: [input::cached_str::CachedStr].*

Input type [`CachedStr<'_>`](input::cached_str::CachedStr) is identical to [`str`], but calculates the line start byte offsets eagerly and caches them for efficient diagnostic generation. Might be useful for wide speculative branching.

This type is mainly used internally for other input types. You should prefer [`str`] unless benchmarking shows that diagnostic generation contributes significantly to the performance of parsing.

This type is *not* exported by the [`prelude`].

A producer of parser combinators expecting this input type should have the signature:
```rust,ignore
fn make_parser<'parse, 'source, E: ParserEnvironment>(...) -> impl Parser<'parse, CachedStr<'source>, A, E>
where
    'source : 'parse,
{...}
```
where `A` is the desired AST payload type. 
Note the two distinct lifetimes: `'source` refers to the lifetime of the underlying [`str`] data that [`CachedStr`](input::cached_str::CachedStr) borrows upon construction, and `'parse` represents the borrow on the input type `CachedStr<'source>` throughout a single parse.