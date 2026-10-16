#![cfg(feature = "logos")]

//! This file tests functionalities related to Logos integration.
//! Including token matching, general parser usability (see also `'source / 'src Distinction`),
//! and general parsing correctness.

use std::rc::Rc;

use biparse::prelude::*;
use expect_test::expect;
use logos::Logos;
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
    #[token("*")]
    Asterisk,

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
    Mult,
}

impl Expression {
    fn evaluate(&self) -> BigUint {
        match self {
            Expression::Number(uint) => uint.clone(),
            Expression::Binary {
                left,
                operator,
                right,
            } => match operator {
                Operator::Add => left.evaluate() + right.evaluate(),
                Operator::Mult => left.evaluate() * right.evaluate(),
            },
        }
    }
}

/// Assertion that the CST constructed is total.
/// Panicking.
fn cst_is_total<L: Language>(source: &str, tree: &SyntaxNode<L>) {
    assert_eq!(tree.text(), source);

    let raw_range = {
        let range = tree.text_range();
        range.start().into()..range.end().into()
    };
    assert_eq!(raw_range, (0..source.len()));
}

/// Lexer trivia recovery: restore skipped whitespace to `Kind::Trivia`.
#[test]
fn lexer_trivia_recovery() {
    let source = "(   )";
    let stream: LexerStream<'_, Kind> = LexerStream::new(source, Kind::Trivia);

    let parser: Rc<dyn Parser<'_, LexerStream<Kind>, _, DefaultParseEnvironment>> =
        just(Kind::LeftParen).then(just(Kind::RightParen)).shared();

    match parser.parse_all(&stream) {
        Ok(report) => {
            assert_eq!(report.ast_only(), &(Kind::LeftParen, Kind::RightParen));
            let (tree, _) = report.cst().canonize_with(Kind::Root);

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

// Tests relating to `Pattern` matching of tokens.
#[test]
fn token_plus() {
    let source = "+";
    let stream: LexerStream<'_, Kind> = LexerStream::new(source, Kind::Trivia);
    let parser: Box<dyn Parser<'_, LexerStream<'_, Kind>, Operator, DefaultParseEnvironment>> =
        just(Kind::Plus).map(|_| Operator::Add).boxed();

    let result = parser.parse_all(&stream);
    expect![[r#"
        Add
    "#]]
    .assert_debug_eq(result.unwrap().ast_only());
}
#[test]
fn token_asterisk() {
    let source = "*";
    let stream: LexerStream<'_, Kind> = LexerStream::new(source, Kind::Trivia);
    let parser: Box<dyn Parser<'_, LexerStream<'_, Kind>, Operator, DefaultParseEnvironment>> =
        just(Kind::Asterisk).map(|_| Operator::Mult).boxed();

    let result = parser.parse_all(&stream);
    expect![[r#"
        Mult
    "#]]
    .assert_debug_eq(result.unwrap().ast_only());
}
/// Regression:
/// Erroneous unconditional matching on ANY present token, regardless of type.
#[test]
fn mismatched_token() {
    let source = "+";
    let stream: LexerStream<'_, Kind> = LexerStream::new(source, Kind::Trivia);
    let parser: Box<dyn Parser<'_, LexerStream<'_, Kind>, Kind, DefaultParseEnvironment>> =
        just(Kind::Root).boxed();
    let result = parser.parse_all(&stream);
    assert!(result.is_err());
    expect![[r#"
        Simple {
            position: LineColumn {
                line: 1,
                column: 1,
            },
            messages: [
                "expected Expectation(Pattern { kind: \"token\", kind_plural: \"tokens\", data: \"Root\" }), found token 'Ok(Plus)' at location (1, 1)",
            ],
        }
    "#]].assert_debug_eq(&result.unwrap_err());
}
#[test]
fn unrecognized_token() {
    let source = "?";
    let stream: LexerStream<'_, Kind> = LexerStream::new(source, Kind::Trivia);
    let parser: Box<dyn Parser<'_, LexerStream<'_, Kind>, Kind, DefaultParseEnvironment>> =
        just(Kind::Root).boxed();
    let result = parser.parse_all(&stream);
    assert!(result.is_err());
    expect![[r#"
        Simple {
            position: LineColumn {
                line: 1,
                column: 1,
            },
            messages: [
                "expected Expectation(Pattern { kind: \"token\", kind_plural: \"tokens\", data: \"Root\" }), found token 'Err(())' at location (1, 1)",
            ],
        }
    "#]].assert_debug_eq(&result.unwrap_err());
}

/// Parser that parses an arithmatic equation involving parentheses, addition and multipliciation.
/// Note on Usage:
/// - The parser constructor *must* be made *after* the stream is constructed due to drop order.
/// - `'source : 'src` and that `LexerStream<'source>` must *not* be `LexerStream<'src, _>`.
fn make_add_mult_parser<'src, 'source>()
-> impl Parser<'src, LexerStream<'source, Kind>, Box<Expression>, DefaultParseEnvironment>
where
    'source: 'src,
{
    let mut expr: Recursive<
        Indirect<'_, '_, LexerStream<'_, Kind>, Box<Expression>, DefaultParseEnvironment>,
    > = Recursive::declare_as("addmult expression");

    let num = satisfies(
        Kind::Number.into(),
        |input: &LexerStream<'_, Kind>, item| match item.token {
            Ok(Kind::Number) => input
                .slice_bytes(item.span.start, item.span.end)
                .parse::<BigUint>()
                .map_err(|error| Box::new(std::io::Error::other(error))),
            _ => Err(Box::new(std::io::Error::other("expected number"))),
        },
    )
    .map(|uint| Box::new(Expression::Number(uint)))
    .shared();

    let terminal = num
        .clone()
        .or_same(
            just(Kind::LeftParen)
                .ignore_then(expr.clone())
                .then_ignore(just(Kind::RightParen)),
        )
        .shared();

    let mult = Recursive::new_as("multiplication", |this| {
        this.then(just(Kind::Asterisk).map(|_| Operator::Mult))
            .then(terminal.clone())
            .map(|((left, operator), right)| {
                Box::new(Expression::Binary {
                    left,
                    operator,
                    right,
                })
            })
            .or_same(terminal)
    });

    let add = Recursive::new_as("addition", |this| {
        this.then(just(Kind::Plus).map(|_| Operator::Add))
            .then(mult.clone())
            .map(|((left, operator), right)| {
                Box::new(Expression::Binary {
                    left,
                    operator,
                    right,
                })
            })
            .or_same(mult)
    });

    expr.define(add);
    expr
}

#[test]
fn add_mult_number() {
    let source = r"42";
    let stream: LexerStream<'_, Kind> = LexerStream::new(source, Kind::Trivia);
    let parser = make_add_mult_parser();
    let result = parser.parse_all(&stream);
    expect![[r#"
        Number(
            42,
        )
    "#]]
    .assert_debug_eq(&result.as_ref().unwrap().ast_only());

    let cst = result.unwrap().cst().canonize_with(Kind::Root).0;
    cst_is_total(source, &cst);
}

#[test]
fn add_mult_1() {
    let source = r"1 * 2";
    let stream: LexerStream<'_, Kind> = LexerStream::new(source, Kind::Trivia);
    let parser = make_add_mult_parser();
    let (ast, cst) = parser.parse_all(&stream).expect("success!").into_parts();

    expect![[r#"
        Binary {
            left: Number(
                1,
            ),
            operator: Mult,
            right: Number(
                2,
            ),
        }
    "#]]
    .assert_debug_eq(&ast);

    let cst = cst.canonize_with(Kind::Root).0;
    cst_is_total(source, &cst);
}
#[test]
fn add_mult_2() {
    let source = r"3 + 5 * 2";
    let stream: LexerStream<'_, Kind> = LexerStream::new(source, Kind::Trivia);
    let tokens: Vec<Kind> = logos::Lexer::new(source).map(Result::unwrap).collect();
    expect![[r#"
        [
            Number,
            Plus,
            Number,
            Asterisk,
            Number,
        ]
    "#]]
    .assert_debug_eq(&tokens);

    let parser = make_add_mult_parser();
    let (ast, cst) = parser.parse_all(&stream).expect("success!").into_parts();

    expect![[r#"
        Binary {
            left: Number(
                3,
            ),
            operator: Add,
            right: Binary {
                left: Number(
                    5,
                ),
                operator: Mult,
                right: Number(
                    2,
                ),
            },
        }
    "#]]
    .assert_debug_eq(&ast);

    let evaluation = ast.evaluate();
    let answer = BigUint::from(13_usize);
    assert_eq!(evaluation, answer);

    let cst = cst.canonize_with(Kind::Root).0;
    cst_is_total(source, &cst);
}

/// Demonstrate left-association (which is how `make_add_mult_parser` is constructed).
#[test]
fn add_mult_3() {
    let source = r"4 + 5 + 6 + 7";
    let stream: LexerStream<'_, Kind> = LexerStream::new(source, Kind::Trivia);
    let parser = make_add_mult_parser();
    let (ast, cst) = parser.parse_all(&stream).expect("success!").into_parts();

    expect![[r#"
        Binary {
            left: Binary {
                left: Binary {
                    left: Number(
                        4,
                    ),
                    operator: Add,
                    right: Number(
                        5,
                    ),
                },
                operator: Add,
                right: Number(
                    6,
                ),
            },
            operator: Add,
            right: Number(
                7,
            ),
        }
    "#]]
    .assert_debug_eq(&ast);

    let cst = cst.canonize_with(Kind::Root).0;
    cst_is_total(source, &cst);
}

/// Test grouping.
#[test]
fn add_mult_grouping() {
    let source = r"(1 + 2) * 3 + ((4 + 5) * 6)";
    let stream: LexerStream<'_, Kind> = LexerStream::new(source, Kind::Trivia);
    let parser = make_add_mult_parser();
    let (ast, cst) = parser.parse_all(&stream).expect("success!").into_parts();

    expect![[r#"
        Binary {
            left: Binary {
                left: Binary {
                    left: Number(
                        1,
                    ),
                    operator: Add,
                    right: Number(
                        2,
                    ),
                },
                operator: Mult,
                right: Number(
                    3,
                ),
            },
            operator: Add,
            right: Binary {
                left: Binary {
                    left: Number(
                        4,
                    ),
                    operator: Add,
                    right: Number(
                        5,
                    ),
                },
                operator: Mult,
                right: Number(
                    6,
                ),
            },
        }
    "#]]
    .assert_debug_eq(&ast);

    let evaluation = ast.evaluate();
    let answer = BigUint::from(63_usize);
    assert_eq!(evaluation, answer);

    let cst = cst.canonize_with(Kind::Root).0;
    cst_is_total(source, &cst);
}
