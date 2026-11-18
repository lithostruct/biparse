//! Module for [`ParseEnvironment`] implementors.
//!
//! Users are encouraged to define their own zero-sized structs implementing [`ParseEnvironment`]
//! for different associated types.

use crate::{
    failure::{self},
    parse::ParseEnvironment,
};

/// The minimal implementation of [`ParseEnvironment`], corresponding to [`failure::types::Minimal`].
///
/// Lowest overhead, but with the least meaningful failure messages.
#[derive(Debug, Clone, Copy)]
pub struct Minimal;
impl ParseEnvironment for Minimal {
    type Failure = failure::types::Minimal;
    type Supply = usize;
    type ErrorSupply = usize;
}

/// The simple implementation of [`ParseEnvironment`], corresponding to [`failure::types::Simple`].
///
/// Moderate overhead, with meaningful positional and simplistic kind information.
#[derive(Debug, Clone, Copy)]
pub struct Simple;
impl ParseEnvironment for Simple {
    type Failure = failure::types::Simple;
    type Supply = usize;
    type ErrorSupply = usize;
}

/// The rich implementation of [`ParseEnvironment`], corresponding to [`failure::types::Rich`].
///
/// High overhead, with most information retained from the site of failure.
#[derive(Debug, Clone, Copy)]
pub struct Rich;
impl ParseEnvironment for Rich {
    type Failure = failure::types::Rich;
    type Supply = usize;
    type ErrorSupply = usize;
}
