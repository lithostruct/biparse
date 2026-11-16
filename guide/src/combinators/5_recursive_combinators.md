# Recursive Combinators

Biparse offers recursive parsing capabilities in the form of self-recursion and mutual recursion.

## Self Recursion

A grammar construct that only refers to itself recursively.

Self-recursive parsers may be constructed via the `Recursive::new` and `Recursive::new_as` methods. The primary argument is a fixpoint closure that takes in a `self` reference and constructs the desired parser.

### Example

The recursive grammar rule:
```text
sexp ::= identifier | '(' sexp* ')'
```
is expressed to a first approximation (ignoring typing of AST payload) as:
```rust,ignore
let sexp : Recursive<Direct<'_, '_, I, _, E>> =
    Recursive::new(|this|                      // the back-edge referring to `sexp`.
        identifier.or(
            just((/* kind */, '('))
            .then(this.many())             // back-edge used!
            .then(just((/* kind */, ')')))
        )
    );
```

## Mutual Recursion

A set of grammar constructs that may refer to each other.

`Recursive::declare` and `Recursive::declare_as` creates a new mutually-recursive parser with an undefined body. This body is filled upon calling `Recursive::define`. Each parser must be defined exactly *once*; attempting to define a parser multiple times result in an immediate panic, and attempting a parse with an undefined parser results in a [`Fatal` failure](../parse_output.md#fatal).

### Limitations

[Strong reference cycles](https://doc.rust-lang.org/std/rc/index.html) formed by mutually-recursive parsers causes them to *never* deallocate when inaccessible, persisting until the end of program execution.

Strategic use of `Recursive::weaken` may be required to break strong cycles where applicable, by constructing a non-owning handle to a recursive combinator. This is typically applied to the recursive back-edges to a "primary" combinator, which keeps its group of related constructs alive. Attempting a parse on a dropped parser results in a [`Fatal` failure](../parse_output.md#fatal).

This may be addressed more completely in the future with dedicated recursive group semantics.

### Example

Consider the grammar of a (very simplistic) expression-based programming language:
```text
expression ::= identifier | number | ... | (statement ';')* expression
statement ::= "let" identifier '=' expression ';'
```

`expression` and `statement` are mutually-recursive. We (somewhat arbitrarily) identify `expression` as the primary parser due to the expression-heavy nature of the syntax. This means that any recursive back edges to `expression` should be `weaken`-ed, and that `expression` must be kept alive for `statement` to be alive too.

We declare these two parsers, which produces undefined owned handles:
```rust,ignore
let mut expression : Recursive<Indirect<'_, '_, I, Expr, E>> =
    Recursive::declare();

let mut statement : Recursive<Indirect<'_, '_, I, Stmt, E>> =
    Recursive::declare();
```
And with them now in scope, we can define them. 
```rust,ignore
// First approximation (typing of AST payload not considered)

expression.define(
    identifier
    .or(number)
    /* elided... */
    .or(
        statement.clone()               // Owning handle cloned.
        .then(just((/* kind */, ';')))
        .many()
        .then(expression.weaken())      // Non-owning handle.
    )
);

statement.define(
    /* elided... */
    .then(expression.weaken())          // Non-owning handle.
    .then(/* ... */)
);
```

Note that defining a parser via `Recursive::define` takes a parser directly, unlike the fixpoint closure of [self-recursion](#self-recursion), which handles weakening for us automatically.

## Notes

### Left-Recursive Grammars

Biparse employs a seed-growing algorithm as described by the paper ["Packrat Parsers can Support Left-Recursion"](https://web.cs.ucla.edu/~todd/research/pepm08.pdf). This means that left-recursive grammars are supported for both self-recursion and mutual recursion, and are oftentimes preferable to equivalent right-recursive grammars due to bounded stack space consumption.

Note that unlike a true packrat parser, Biparse makes no attempt to memoize intermediate results formed from non-recursive constructs. Memoization only occurs for intermediate seeds formed via left recursion.

### Right-Recursive Grammars

Right-recursive grammars are natively supported via traditional recursive descent.

Note that right-recursive parsing inherently requires the stack frame for each iteration to remain on the stack until the end of the recursive construct. This may lead to stack overflow for deeply-nested parsing.

Prefer left-recursive grammars where applicable.

### Unbounded Recursion

Unbounded recursion from self or mutual recursion will result in a [`Fatal` failure](../parse_output.md#fatal).

Examples of unbounded recursive grammars include but are not limited to:

```text
self ::= self
```
```text
a ::= b
b ::= a
```

### Native Repetition

Prefer using the [native repetition composites](./2_composite_combinators/23_repetition.md) `many`, `some` and `optional` over recursive grammars where applicable. This is particularly true for right-recursive grammars, which may consume unbounded stack space proportional to the recursive depth.