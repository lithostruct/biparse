
use std::str::FromStr;

use biparse::{cst::IdSupply, green::GreenInterner, input::str::SkippingStr, parse::TerminalFailure, prelude::*};
use expect_test::expect;
use indoc::indoc;
use proptest::{arbitrary::any, proptest, strategy::Strategy};
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
                sexps.iter().fold(format!("(:{:}", kind.0), |mut s, child| {
                    s += " ";
                    s += &child.to_text();
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

    pub fn parse(s : &str) -> Result<Self, TerminalFailure<<DefaultParseEnvironment as ParseEnvironment>::Failure>> {
        SExp::parse_from::<DefaultParseEnvironment>(s)
    }

    pub fn parse_from<E: ParseEnvironment>(s : &str) -> Result<Self, TerminalFailure<E::Failure>> {
        let skipping = SkippingStr::whitespace(s, SyntaxKind(100));

        // It doesn't matter what CST data we assign here because it can never be accurate.
        use std::io::{Error, ErrorKind};
        let syntax_kind: Box<dyn Parser<'_, SkippingStr<'_>, SyntaxKind, E>>
        = just((
                SyntaxKind(2),
                regex::Regex::new(r":[0-9]+").unwrap(),
            ))
            .try_map(|s: &str| if s == ":65535" {
                    Err(Error::new(
                        ErrorKind::InvalidData, 
                        "invalid value of node SyntaxKind 'u16::MAX'",
                    ))
                } else { Ok(&s[1..]) }
            )
            .try_map(u16::from_str)
            .map(SyntaxKind)
            .named("kind", SyntaxKind(3))
            .boxed();

        let parser: Recursive<dyn Parser<'_, SkippingStr<'_>, SExp, E>>
        = Recursive::new_as("sexp", |this| {
            // base case: identifier.
            just::<SkippingStr<'_>, E, (SyntaxKind, regex::Regex)>((
                SyntaxKind(0),
                regex::Regex::new(r"[a-zA-Z][a-zA-Z0-9_-]*").unwrap(),
            ))
            .into_owned()
            .map(SExp::Symbol)
            .or_same(
                // recursive case:
                // list ::= '(' u16 (this)* ')'
                just((SyntaxKind(1), '('))
                .ignore_then(syntax_kind)
                .then(this.many())
                .then_ignore(just((SyntaxKind(4), ')')))
                .map(|(kind, children)| SExp::List(kind, children))
                .named("list", SyntaxKind(5))
            )
        });
        parser.parse_all(&skipping)
            .map(|artifact| artifact.into_ast_only())
    }
}
impl FromStr for SExp {
    type Err = TerminalFailure<<DefaultParseEnvironment as ParseEnvironment>::Failure>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        SExp::parse_from::<DefaultParseEnvironment>(s)
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
        if raw.0 == u16::MAX {
            Lang::Token
        } else {
            Lang::Node(raw.0)
        }
    }
    fn kind_to_raw(kind: Self::Kind) -> SyntaxKind {
        match kind {
            Lang::Token => SyntaxKind(u16::MAX),
            Lang::Node(u) => {
                assert_ne!(u, u16::MAX);
                SyntaxKind(u)
            },
        }
    }
}

pub fn arbitrary_sexp() -> impl Strategy<Value = SExp> {
    let symbol = r"[a-zA-Z][a-zA-Z0-9_-]{0,15}".prop_map(SExp::Symbol);
    let kind = any::<u16>().prop_map(|inner| SyntaxKind(inner.saturating_sub(1)));
    symbol.prop_recursive(8, 256, 10,
        move |inner| {
            (kind.clone(), proptest::collection::vec(inner, 0..10))
                .prop_map(|(kind, vec)| SExp::List(kind, vec))
        }
    )
}

/// Ah yes, the test to assert that the parser used for the test does indeed work for use in the test.
#[test]
fn sexp_metatest() {
    let sexp = SExp::from_str("(:1 a b c)").unwrap();
    expect![[r#"
        List(
            SyntaxKind(
                1,
            ),
            [
                Symbol(
                    "a",
                ),
                Symbol(
                    "b",
                ),
                Symbol(
                    "c",
                ),
            ],
        )
    "#]].assert_debug_eq(&sexp);

    let err = SExp::from_str("(:65535)").unwrap_err();
    expect!["(1, 8) invalid value of node SyntaxKind 'u16::MAX'"].assert_eq(&err.terminating.to_string());
}

#[test]
fn sexp_roundtrip() {
    let sexp = SExp::from_str(indoc! {"
        (:100
            (:10)
            (:20 lorem ipsum)
            (:30 dolor sit amet)
            (:40
                (:1 nested)
                varying
                (:3 interspersed values)
                with-no-rhyme-or-reason
            )
        )
    "}).unwrap();
    assert_eq!(sexp, SExp::from_str(&sexp.to_text()).unwrap())
}
proptest! {
    #[test]
    fn sexp_arb_roundtrip(sexp in arbitrary_sexp()) {
        assert_eq!(sexp, SExp::from_str(&sexp.to_text()).unwrap())
    }
}