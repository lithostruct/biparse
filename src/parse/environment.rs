use std::{fmt::Debug, hash::Hash};

use crate::failure::{self, ParseFailure};

pub trait ParseEnvironment {
    type Failure: ParseFailure + std::error::Error;
    type Id: Debug + Clone + Eq + Hash;
}
pub struct DefaultParseEnvironment;
impl ParseEnvironment for DefaultParseEnvironment {
    type Failure = failure::Simple;
    type Id = u64;
}
