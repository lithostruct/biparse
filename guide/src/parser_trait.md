# `Parser` Trait Description and Usage

## Main `Parser<'src, I, A, E>` Trait

The `Parser` trait takes the form:
```rs
/// The central parsing trait, implemented by all combinators.
pub trait Parser<'src, I: Input + ?Sized, A, E: ParseEnvironment = DefaultParseEnvironment> {
    /// Parse the input from the given state, returning a result state and output.
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<A, E>);

    /// Checks the input for any errors from the given state.
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<(), E>) { ... }

    /// Returns a parse hint conceptually relating the item that is expected by this parser.
    fn hint(&self) -> ParseHint { ... }
}
```
- `'src` is the lifetime over which the input `I` is borrowed for during a parse operation.
- `I` is the input type that this parser accepts.
- `A` is the AST output type this parser produces.
- `E` is a (typically zero-sized) type implementing `ParseEnvironment` which carries additional information regarding CST association and type for parsing errors.
- `parse` is the main operation used by the combinator ecosystem, which implements how this combinator takes in an input and parse state, to return either a successful parse containing payload `A` or an error.
- `check` is similar to `parse` but discards all information other than whether the parse is successful.
- `hint` provides diagnostic information regarding the input this parser expects.

## Peripheral `ParserExt<'src, I, A, E>` Trait

To maintain `dyn`-compatibility, the main `Parser` trait does not include method chaining options. Instead, a `ParserExt` trait is used for this purpose, and is blanket-implemented for all implementors of `Parser`:

```rs
/// Convenience methods available to all implementors of [`Parser<'src, I, A, E>`]. Enables combinator method chaining.
pub trait ParserExt<'src, I: Input + ?Sized, A, E: ParseEnvironment = DefaultParseEnvironment>:
    Parser<'src, I, A, E>
{
    /// Parses from the start of input to the end, converting the output to a top-level result.
    fn parse_all(&self, input: &'src I) -> ParseResult<A, E> { ... }

    /// Checks from the start of input to the end, converting the output to a top-level result.
    fn check_all(&self, input: &'src I) -> ParseResult<(), E> { ... }

    // Additional chaining methods which delegate to combinator constructors...
}

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, T> ParserExt<'src, I, A, E> for T where
    T: Parser<'src, I, A, E>
{ }
```
`parse_all` and `check_all` are the top-level methods which are expected to be used by end users of the parser combinator ecosystem.

Method chaining methods include but are not limited to:
- Sequential and alternative composition: `.then(...)`, `.or(...)`.
- Repetition or optional parsing: `.many()`, `.some()`, `.optional()`.
- Mapping and CST node construction and association: `.map()`, `.node()`, `associate(...)`.
- `dyn` object construction: `.boxed()`, `.shared()` which constructs `Rc<dyn ...>`.
- Derived combinations of parsers: `.ignore()`, `or_same()`, `optional_with_default()`, etc.

In other words, most non-leaf combinators are constructed via `ParserExt` method chaining, and is the preferred method of construction over direct usage of the corresponding combinator types. The combinator types themselves and their respective constructors methods remain available in the `biparse::combinator` module.