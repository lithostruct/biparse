# Input `LexerStream<'source, T>`

Integration with `logos` based lexing, based on `logos::Lexer`. Enabled by default feature `logos`.

## Usage
A producer of parser combinators expecting this input type should have the signature:
```rs,ignore
fn make_parser<'parse, 'source, E: ParserEnvironment>(...) -> impl Parser<'parse, LexerStream<'source, T>, A, E>
where
    'source : 'parse,
{...}
```
where `A` is the desired AST payload type. 
Note the two distinct lifetimes: `'source` refers to the lifetime of the underlying `str` data that `LexerStream` borrows upon construction, and `'parse` represents the borrow on the input type `LexerStream<'source, T>` throughout a single parse.

`T` implements the lexing grammar using the `logos` ecosystem, and must adhere to the following bounds:
```rs,ignore
impl<'source, T> Input for LexerStream<'source, T>
where
    T: Debug + Clone + Logos<'source, Source = str>,
{...}
```

## Supported Patterns
`Pattern<'src, LexerStream<'source, T>>` implementors include:
```rs,ignore
(K, T) where
    'source: 'src,
    K: Clone + Into<SyntaxKind>,
    T: Debug + Clone + Eq + Logos<'source, Source = str> + 'src,
```
```rs,ignore
T where
    'source: 'src,
    T: Debug + Clone + Eq + Logos<'source, Source = str> + Into<SyntaxKind> + 'src,
```

## Tradeoffs

There are a few major limitations within `LexerStream` that does not cover the full feature set of `logos`:
- The source supported for lexing grammar `T` *must* be a UTF-8 string.
- Due to diagnostic generation and the inability to dispatch dynamically on trait implementation, `T : Debug`.
- Due to internal implementation constraints, `T : Clone`.
- The natively supported `Pattern` types typically require `T : Eq`.

Furthermore, `logos` imposes the constraint that each token must be produced in a stateless manner. This means that lexing grammars must be globally unambiguous (eg. regex patterns `r"[a-zA-Z0-9_ ]+"` and `r"[ \t\r\n]+"` overlap, and hence must have a specified priority ordering or be rejected), whereas `str`-based parsing may disambugate between the two via control flow.

In exchange, `LexerStream` provides various correctness and ergonomic advantages:
- `Pattern`-matching constructs and lexing are vastly simplified due to `#[derive(Logos)]` syntax.
- Using `T` patterns means that a pattern-matching construct and its associated `SyntaxKind` can never be erroneously coupled.
- Trivia such as whitespace may be omitted or skipped, rather than explicitly consumed. Skipped trivia is preserved in the CST representation using a `SyntaxKind` value provided at construction, or the caller may assert that the lexing grammar is lossless via `LexerStream::assume_contiguous`.

This tends to lead to less noise within parser combinator definition.

## Example Usage

Take this example of a JSON parser for objects, describing the grammar rule:
> `object ::= '{' (pair members)? '}'`

Compare and constrast the `str`-based and `LexerStream`-based implementation.

```rs,ignore
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
            .optional()
            .map(Option::unwrap_or_default)
    )
    .then_ignore(
        optional_whitespace
            .clone()
            .then(just((Kind::RightBrace, '}'))),
    )
    .map(Json::Object)
    .node(Kind::Object);
```
```rs,ignore
// `LexerStream`-based implementation.
let object = just(Kind::LeftBrace)
    .ignore_then(
        pair.clone()
            .then(members)
            .map(|(first, mut rest)| {
                rest.insert(0, first);
                rest
            })
            .optional()
            .map(Option::unwrap_or_default)
    )
    .then_ignore(just(Kind::RightBrace))
    .map(Json::Object)
    .node(Kind::Object);
```