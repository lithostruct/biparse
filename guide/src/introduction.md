# Introduction

Biparse is a parser-combinator framework designed around dual AST + CST construction and correspondence.  
Write an AST parser, get the CST for free!

# Quick Start

To add to your project:

```bash
cargo add biparse
```

Go through the [guide book](https://lithostruct.github.io/biparse) for further information, or refer to [`Parser`] and [`ParserExt`] for initial documentation.

You may use the [`prelude`] module to bring Biparse's commonly used items and traits into scope:
```rust
use biparse::prelude::*;
```

## Note

This guide offers a larger-scale overview and understanding of the parser ecosystem, its traits, and notes on usage. For specific documentation, you may find the [Docs.rs](https://docs.rs/biparse/latest/biparse/) page more applicable.

## Acknowledgements

This project is heavily inspired and based on [`chumsky`](https://crates.io/crates/chumsky) and Kiran Shila's blog post "Easy Lossless Trees with Nom and Rowan" ([Wayback Machine](https://web.archive.org/web/20250318134533/https://blog.kiranshila.com/post/easy_cst)).