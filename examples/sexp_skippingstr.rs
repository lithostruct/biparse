//! Simple symbolic expression parser using [`SkippingStr`].

use biparse::{input::skipping_str::SkippingStr, prelude::*};
use expect_test::expect;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
enum Kind {
    // Terminals.
    Whitespace = 0,
    LeftParen,
    RightParen,
    Atom,

    // Non-terminals
    List,
    Root,
}
impl rowan::Language for Kind {
    type Kind = Self;
    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        match raw.0 {
            0 => Kind::Whitespace,
            1 => Kind::LeftParen,
            2 => Kind::RightParen,
            3 => Kind::Atom,
            4 => Kind::List,
            5 => Kind::Root,
            _ => unreachable!(),
        }
    }
    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(match kind {
            Kind::Whitespace => 0,
            Kind::LeftParen => 1,
            Kind::RightParen => 2,
            Kind::Atom => 3,
            Kind::List => 4,
            Kind::Root => 5,
        })
    }
}
#[allow(clippy::from_over_into)]
impl Into<rowan::SyntaxKind> for Kind {
    fn into(self) -> rowan::SyntaxKind {
        <Kind as rowan::Language>::kind_to_raw(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SExp {
    Atom(String),
    List(Vec<SExp>),
}

fn sexp_parser<'source, 'parse, E: ParseEnvironment>()
-> impl Parser<'parse, SkippingStr<'source>, SExp, E>
where
    'source: 'parse,
{
    let atom = just::<'parse, SkippingStr<'source>, E, _>((
        Kind::Atom,
        regex::Regex::new("[a-zA-Z0-9_-]+").unwrap(),
    ));
    Recursive::new(|sexp| {
        atom.into_owned()
            .map(SExp::Atom)
            .named("atom", Kind::Atom)
            .or(just((Kind::LeftParen, '('))
                .ignore_then(sexp.many().map(SExp::List))
                .then_ignore(just((Kind::RightParen, ')')))
                .named("list", Kind::List))
    })
}

fn main() {
    let input = SkippingStr::whitespace("((1) (2 3) (4 5 6))", Kind::Whitespace);
    let result = sexp_parser::<environment::Rich>().parse_all(&input);
    let artifact = result.expect("parse success");

    assert_eq!(
        *artifact.ast_only(),
        SExp::List(vec![
            SExp::List(vec![SExp::Atom("1".to_string()),]),
            SExp::List(vec![
                SExp::Atom("2".to_string()),
                SExp::Atom("3".to_string()),
            ]),
            SExp::List(vec![
                SExp::Atom("4".to_string()),
                SExp::Atom("5".to_string()),
                SExp::Atom("6".to_string()),
            ]),
        ])
    );

    let red_tree = artifact.cst().clone().canonize::<Kind>(Kind::Root);
    expect![[r#"
        Root@0..19
          List@0..19
            LeftParen@0..1 "("
            List@1..5
              LeftParen@1..2 "("
              Atom@2..3
                Atom@2..3 "1"
              RightParen@3..4 ")"
              Whitespace@4..5 " "
            List@5..11
              LeftParen@5..6 "("
              Atom@6..8
                Atom@6..7 "2"
                Whitespace@7..8 " "
              Atom@8..9
                Atom@8..9 "3"
              RightParen@9..10 ")"
              Whitespace@10..11 " "
            List@11..18
              LeftParen@11..12 "("
              Atom@12..14
                Atom@12..13 "4"
                Whitespace@13..14 " "
              Atom@14..16
                Atom@14..15 "5"
                Whitespace@15..16 " "
              Atom@16..17
                Atom@16..17 "6"
              RightParen@17..18 ")"
            RightParen@18..19 ")"

    "#]]
    .assert_debug_eq(red_tree.tree());
}
