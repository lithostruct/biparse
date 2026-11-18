#![doc = include_str!("README.rustdoc.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod green;

pub mod cst;
pub mod failure;
pub mod input;

pub mod combinator;
pub mod parse;

pub mod prelude;

pub use rowan;

#[cfg(feature = "logos")]
pub use logos;
#[cfg(feature = "regex")]
pub use {regex, regex_lite};


/// Mirrors my usage of `mdbook-rustdoc-links`.
#[cfg(doctest)]
#[allow(unused)]
use crate::prelude::*;

/// Book examples tested here due to some weird incompatibilities with `mdbook test`.
// #[cfg(doctest)]
mod book_tests {
    mod examples {
        #[doc = include_str!("../guide/src/examples/1_s-expressions.md")]
        mod s_expressions {}
    }
}