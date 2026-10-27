//! Module for an internally-used [`GreenInterner`], which interns [`rowan::GreenNode`] and [`rowan::GreenToken`].
//! Spiritually similar to [`rowan::NodeCache`] but with exposed methods.

mod green_interner;
mod tests;

pub use green_interner::*;
