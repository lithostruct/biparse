//! Module for items relating to CST and origin tracking.

mod association;
mod output;
mod path;
mod supply;

pub type Cst<T> = im::Vector<T>;
pub use self::{association::*, output::*, path::*, supply::*};
