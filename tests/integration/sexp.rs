#![cfg(feature = "regex")]

use biparse::{input::str::SkippingStr, prelude::*};
use derive_more::From;
use expect_test::expect;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use rowan::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, From)]
struct Id(#[from] usize);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SExp {
    Symbol(Id, String),
    List(Id, Vec<Self>),
}

struct IdParseEnvironment;
impl ParseEnvironment for IdParseEnvironment {
    type Failure = biparse::failure::Rich;
    type Supply = biparse::cst::NewtypeSupply<Id>;
    type ErrorSupply = usize;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, FromPrimitive, ToPrimitive)]
#[repr(u16)]
enum Kind {
    // Leaf tokens
    String = 0,
    LeftParen,
    RightParen,

    // Trivia
    Whitespace,

    // CST nodes
    Symbol,
    List,
    Root,
}
impl rowan::Language for Kind {
    type Kind = Self;
    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        Self::from_u16(raw.0).unwrap()
    }
    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind.to_u16().unwrap())
    }
}
impl From<Kind> for rowan::SyntaxKind {
    fn from(value: Kind) -> Self {
        Kind::kind_to_raw(value)
    }
}

fn make_sexp_parser<'parse, 'source>()
-> impl Parser<'parse, SkippingStr<'source>, SExp, IdParseEnvironment>
where
    'source: 'parse,
{
    Recursive::new_as("sexp", |this| {
        // Symbol
        just((Kind::String, regex::Regex::new(r"[a-zA-Z0-9_-]+").unwrap()))
            .into_owned()
            .associate(Kind::Symbol, SExp::Symbol)
            .or(just((Kind::LeftParen, "("))
                .ignore_then(this.many())
                .then_ignore(just((Kind::RightParen, ")")))
                .associate_as("list", Kind::List, SExp::List))
    })
}

#[test]
fn symbol() {
    const DATA: &str = "kebab-case";
    let input = SkippingStr::whitespace(DATA, Kind::Whitespace);
    let artifact = make_sexp_parser().parse_all(&input);

    let (ast, cst) = artifact.unwrap().into_parts();
    expect![[r#"
        Symbol(
            Id(
                0,
            ),
            "kebab-case",
        )
    "#]]
    .assert_debug_eq(&ast);
    expect![[r#"
        GreenNode {
            kind: SyntaxKind(
                4,
            ),
            text_len: 10,
            n_children: 1,
        }
    "#]]
    .assert_debug_eq(&cst.search(&Id(0)).unwrap());
}

#[test]
fn nested_lists() {
    const DATA: &str = "(a b c)";
    let input = SkippingStr::whitespace(DATA, Kind::Whitespace);
    let artifact = make_sexp_parser().parse_all(&input);

    let (ast, _) = artifact.unwrap().into_parts();
    expect![[r#"
        List(
            Id(
                3,
            ),
            [
                Symbol(
                    Id(
                        0,
                    ),
                    "a",
                ),
                Symbol(
                    Id(
                        1,
                    ),
                    "b",
                ),
                Symbol(
                    Id(
                        2,
                    ),
                    "c",
                ),
            ],
        )
    "#]]
    .assert_debug_eq(&ast);
}
