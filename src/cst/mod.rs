//! Module for items relating to CST and origin tracking.

mod association;
mod io;
mod path;
mod supply;

pub type Cst<T> = im::Vector<T>;
pub use self::{association::*, io::*, path::*, supply::*};
