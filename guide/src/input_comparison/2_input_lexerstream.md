# Input `LexerStream<'source, T>`
*Docs.rs documentation: [input::logos::LexerStream].*

Integration with [`logos`] based lexing, based on [`logos::Lexer`]. Enabled by default feature `logos`.

## Usage
A producer of parser combinators expecting this input type should have the signature:
```rust,ignore
fn make_parser<'parse, 'source, E: ParserEnvironment>(...)
    -> impl Parser<'parse, LexerStream<'source, T>, A, E>
where
    'source : 'parse,
{
    /* ... */
    # success()
}
```
where `A` is the desired AST payload type. 
Note the two distinct lifetimes: `'source` refers to the lifetime of the underlying [`str`] data that [`LexerStream`] borrows upon construction, and `'parse` represents the borrow on the input type [`LexerStream<'source, T>`] throughout a single parse.

`T` implements the lexing grammar using the `logos` ecosystem via [`logos::Logos<'source, Source = str>`], and must additionally implement [`Clone`].

## Supported Patterns
[`Pattern<'src, LexerStream<'source, T>>`] implementors include:
```rust,ignore
(K, T) where
    'source: 'src,
    K: Clone + Into<rowan::SyntaxKind>,
    T: Debug + Clone + Eq + Logos<'source, Source = str> + 'src,
```
```rust,ignore
T where
    'source: 'src,
    T: Debug + Clone + Eq + Logos<'source, Source = str> + Into<rowan::SyntaxKind> + 'src,
```

## Tradeoffs

There are a few major limitations within [`LexerStream`] that does not cover the full feature set of [`logos`]:
- The source supported for lexing grammar `T` *must* be a UTF-8 string.
- Due to internal implementation constraints, `T : `[`Clone`].
- The natively supported [`Pattern`] types typically require `T : `[`Eq`].
- A function producing a string representation from a token reference is required for error reporting.
  - This defaults to the [`Display`](std::fmt::Display) implementation for [`LexerStream::new`], but may be weakened to [`Debug`] or a user-supplied function via constructor functions [`LexerStream::new_via_debug`] and [`LexerStream::new_with_descriptor`].

Furthermore, `logos` imposes the constraint that each token must be produced in a stateless manner. This means that lexing grammars must be globally unambiguous (eg. regex patterns `r"[a-zA-Z0-9_ ]+"` and `r"[ \t\r\n]+"` overlap eg `" "`, and hence must have a specified priority ordering or be rejected), whereas `str`-based parsing may disambugate between the two via control flow.

In exchange, [`LexerStream`] provides various correctness and ergonomic advantages:
- [`Pattern`]-matching constructs and lexing are vastly simplified due to `#[derive(Logos)]` syntax.
- Using `T` patterns means that a pattern-matching construct and its associated [`SyntaxKind`] can never be erroneously coupled.
- Trivia such as whitespace may be omitted or skipped, rather than explicitly consumed. Skipped trivia is preserved in the CST representation using a [`SyntaxKind`] value provided at construction, or the caller may assert that the lexing grammar is lossless via [`LexerStream::assume_contiguous`].

This tends to lead to less noise within parser combinator definition.