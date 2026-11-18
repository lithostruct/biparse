# But Why Though?

This section elaborates a bit on the motivation behind this project, and alternatives that end-users may find more useful depending on their usecase.

## Motivation

Handwriting a source to CST to AST parser is fine at most *once*.

Most language experiments simply don't bother with a source-to-CST parser, because using a parser generator or handwriting a `rowan` parser can be unintuitive and slow to iterate upon. This is despite the fact that a CST contains vital information for ergonomic error reporting and DSL support.

This project makes two contributions:
- Despite being a parser combinator ecosystem comparable to [`chumsky`](https://crates.io/crates/chumsky) in its intuitive use, all parser combinators natively support CST construction and do so automatically. You essentially get a CST without even trying.
- [`associate`](./combinators/3_transformation.md#associate) combinators provide AST-CST correspondence through a configurable `Id` embedding, making it trivial to track which AST constructs correspond to the text and spans the CST node contains.

This crate is unlikely to be stellar at performance compared to other options. If you want something that's quick to pick up and hack on though? Biparse may be just for you.

## Alternatives

There's quite a few! We'll start with the strongest contenders.

### [`treesitter`](https://tree-sitter.github.io/tree-sitter/) and [`rust_sitter`](https://crates.io/crates/rust-sitter)

The pair of projects that I would say exceeds the value proposition of Biparse and then some.

`treesitter` needs no introduction: it is the de-facto standard for fast and incremental parsing, natively producing a lossless CST. The `rust_sitter` wrapper makes writing the grammar and converting it to a typed AST effortless via proc-macro generation and type-directed extraction via [`rust_sitter::Extract`](https://docs.rs/rust-sitter/latest/rust_sitter/trait.Extract.html).

By exposing the [`tree_sitter::Language`](https://docs.rs/tree-sitter/latest/tree_sitter/struct.Language.html) generated, *all* of the capabilities of `treesitter` are retained. This means that incremental CST reuse (ASTs are rematerialized on each parse), fast parsing and quick iteration within Rust are all a given. These are all *massive* advantages if you're keen on leveraging them.

Minor points of contention are that:
- `treesitter` (and hence `rust_sitter`) use [GLR parsing](https://en.wikipedia.org/wiki/GLR_parser), which can produce unintuitive grammar errors in shift-reduce and reduce-reduce errors.
- Error recovery during parsing is automated and hence less controlled by the parser author. This can be seen as both a benefit and drawback.

In contrast, Biparse:
- Is based on [parsing expression grammars](https://en.wikipedia.org/wiki/Parsing_expression_grammar), which follows ordered choice and execution-based control flow.
- [`recover`](./combinators/4_commitment_and_error_recovery.md#recovery-parsing-recover) combinators allow for arbitrary recovery strategies, offering significant granularity to parser authors.

Some may find Biparse's style of parser writing more familiar coming from handwritten recursive descent.

### Rust Parser Generators

Most of the major parser generators in the Rust ecosystem focus on either AST or CST generation, but not both:
- [`lalrpop`](https://crates.io/crates/lalrpop): AST only; familiar syntax and usage with Rust code.
- [`lrpar`](https://crates.io/crates/lrpar): AST only; [Yacc](https://en.wikipedia.org/wiki/Yacc) implementation with automated error recovery and repair sequences.
- [`racc`](https://crates.io/crates/racc): AST only; [Yacc](https://en.wikipedia.org/wiki/Yacc) implementation with focus on proc-macro generation.
- [`pest`](https://crates.io/crates/pest): CST only; parser expression grammar.

Naturally, their reduced scope allows for efficient implementation of their core parsing capabilities. It is unlikely that Biparse will beat any of these in terms of performance.

It is possible (though extremely tedious) to construct both AST and CST components simultaneously with some of these options, though the author has to maintain the lossless invariants of a CST manually.

### Rust Parser Combinators

I'll be remiss to not mention that this project is heavily inspired by [`chumsky`](https://crates.io/crates/chumsky) and Kiran Shila's blog post "Easy Lossless Trees with Nom and Rowan" ([Wayback Machine](https://web.archive.org/web/20250318134533/https://blog.kiranshila.com/post/easy_cst)).

To my knowledge, most parser combinator crates in Rust focus exclusively on AST generation, with few attempts to support more sophisticated capabilities such as incremental reuse. Similarly to the previous section, while you *can* build AST and CST structures, it doesn't mean that you *should*, and that it's easy to mess up by accidentally discarding CST fragments.

Options include but are not limited to:
- [`chumsky`](https://crates.io/crates/chumsky): primary inspiration for this crate and its structure. Notable approach to error recovery, which this project builds upon.
- [`nom`](https://crates.io/crates/nom): natively supports binary formats, parser combinators are functions.
- [`winnow`](https://crates.io/crates/winnow): similar to `nom`.

### Handwritten Recursive Descent

While this is the yardstick that Biparse is explicitly built against, handwritten parsers oftentimes can offer authors significant control over error recovery and error messages over any form of pre-built parser architecture. The tedium mostly lies in the double pass one often needs to write if you go from source to CST to AST.

It is perfectly possible (and at times preferable) to handwrite a robust parser. I recommend Thunderseethe's blogpost ["Reproachfully Presenting Resilient Recursive Descent Parsing"](https://thunderseethe.dev/posts/parser-base/).
