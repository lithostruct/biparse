//! Implementors of [`ParseFailure`](crate::failure::ParseFailure) and related types.

mod minimal;
mod rich;
mod simple;

pub use {
    minimal::Minimal,
    rich::{Rich, RichKind},
    simple::{Simple, SimpleKind},
};
