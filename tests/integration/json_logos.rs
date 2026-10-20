//! Integration test demonstrating a JSON parser.
#![cfg(feature = "regex")]

use std::str::FromStr;

use biparse::prelude::*;
use expect_test::expect;
use indoc::indoc;
use logos::Logos;
use num_bigint::BigInt;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use rowan::Language;

#[allow(unused)]
#[derive(Debug, Clone)]
enum Json {
    Null,
    Boolean(bool),
    String(String),
    Number(BigInt),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
}
impl Json {
    /// Quick and dirty serializer.
    ///
    /// You would prefer a proper document printer for this sort of purpose.
    fn to_flat_text(&self) -> String {
        match self {
            Json::Null => "null".into(),
            Json::Boolean(bool) => format!("{bool:?}"),
            Json::String(s) => format!("\"{:}\"", s),
            Json::Number(big_int) => format!("{big_int}"),
            Json::Array(elements) => {
                let mut result = String::from("{");
                let mut elem_iter = elements.iter();
                while let Some(item) = elem_iter.next() {
                    result += &item.to_flat_text();
                    if elem_iter.len() != 0 {
                        result += ", "
                    }
                }
                result += "}";
                result
            }
            Json::Object(items) => {
                let mut result = String::from("{");
                let mut item_iter = items.iter();
                while let Some(item) = item_iter.next() {
                    result += &format!("\"{:}\": ", item.0);
                    result += &item.1.to_flat_text();
                    if item_iter.len() != 0 {
                        result += ", "
                    }
                }
                result += "}";
                result
            }
        }
    }
}

#[derive(
    FromPrimitive, ToPrimitive, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Logos,
)]
#[repr(u16)]
enum Kind {
    // Literals
    #[token("null")]
    Null = 0,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("[")]
    LeftBracket,
    #[token("]")]
    RightBracket,
    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,
    #[token(":")]
    Colon,
    #[token(",")]
    Comma,

    #[regex(r"-?[0-9]+", priority = 3)]
    Number,
    #[regex(r#""[a-zA-Z0-9_ ]+""#)]
    RawText,

    #[regex(r"[ \t\n]+", logos::skip)]
    Whitespace,

    // Nodes
    String,
    Array,
    Pair,
    Object,
    Root,
}
impl Language for Kind {
    type Kind = Kind;
    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        Kind::from_u16(raw.0).unwrap()
    }
    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind.to_u16().unwrap())
    }
}
impl From<Kind> for rowan::SyntaxKind {
    fn from(val: Kind) -> Self {
        Kind::kind_to_raw(val)
    }
}

/// Constructs a JSON parser.
fn json_parser<'src, 'source, E: ParseEnvironment>()
-> impl Parser<'src, LexerStream<'source, Kind>, Json, E>
where
    'source: 'src,
{
    let mut value: Recursive<Indirect<'_, '_, LexerStream<'_, Kind>, Json, E>> =
        Recursive::declare_as("json");

    // Primitives.

    let null = just(Kind::Null).map(|_| Json::Null);
    let boolean = just(Kind::True)
        .or(just(Kind::False))
        .map(|either| match either {
            Either::Left(_) => Json::Boolean(true),
            Either::Right(_) => Json::Boolean(false),
        });

    // Simplifying assumption:
    // Strings consist of characters matching the following `Regex`, with no regard for escape characters.
    // - Alphanumeric, space or underscore.
    //
    // Divergence from `str`-based implementation which can demarcate between `DoubleQuote` tokens,
    // which are required in this `logos`-based implementation to distinct this from whitespace matching.
    // This is not an issue for `str`-based implementations due to explicit control flow.
    //
    // Hence, the CST produced by this combinator is different.
    let string = just(WithText(Kind::RawText))
        .map(|(_, fragment)| {
            let mut s = fragment.into_owned();
            s.remove(0);
            s.pop();
            s
        })
        .node(Kind::String)
        .shared();

    // Simplifying assumption:
    // Numbers are integers only, and can be cast to `BigInt` without failing.
    let number = just(WithText(Kind::Number))
        .map(|(_, s)| BigInt::from_str(&s).unwrap())
        .shared();

    // Corresponds to (',' value)*.
    // This used to be implemented with recursion, but has since been supplanted by the `Many` combinator.
    let values = just(Kind::Comma).ignore_then(value.clone()).many();

    // Corresponds to array ::= '[' (value (',' value)*)? ']'
    let array = just(Kind::LeftBracket)
        .ignore_then(
            value
                .clone()
                .then(values)
                .map(|(first, mut rest)| {
                    rest.insert(0, first);
                    rest
                })
                .or(success())
                .map(Either::left_or_default),
        )
        .then_ignore(just(Kind::RightBracket))
        .map(Json::Array)
        .node(Kind::Array);

    // Key-value pair.
    let key_value = string
        .clone()
        .then_ignore(just(Kind::Colon))
        .then(value.clone())
        .node(Kind::Pair);

    // Corresponds to (',' pair)*
    let members = just(Kind::Comma).ignore_then(key_value.clone()).many();

    // Corresponds to object ::= '{' (pair (',' pair)*)? '}'
    let object = just(Kind::LeftBrace)
        .ignore_then(
            key_value
                .clone()
                .then(members)
                .map(|(first, mut rest)| {
                    rest.insert(0, first);
                    rest
                })
                .or(success())
                .map(Either::left_or_default),
        )
        .then_ignore(just(Kind::RightBrace))
        .map(Json::Object)
        .node(Kind::Object);

    // Finally!
    value.define(
        null.or_same(boolean)
            .or_same(string.map(Json::String))
            .or_same(number.map(Json::Number))
            .or_same(array)
            .or_same(object),
    );

    value
}

