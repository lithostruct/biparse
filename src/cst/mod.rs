//! Module for items relating to CST and origin tracking.

mod association;
mod io;
mod path;
mod supply;

pub use self::{io::*, path::*, supply::*};

/// A [`rowan::Language`] implementor that has `Kind = rowan::SyntaxKind`.
/// 
/// May be used for red-tree canonization if a more specific implementor is not defined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoLang;
impl rowan::Language for NoLang {
    type Kind = rowan::SyntaxKind;
    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        raw
    }
    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind
    }
}