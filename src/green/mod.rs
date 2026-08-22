//! The module containing the internal implementation of a `GreenNode` and `GreenToken` interner.
//! Spiritually similar to [`rowan::NodeCache`].

mod green_interner;
mod tests;

pub use green_interner::*;
