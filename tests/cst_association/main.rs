//! Testing that CST construction via left-right concatenation and parent wrapping
//! maintains node identity through path transformation.
//!
//! This allows for Biparse to provide stronger provenance tracking capabilities than [`rowan::SyntaxNodePtr`],
//! and to provide the ecosystem invariant that CST construction must be total.

mod sexp_def;
pub use sexp_def::{Lang, SExp, sexp_strategy};

mod cst;
mod monoid_laws;
