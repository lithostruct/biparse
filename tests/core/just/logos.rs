#![cfg(feature = "logos")]

use std::rc::Rc;

use biparse::prelude::*;
use expect_test::expect;
use logos::{Lexer, Logos};
use num_bigint::BigUint;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use rowan::{Language, SyntaxKind, SyntaxNode};

#[derive(
    Debug, Logos, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, FromPrimitive, ToPrimitive,
)]
#[logos(skip(r"[ \t\n]+"))]
enum Kind {
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Asterisk,
    #[token("/")]
    Slash,
    #[regex("[0-9]+")]
    Number,

    Trivia,
    Root,
}
impl rowan::Language for Kind {
    type Kind = Self;
    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        Self::from_u16(raw.0).unwrap()
    }
    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        SyntaxKind(kind.to_u16().unwrap())
    }
}
impl From<SyntaxKind> for Kind {
    fn from(value: SyntaxKind) -> Self {
        Kind::kind_from_raw(value)
    }
}
impl From<Kind> for SyntaxKind {
    fn from(value: Kind) -> Self {
        Kind::kind_to_raw(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Expression {
    Number(BigUint),

    Binary {
        left: Box<Expression>,
        operator: Operator,
        right: Box<Expression>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Operator {
    Add,
    Sub,
    Mult,
    Div,
}

/// Assertion that the CST constructed is total.
fn cst_is_total<L: Language>(source: &str, tree: &SyntaxNode<L>) {
    assert_eq!(tree.text(), source);

    let raw_range = {
        let range = tree.text_range();
        range.start().into()..range.end().into()
    };
    assert_eq!(raw_range, (0..source.len()));
}

#[test]
fn lexer_trivia_recovery() {
    let source = "(   )";
    let stream: LexerStream<'_, Kind> = LexerStream::new(source, Kind::Trivia);

    let parser: Rc<dyn Parser<'_, LexerStream<Kind>, _, DefaultParseEnvironment>> =
        just(Kind::LeftParen).then(just(Kind::RightParen)).shared();

    match parser.parse_all(&stream) {
        Ok((ast, cst)) => {
            assert_eq!(ast, (Kind::LeftParen, Kind::RightParen));
            let (tree, _) = cst.canonize_with(Kind::Root);

            assert_eq!(tree.kind(), Kind::Root);
            cst_is_total(source, &tree);

            assert_eq!(tree.children_with_tokens().count(), 3);
            expect![[r#"
                Root@0..5
                  LeftParen@0..1 "("
                  Trivia@1..4 "   "
                  RightParen@4..5 ")"

            "#]]
            .assert_debug_eq(&tree);
        }
        Err(e) => {
            unreachable!("{e}")
        }
    }
}

fn make_add_mult_parser<'src>()
-> impl Parser<'src, LexerStream<'src, Kind>, Box<Expression>, DefaultParseEnvironment> {
    let mut expr: Recursive<
        Indirect<'_, '_, LexerStream<'_, Kind>, Box<Expression>, DefaultParseEnvironment>,
    > = Recursive::declare_as("addition or multiplication expression");

    let num = satisfies(
        Kind::Number.into(),
        |input: &LexerStream<'_, Kind>, item| match item.token {
            Ok(Kind::Number) => Ok(input
                .slice_bytes(item.span.start, item.span.end)
                .parse::<BigUint>()
                .unwrap()),
            _ => Err(Box::new(std::io::Error::other("expected number"))),
        },
    )
    .map(|uint| Box::new(Expression::Number(uint)));

    let add = expr
        .clone()
        .then(just(Kind::Asterisk).map(|_| Operator::Mult))
        .then(expr.clone())
        .map(|((left, operator), right)| {
            Box::new(Expression::Binary {
                left,
                operator,
                right,
            })
        })
        .shared();
    let mult = add
        .clone()
        .then(just(Kind::Asterisk).map(|_| Operator::Mult))
        .then(add.clone())
        .map(|((left, operator), right)| {
            Box::new(Expression::Binary {
                left,
                operator,
                right,
            })
        })
        .shared();

    expr.define(
        // Parenthesized expression.
        just(Kind::LeftParen)
            .ignore_then(expr.clone())
            .then_ignore(just(Kind::RightParen))
            // Mult expression.
            .or_same(mult)
            // Add expression.
            .or_same(add)
            // Literal number.
            .or_same(num),
    );
    expr
}

#[test]
fn add_mult_1() {
    let stream1: LexerStream<'_, Kind> = LexerStream::new("1 + 2 * 3", Kind::Trivia);
    let result1 = make_add_mult_parser().parse_all(&stream1);
    expect![].assert_debug_eq(&result1.unwrap().0);

    // let result2 = make_add_mult_parser().parse_all(&stream2);
    // expect![].assert_debug_eq(&result2.unwrap().0);
}
