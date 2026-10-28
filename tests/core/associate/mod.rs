mod proptest;
mod cst;

use std::str::FromStr;

use biparse::{cst::IdSupply, green::GreenInterner, parse::TerminalFailure, prelude::*};
use rowan::{NodeOrToken, SyntaxKind};

/// A specialized S-expression variant, which embeds a SyntaxKind.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SExp {
    Symbol(String),
    List(SyntaxKind, Vec<Self>),
}
impl SExp {
    /// Converts `self` to a textual representation.
    /// 
    /// Guarantees that `SExp::from_str(self.to_text()) == Ok(self)`.
    pub fn to_text(&self) -> String {
        match self {
            SExp::Symbol(s) => s.clone(),
            SExp::List(kind, sexps) => {
                sexps.iter().fold(format!("({:}", kind.0), |mut s, child| {
                    if &s == "(" {
                        //  First element
                        s += &child.to_text();
                    } else {
                        s += " ";
                        s += &child.to_text();
                    }
                    s
                }) + ")"
            }
        }
    }
    pub fn to_cst(&self, interner: &mut GreenInterner, supply: &mut Option<usize>) -> AssociatedGreenCst<usize, usize, ()> {
        match self {
            SExp::Symbol(s) => {
                let token = interner.token(SyntaxKind(0), s);
                AssociatedGreenCst::from_tokens([NodeOrToken::Token(token)])
            },
            SExp::List(kind, sexps) => {
                let forest = sexps.iter().fold(AssociatedGreenCst::new(), 
                    |acc, child| {
                        acc.merge_with(child.to_cst(interner, supply))
                    }
                );
                if let Some(supply) = supply {
                    forest.associate_new_parent(*kind, supply.next_id(), interner)
                } else {
                    forest.attach_parent(*kind, interner)
                }
            },
        }
    }
}
impl FromStr for SExp {
    type Err = TerminalFailure<<DefaultParseEnvironment as ParseEnvironment>::Failure>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Lang {
    Token,
    /// Representation invariant: field must *not* be 0!
    Node(u16),
}
impl rowan::Language for Lang {
    type Kind = Self;
    fn kind_from_raw(raw: SyntaxKind) -> Self::Kind {
        if raw.0 == 0 {
            Lang::Token
        } else {
            Lang::Node(raw.0)
        }
    }
    fn kind_to_raw(kind: Self::Kind) -> SyntaxKind {
        match kind {
            Lang::Token => SyntaxKind(0),
            Lang::Node(u) => {
                assert_ne!(u, 0);
                SyntaxKind(u)
            },
        }
    }
}