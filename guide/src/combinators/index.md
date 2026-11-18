# Combinators

This section is an exhaustive listing of all combinators Biparse provides, categorized based on their conceptual role.

It is not necessary to know all of these to start using Biparse! Treat this section more as an encyclopedia than anything.

You can check out some examples if you want a more holistic look at the ecosystem.

Alternatively, most of the information in this section is available in the Docs.rs documentation on [`ParserExt`], which provides the method-chaining construction typical of parser combinator libraries.

## Overview

Biparse's parser combinator ecosystem is primarily inspired by that of [`chumsky`](https://docs.rs/chumsky/latest/chumsky/index.html), which frontloads cognitive complexity onto the type system through trait implementations and dispatch. The actual runtime values of the parser combinators themselves tend to be lightweight.

Therefore, the intended usage pattern is that of a parser-producing function:
```rs,ignore
fn make_parser() -> impl Parser<...> { ... }
```

Refer to the [`Input Types`](../input_comparison/index.md) chapter for further details and the exact form of the function signatures for each input type.

## Note on Method Chaining and Nesting

Most non-leaf combinators are constructed via [`ParserExt`] method chaining, and is the preferred method of construction over direct usage of the corresponding combinator types. This means that following Rust's evaluation rules, parser method chaining is left-associative.

This is most obviously seen in chaining of [`then`](ParserExt::then) sequencing parsers and [`either_or`](ParserExt::or) alternative parsers, which produce left-nested data structures:
- `a.then(b).then(c)` is equivalent to `(a.then(b)).then(c)`, producing a payload of type `((A, B), C)`.
- `a.then(b.then(c))` produces a payload of type `(A, (B, C))`. Isomorphic but not identical to the other two forms.

## Notes on Type Inference

Biparse leverages Rust's type inference for ergonomic usage. Hence, it is always helpful to fix the types of [`I: Input + ?Sized`](Input) and [`E: ParseEnvironment`](ParseEnvironment), as these types cannot be inferred from the contents of the parser combinator itself.

You can do this by either specifying the type of the parser as the return type of a function:

```rust, ignore
// `I: Input` and AST payload type `A` known.
fn make_parser<'src, E>() -> impl Parser<'src, I, A, E> { /*...*/ }
```

As part of a let-binding, typically for a type-erased parser:

```rust, ignore
// `E: ParseEnvironment` in scope, `I: Input` known.
let expression : Rc<dyn Parser<'_, I, Expr, E>> = /*...*/.shared();
```

Or by specifying it for the first leaf in the combinator object, according to its constructor signature:
```rust, ignore
// Here, `I: Input = str`, `E: ParseEnvironment = DefaultParseEnvironment`.
let let_identifier = just<'_, str, DefaultParseEnvironment, _>((SyntaxKind(0), "let"))
    .then(/* ... */);
```

# Note on `#[non_exhaustive]`

Some parser combinators enforce type invariants upon construction. For instance, `a.then(b)` (corresponding to [`Then::new(a, b)`](biparse::combinator::Then::new)) asserts that both `a` and `b` are parser objects satisfying [`Parser<...>`]. However, once constructed, the inner objects may be borrowed, mutated or taken without issue.

To avoid extraneous accessor and destructor functions, these structures are marked `#[non_exhaustive]` to allow for borrowing (immutable and/or mutable) and destruction using typical Rust syntax, while preserving the construction invariants that these parser objects observe.