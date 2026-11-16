# Biparse Guide Book

This folder contains the guide book for Biparse.

## Quick Start

This book is hosted at [TODO].

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