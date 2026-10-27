# Input `SkippingStr<'source>`

Trivia skipping for `str`-based input, typically whitespace. Trivia is skipped before and after successful pattern matching.

## Usage
A producer of parser combinators expecting this input type should have the signature:
```rs
fn make_parser<'parse, 'source, E: ParserEnvironment>(...) -> impl Parser<'parse, SkippingStr<'source>, A, E>
where
    'source : 'parse,
{...}
```
where `A` is the desired AST payload type. 
Note the two distinct lifetimes: `'source` refers to the lifetime of the underlying `str` data that `SkippingStr` borrows upon construction, and `'parse` represents the borrow on the input type `SkippingStr<'source>` throughout a single parse.

## Supported Patterns
All patterns supported by `str`.

## Tradeoffs
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

Overall, this means that despite the relaxation on unambiguity requirements for lexing, `SkippingStr` may still be less expressive compared to a non-skipping input. This may be relevant for specific CST construction eg. separating the raw text of a string literal from the enclosing double quotes.

Prefer the other `Input` options when a more sophisticated trivia skipping mechanism or explicit matching on trivia characters is desired.