# AST and CST Transformation

This section describes transformation of the AST and CST components of a parse outcome.

## AST Transformation

AST payloads are freely transformable as Rust values. You may call methods, consume or discard them entirely. These operate on `Success` payloads only; failure values pass through unchanged.

### `map`
*Docs.rs documentation: [`ParserExt::map`], [`combinator::Map`].*

Uses a Rust function or closure to transform the AST payload, which is passed into it by value.

For a function or closure `F : impl Fn(A) -> B`, a parser of `a.map(f)` produces an AST payload of type `B`.

### `ignore`
*Docs.rs documentation: [`ParserExt::ignore`].*

Syntax sugar for eg. `a.map(|_| ())`, discarding the AST payload and returning the unit value `()`.

### `into_owned`
*Docs.rs documentation: [`ParserIntoOwnedExt::into_owned`], [`combinator::IntoOwned`].*

A specialized implementation for converting some reference into an owned value via [`ToOwned::to_owned`].

A parser of `a_ref.into_owned()` produces an AST payload of type `<A as ToOwned>::Owned`, with the initial parser `a_ref` producing a `&'src A` typically borrowing into the input.

Due to type requirements being stricter than most [`Parser`] implementors, using the `.into_owned` chaining method requires the [`ParserIntoOwnedExt`] trait to be in scope, rather than [`ParserExt`]. This trait is automatically blanket-implemented and included by the [`prelude`].

### `try_map`
*Docs.rs documentation: [`ParserExt::try_map`], [`combinator::TryMap`].*

Uses a fallible Rust function or closure returning `Result` to transform the AST payload.

On success producing the `Ok` branch, the inner value is taken and used as the AST payload. On failure producing the `Err` branch, the inner value is transformed into a failure payload according to `ParseFailure::custom_failure_at`, which is then propagated as a [`ParseOutcome::Committed`](../parse_output.md#committed) failure.

The error type `E` contained within the `Err` branch must implement [`std::error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html) and be `'static`.

As an example, a parser of `a.try_map(<B as TryFrom>::try_from)` produces an AST payload of `B` on success.

## CST Transformation

The green CST forest is automatically accumulated during parsing. Without interference, it will end up parsing as a flat forest, but transforming it along the syntax rules of the grammar takes little effort and makes it substantially more useful for error reporting.

To maintain correspondence invariants, CST transformation is more limited and pre-defined compared to AST transformation, and does not involve type conversions.

### `node`, `named`
*Docs.rs documentation: [`ParserExt::node`], [`ParserExt::named`], [`combinator::Node`].*

Appends a `SyntaxKind` as a new parent node, taking the existing forest as all of its children. `named` additionally provides the current construct with a name string, which may be useful for parsing error-reporting clarity.

Use these to mark syntactic constructs within the CST grammar. These typically will mirror the AST types produced.

### `associate`
*Docs.rs documentation: [`ParserExt::associate`], [`combinator::Associate`].*

Provides a unique `ParseEnvironment::Supply::Id` value to associate AST and CST constructs.

For the AST component, this identification value, along with the current AST value, is passed into a user-supplied closure. Users may choose to embed this value into the AST tree, or discard it. For the CST component, the identification value is associated with the path to a new parent node, which takes the existing forest as all of its children. This association is invariant under further CST transformation, hence AST-CST correspondence is maintained from the point at which the `Id` is supplied to the end of the parse.

Use this to associate AST and CST constructs for easy readback from the AST representation to the CST representation. This may be helpful for error reporting from downstream analysis, where the CST span is required for eg. typing failures within well-parsed expressions.