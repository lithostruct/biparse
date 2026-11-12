# AST and CST Transformation

This section describes transformation of the AST and CST components of a parse outcome.

## AST Transformation

AST payloads are freely transformable as Rust values. You may call methods, consume or discard them entirely. These operate on `Success` payloads only; failure values pass through unchanged.

### `map`

Uses a Rust function or closure to transform the AST payload, which is passed into it by value.

For a function `func : impl Fn(A) -> B`, a parser of `a.map(func)` produces an AST payload of type `B`.

### `into_owned`

A specialized implementation for mapping some reference (typically `&str` into the input) into an owning value via [`ToOwned::to_owned`].

A parser of `a_ref.into_owned()` produces an AST payload of type `<A as ToOwned>::Owned`.

Due to type requirements being stricter than most `Parser` implementors, using the `.into_owned` chaining method requires the `ParserIntoOwnedExt` trait to be in scope, rather than `ParserExt`. This trait is automatically blanket-implemented and included by the prelude.

### `try_map`

Uses a fallible Rust function or closure returning `Result` to transform the AST payload.

On success producing the `Ok` branch, the inner value is taken and used as the AST payload. On failure producing the `Err` branch, the inner value is transformed into a failure payload according to `ParseFailure::custom_failure_at`, which is then propagated as a `Committed` failure. The type `E` contained within the `Err` branch must implement [`std::error::Error`].

As an example, a parser of `a.try_map(<B as TryFrom>::try_from)` produces an AST payload of `B` on success.

## CST Transformation

The green CST forest is automatically accumulated during parsing. Without interference, it will end up parsing as a flat forest, but transforming it along the syntax rules of the grammar takes little effort and makes it substantially more useful for error reporting.

To maintain correspondence invariants, CST transformation is more limited and pre-defined compared to AST transformation, and does not involve type conversions.

### `node`, `named`

Appends a `SyntaxKind` as a new parent node, taking the existing forest as all of its children. `named` additionally provides the current construct with a name string, which may be useful for parsing error-reporting clarity.

Use these to mark syntactic constructs within the grammar.

### `associate`

Provides a unique `ParseEnvironment::Supply::Id` value to associate AST and CST constructs.

For the AST component, this identification value, along with the current AST value, is passed into a user-supplied closure. Users may choose to embed this value into the AST tree, or discard it. For the CST component, the identification value is associated with the path to a new parent node, which takes the existing forest as all of its children. This association is invariant under further CST transformation, hence AST-CST correspondence is maintained from the point at which the `Id` is supplied to the end of the parse.

Use this to associate AST and CST constructs for easy readback from the AST representation to the CST representation. This may be helpful for error reporting from downstream analysis, where the CST span is required for eg. typing failures within well-parsed expressions.

### `recover`

True only in a technical sense. Performs an `associate`-like operation onto the AST and CST upon recovery success, using

Refer to the [next section](./4_commitment_and_error_recovery.md) for elaboration on commitment and error recovery.