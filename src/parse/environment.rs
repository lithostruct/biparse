use crate::{
    cst::IdSupply,
    failure::{self, ParseFailure},
};

/// The trait describing additional information required by certain parsing constructs.
///
/// This trait should be implemented by zero-sized types.
pub trait ParseEnvironment: 'static {
    /// The type of the failure payload.
    type Failure: ParseFailure;
    /// The type of the supply provided to [`Associate`](crate::combinator::Associate) combinators.
    /// Used for associations.
    type Supply: IdSupply;
    /// The type of the supply provided to [`Recover`](crate::combinator::Recover) combinators.
    /// Used for diagnostics.
    type ErrorSupply: IdSupply;
}

/// The default implementation of [`ParseEnvironment`].
///
/// Users are encouraged to define their own zero-typed structs implementing [`ParseEnvironment`]
/// for different associated types.
#[derive(Debug, Clone, Copy)]
pub struct DefaultParseEnvironment;
impl ParseEnvironment for DefaultParseEnvironment {
    type Failure = failure::types::Rich;
    type Supply = usize;
    type ErrorSupply = usize;
}
