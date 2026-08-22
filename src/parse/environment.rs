use crate::{
    cst::IdSupply,
    failure::{self, ParseFailure},
};

/// The trait describing additional information required by certain parsing constructs.
pub trait ParseEnvironment {
    type Failure: ParseFailure + std::error::Error;
    type Supply: IdSupply;
}
pub struct DefaultParseEnvironment;
impl ParseEnvironment for DefaultParseEnvironment {
    type Failure = failure::Simple;
    type Supply = usize;
}
