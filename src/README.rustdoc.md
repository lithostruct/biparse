Biparse is a parser-combinator framework designed around dual AST + CST construction and correspondence.  
Write an AST parser, get the CST for free!

# Motivation

Writing a source to CST to AST parser by hand is fine at most once.

Most language experiments simply don't bother with a source-to-CST parser, because using a parser generator or handwriting a `rowan` parser can be unintuitive and slow to iterate upon. This is despite the fact that a CST contains vital information for ergonomic error reporting and DSL support.

This project makes two contributions:
- Despite being a parser combinator ecosystem comparable to `chumsky` in its intuitive use, all parser combinators natively support CST construction and do so automatically. You essentially get a CST without even trying.
- `associate` combinators provide AST-CST correspondence through a configurable `Id` embedding, making it trivial to track which AST constructs correspond to the text and spans the CST node contains.

If you want something that's quick to pick up and hack on, this crate may be just for you.

# Quick Start

```bash
cargo add biparse
```