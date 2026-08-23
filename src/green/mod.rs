//! Module for an internal [`rowan::GreenNode`] and [`rowan::GreenToken`] interner, with other convenience type definitions.
//! Spiritually similar to [`rowan::NodeCache`].

mod green_interner;
mod tests;

pub use green_interner::*;
