//! The module containing items related to CST construction and origin tracking
//! throughout the parsing process.

mod association;
mod output;
mod path;
mod supply;

pub type Cst<T> = im::Vector<T>;
pub use self::{association::*, output::*, path::*, supply::*};