/// Nullary value.
#[test]
fn json_null() {
    let data = LexerStream::new("null", Kind::Whitespace);
    let parser = json_parser::<DefaultParseEnvironment>();
    let result = parser.parse_all(&data).unwrap();
    expect![[r#"
        Null
    "#]]
    .assert_debug_eq(result.ast_only());

    let tree: rowan::SyntaxNode<Kind> = result.cst().canonize_with(Kind::Root).0;
    assert_eq!(tree.text(), data.source());
    expect![[r#"
        Root@0..4
          Null@0..4 "null"

    "#]]
    .assert_debug_eq(&tree);
}

/// Boolean `true`.
#[test]
fn json_true() {
    let data = LexerStream::new("true", Kind::Whitespace);
    let parser = json_parser::<DefaultParseEnvironment>();
    let result = parser.parse_all(&data).unwrap();
    expect![[r#"
        Boolean(
            true,
        )
    "#]]
    .assert_debug_eq(result.ast_only());

    let tree: rowan::SyntaxNode<Kind> = result.cst().canonize_with(Kind::Root).0;
    assert_eq!(tree.text(), data.source());
    expect![[r#"
        Root@0..4
          True@0..4 "true"

    "#]]
    .assert_debug_eq(&tree);
}
/// Boolean `false`.
#[test]
fn json_false() {
    let data = LexerStream::new("false", Kind::Whitespace);
    let parser = json_parser::<DefaultParseEnvironment>();
    let result = parser.parse_all(&data).unwrap();
    expect![[r#"
        Boolean(
            false,
        )
    "#]]
    .assert_debug_eq(result.ast_only());

    let tree: rowan::SyntaxNode<Kind> = result.cst().canonize_with(Kind::Root).0;
    assert_eq!(tree.text(), data.source());
    expect![[r#"
        Root@0..5
          False@0..5 "false"

    "#]]
    .assert_debug_eq(&tree);
}

/// Number.
#[test]
fn json_num() {
    let data = LexerStream::new("12345", Kind::Whitespace);
    let parser = json_parser::<DefaultParseEnvironment>();
    let result = parser.parse_all(&data).unwrap();
    expect![[r#"
        Number(
            12345,
        )
    "#]]
    .assert_debug_eq(result.ast_only());

    let tree: rowan::SyntaxNode<Kind> = result.cst().canonize_with(Kind::Root).0;
    assert_eq!(tree.text(), data.source());
    expect![[r#"
        Root@0..5
          Number@0..5 "12345"

    "#]]
    .assert_debug_eq(&tree);
}

/// Array.
#[test]
fn json_array() {
    let data = LexerStream::new(r#"[ "a", "b", "c", [1,2,3] ]"#, Kind::Whitespace);
    let parser = json_parser::<DefaultParseEnvironment>();
    let result = parser.parse_all(&data).unwrap();
    expect![[r#"
        Array(
            [
                String(
                    "a",
                ),
                String(
                    "b",
                ),
                String(
                    "c",
                ),
                Array(
                    [
                        Number(
                            1,
                        ),
                        Number(
                            2,
                        ),
                        Number(
                            3,
                        ),
                    ],
                ),
            ],
        )
    "#]]
    .assert_debug_eq(result.ast_only());

    let tree: rowan::SyntaxNode<Kind> = result.cst().canonize_with(Kind::Root).0;
    assert_eq!(tree.text(), data.source());
    expect![[r#"
        Root@0..26
          Array@0..26
            LeftBracket@0..1 "["
            Whitespace@1..2 " "
            String@2..5
              RawText@2..5 "\"a\""
            Comma@5..6 ","
            Whitespace@6..7 " "
            String@7..10
              RawText@7..10 "\"b\""
            Comma@10..11 ","
            Whitespace@11..12 " "
            String@12..15
              RawText@12..15 "\"c\""
            Comma@15..16 ","
            Whitespace@16..17 " "
            Array@17..25
              LeftBracket@17..18 "["
              Number@18..19 "1"
              Comma@19..20 ","
              Number@20..21 "2"
              Comma@21..22 ","
              Number@22..23 "3"
              RightBracket@23..24 "]"
              Whitespace@24..25 " "
            RightBracket@25..26 "]"

    "#]]
    .assert_debug_eq(&tree);
}

/// Representative data of this crate!
#[test]
fn package_data() {
    let source = indoc! {r#"
        {
            "name": "biparse",
            "version": 0,
            "features": [
                "recursion",
                "forest",
                { "left_recursive": true }
            ]
        }
    "#};
    let data = LexerStream::new(source, Kind::Whitespace);
    let parser = json_parser::<DefaultParseEnvironment>();
    let (ast, cst) = parser.parse_all(&data).expect("success!").into_parts();

    expect![[r#"
        Object(
            [
                (
                    "name",
                    String(
                        "biparse",
                    ),
                ),
                (
                    "version",
                    Number(
                        0,
                    ),
                ),
                (
                    "features",
                    Array(
                        [
                            String(
                                "recursion",
                            ),
                            String(
                                "forest",
                            ),
                            Object(
                                [
                                    (
                                        "left_recursive",
                                        Boolean(
                                            true,
                                        ),
                                    ),
                                ],
                            ),
                        ],
                    ),
                ),
            ],
        )
    "#]]
    .assert_debug_eq(&ast);
    expect![[r#"{"name": "biparse", "version": 0, "features": {"recursion", "forest", {"left_recursive": true}}}"#]]
        .assert_eq(&ast.to_flat_text());

    let tree: rowan::SyntaxNode<Kind> = cst.canonize_with(Kind::Root).0;
    assert_eq!(tree.text(), data.source());
    expect![[r#"
        Root@0..143
          Object@0..143
            LeftBrace@0..1 "{"
            Whitespace@1..6 "\n    "
            Pair@6..23
              String@6..12
                RawText@6..12 "\"name\""
              Colon@12..13 ":"
              Whitespace@13..14 " "
              String@14..23
                RawText@14..23 "\"biparse\""
            Comma@23..24 ","
            Whitespace@24..29 "\n    "
            Pair@29..41
              String@29..38
                RawText@29..38 "\"version\""
              Colon@38..39 ":"
              Whitespace@39..40 " "
              Number@40..41 "0"
            Comma@41..42 ","
            Whitespace@42..47 "\n    "
            Pair@47..141
              String@47..57
                RawText@47..57 "\"features\""
              Colon@57..58 ":"
              Whitespace@58..59 " "
              Array@59..141
                LeftBracket@59..60 "["
                Whitespace@60..69 "\n        "
                String@69..80
                  RawText@69..80 "\"recursion\""
                Comma@80..81 ","
                Whitespace@81..90 "\n        "
                String@90..98
                  RawText@90..98 "\"forest\""
                Comma@98..99 ","
                Whitespace@99..108 "\n        "
                Object@108..139
                  LeftBrace@108..109 "{"
                  Whitespace@109..110 " "
                  Pair@110..133
                    String@110..126
                      RawText@110..126 "\"left_recursive\""
                    Colon@126..127 ":"
                    Whitespace@127..128 " "
                    True@128..132 "true"
                    Whitespace@132..133 " "
                  RightBrace@133..134 "}"
                  Whitespace@134..139 "\n    "
                RightBracket@139..140 "]"
                Whitespace@140..141 "\n"
            RightBrace@141..142 "}"
            Whitespace@142..143 "\n"

    "#]]
    .assert_debug_eq(&tree);
}
