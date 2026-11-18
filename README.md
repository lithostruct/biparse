# biparse

[![Crates.io](https://img.shields.io/crates/v/biparse.svg)](https://crates.io/crates/biparse)
[![Documentation](https://docs.rs/biparse/badge.svg)](https://docs.rs/biparse)
[![Guide](https://img.shields.io/badge/guide-biparse-181717)](https://lithostruct.github.io/biparse)
[![GitHub](https://img.shields.io/badge/GitHub-biparse-181717?logo=github)](https://github.com/lithostruct/biparse)
[![CI](https://github.com/lithostruct/biparse/actions/workflows/ci.yml/badge.svg)](https://github.com/lithostruct/biparse/actions/workflows/ci.yml)

Biparse is a parser-combinator framework designed around dual AST + CST construction and correspondence.  
Write an AST parser, get the CST for free!

## Quick Start

To add to your project:

```bash
cargo add biparse
```

Go through the [guide book](https://lithostruct.github.io/biparse) for further information, or refer to [`Parser`] and [`ParserExt`] for initial documentation.

You may use the [`prelude`] module to bring Biparse's commonly used items and traits into scope:
```rust
use biparse::prelude::*;
```

### Why Should I Use This?

Handwriting a source to CST to AST parser is fine at most *once*.

Most language experiments simply don't bother with a source-to-CST parser, because using a parser generator or handwriting a `rowan` parser can be unintuitive and slow to iterate upon. This is despite the fact that a CST contains vital information for ergonomic error reporting and DSL support.

This project makes two contributions:
- Despite being a parser combinator ecosystem comparable to [`chumsky`](https://crates.io/crates/chumsky) in its intuitive use, all parser combinators natively support CST construction and do so automatically. You essentially get a CST without even trying.
- `associate` combinators provide AST-CST correspondence through a configurable `Id` embedding, making it trivial to track which AST constructs correspond to the text and spans the CST node contains.

This crate is unlikely to be stellar at performance compared to other options. If you want something that's quick to pick up and hack on though? Biparse may be just for you.

### Acknowledgements

This project is heavily inspired and based on [`chumsky`](https://crates.io/crates/chumsky) and Kiran Shila's blog post "Easy Lossless Trees with Nom and Rowan" ([Wayback Machine](https://web.archive.org/web/20250318134533/https://blog.kiranshila.com/post/easy_cst)).

[`Parser`]: https://docs.rs/biparse/latest/biparse/parse/parser_trait/trait.Parser.html
[`ParserExt`]: https://docs.rs/biparse/latest/biparse/parse/parser_trait/trait.ParserExt.html
[`prelude`]: https://docs.rs/biparse/latest/biparse/prelude/index.html

License: MIT OR Apache-2.0
