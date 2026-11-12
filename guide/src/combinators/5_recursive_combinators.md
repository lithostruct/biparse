# Recursive Combinators

Biparse offers recursive parsing capabilities in the form of self-recursion and mutual recursion.

## Self Recursion

A grammar construct that only refers to itself, requiring a recursive back-edge.

Self-recursive parsers may be constructed via the `Recursive::new` and `Recursive::new_as` methods. The primary argument is a closure that takes in a `self` reference and constructs the desired parser.

## Mutual Recursion

A set of grammar constructs that may refer to each other.

`Recursive::declare` and `Recursive::declare_as` creates a new mutually-recursive parser with an undefined body. This body is filled upon calling `Recursive::define`. Each parser must be defined exactly *once*; attempting to define a parser multiple times result in an immediate panic, and attempting a parse with an undefined parser returns a [`Fatal` error](../parse_output.md/#fatal).

A major limitation of the current implementation is that strong reference cycles formed by mutually-recursive parsers causes them to *never* deallocate when inaccessible, persisting until the end of program execution. Strategic use of `Recursive::weaken` may be required to break recursive back edges of strong cycles where applicable.

This may be addressed more completely in the future with dedicated recursive group semantics.

## Notes

### Left-Recursive Grammars

Biparse employs a seed-growing algorithm as described by the paper ["Packrat Parsers can Support Left-Recursion"](https://web.cs.ucla.edu/~todd/research/pepm08.pdf). This means that left-recursive grammars are supported for both self-recursion and mutual recursion, and are oftentimes preferable to equivalent right-recursive grammars due to bounded stack space consumption.

Note that unlike a true packrat parser, Biparse makes no attempt to memoize intermediate results formed from non-recursive constructs. Memoization only occurs for intermediate seeds formed via left recursion.

### Right-Recursive Grammars

Right-recursive grammars are natively supported via traditional recursive descent.

Note that right-recursive parsing inherently requires the stack frame for each iteration to remain on the stack until the end of the recursive construct. This may lead to stack overflow for deeply-nested parsing.

Prefer left-recursive grammars where applicable.

### Native Repetition

Prefer using the [native repetition composites](./2_composite_combinators.md/#repetition) `many`, `some` and `optional` over recursive grammars where applicable. This is particularly applicable for right-recursive grammars, which may consume unbounded stack space proportional to the recursive depth.