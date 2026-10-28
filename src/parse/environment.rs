use crate::{
    cst::IdSupply,
    failure::{self, ParseFailure},
};

/// The trait describing additional information required by certain parsing constructs.
///
/// This trait should be implemented by zero-sized types.
pub trait ParseEnvironment: 'static {
    type Failure: ParseFailure + std::error::Error;
    type Supply: IdSupply;
    type ErrorSupply: IdSupply;
}
#[derive(Debug, Clone, Copy)]
pub struct DefaultParseEnvironment;
impl ParseEnvironment for DefaultParseEnvironment {
    type Failure = failure::Rich;
    type Supply = usize;
    type ErrorSupply = usize;
}
