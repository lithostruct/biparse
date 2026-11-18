# Biparse Guide Book

This folder contains the guide book for Biparse.

This book is hosted at [https://lithostruct.github.io/biparse].

## Note on Rust `>= 1.100.0` and `mdbook-rustdoc-links`

Attempting to compile this book with the 1.100.0 toolchain results in an error from the `mdbook-rustdoc-links` preprocessor invoking `rustdoc` with a potentially stale or incorrect build cache. Why exactly this is the case is unclear to me, but it may have something to do with the [Cargo build directory rework](https://goals.rust-lang.org/2025h2/cargo-build-dir-layout.html), which was [implemented in part in Rust 1.100.0](https://blog.rust-lang.org/2026/11/12/Rust-1.100.0/#reducing-cargo-cache-collisions).

[`book.toml`](./book.toml) fixes the toolchain version to Rust 1.99.0, somewhat arbitrarily. You may choose to use pre-1.100.0 versions.

## Quick Start

To compile this book yourself, the following dependencies are used:
- `mdbook`.
- `mdbook-rustdoc-links`, which generates `rustdoc`-style links. Omitting this may cause certain links to break.

You may install these via [`Cargo`](https://doc.rust-lang.org/cargo/getting-started/installation.html):
```bash
cargo install mdbook
cargo install mdbook-rustdoc-links
```
It is recommended that you use [`cargo-binstall`](https://crates.io/crates/cargo-binstall) to speed up installation times.

To build, serve and open the book:
```bash
mdbook serve --open
```