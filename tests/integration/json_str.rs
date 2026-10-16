//! Integration test demonstrating a JSON parser.
#![cfg(feature = "regex")]

use std::str::FromStr;

use biparse::prelude::*;
use expect_test::expect;
use indoc::indoc;
use num_bigint::BigInt;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use regex_lite::Regex;
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

#[derive(FromPrimitive, ToPrimitive, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
enum Kind {
    // Literals
    Null = 0,
    True,
    False,
    RawText,
    DoubleQuote,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,
    Colon,
    Comma,
    Whitespace,

    // Nodes
    Number,
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
///
/// Caveats:
/// - Due to the whitespace-preserving nature of `str`-based inputs, optional whitespace is consumed manually and explicitly.
///   Users may be consider this verbose or error-prone, and prefer `LexerStream`-based solutions instead.
///   Refer to [TODO].
fn json_parser<'src, E: ParseEnvironment>() -> impl Parser<'src, str, Json, E> {
    let mut value: Recursive<Indirect<'_, '_, str, Json, E>> = Recursive::declare_as("json");

    // Primitives.
    let optional_whitespace = just((Kind::Whitespace, Regex::new(r"[ \t\n]+").unwrap()))
        .optional()
        .ignore()
        .shared();

    let null = just((Kind::Null, "null")).map(|_| Json::Null);
    let boolean = just((Kind::True, "true"))
        .or(just((Kind::False, "false")))
        .map(|either| match either {
            Either::Left(_) => Json::Boolean(true),
            Either::Right(_) => Json::Boolean(false),
        });

    // Simplifying assumption:
    // Strings consist of characters matching the following `Regex`, with no regard for escape characters.
    // - Alphanumeric, space or underscore.
    let string = just((Kind::DoubleQuote, '\"'))
        .ignore_then(just((
            Kind::RawText,
            regex_lite::Regex::new(r"[a-zA-Z0-9_ ]+").unwrap(),
        )))
        .then_ignore(just((Kind::DoubleQuote, '\"')))
        .into_owned()
        .node(Kind::String)
        .shared();

    // Simplifying assumption:
    // Numbers are integers only, and can be cast to `BigInt` without failing.
    let number = just((Kind::Number, Regex::new(r"-?[0-9]+").unwrap()))
        .map(|inner| BigInt::from_str(inner).unwrap())
        .shared();

    // Corresponds to (',' value)*.
    // This used to be implemented with recursion, but has since been supplanted by the `Many` combinator.
    let values = (optional_whitespace
        .clone()
        .then(just((Kind::Comma, ',')))
        .then(optional_whitespace.clone()))
    .ignore_then(value.clone())
    .many();

    // Corresponds to array ::= '[' (value (',' value)*)? ']'
    let array = (just((Kind::LeftBracket, '[')).then(optional_whitespace.clone()))
        .ignore_then(
            value
                .clone()
                .then_ignore(optional_whitespace.clone())
                .then(values)
                .map(|(first, mut rest)| {
                    rest.insert(0, first);
                    rest
                })
                .or(success())
                .map(|either| either.map_right(|_| Vec::new()).into_inner()),
        )
        .then_ignore(
            optional_whitespace
                .clone()
                .then(just((Kind::RightBracket, ']'))),
        )
        .map(Json::Array)
        .node(Kind::Array);

    // Key-value pair.
    let key_value = string
        .clone()
        .then_ignore(
            optional_whitespace
                .clone()
                .then(just((Kind::Colon, ':')))
                .then(optional_whitespace.clone()),
        )
        .then(value.clone())
        .node(Kind::Pair);

    // Corresponds to (',' pair)*
    let members = optional_whitespace
        .clone()
        .ignore_then(just((Kind::Comma, ',')))
        .ignore_then(optional_whitespace.clone())
        .ignore_then(key_value.clone())
        .many();

    // Corresponds to object ::= '{' (pair (',' pair)*)? '}'
    let object = (just((Kind::LeftBracket, '{')).then(optional_whitespace.clone()))
        .ignore_then(
            key_value
                .clone()
                .then_ignore(optional_whitespace.clone())
                .then(members)
                .map(|(first, mut rest)| {
                    rest.insert(0, first);
                    rest
                })
                .or(success())
                .map(|either| either.map_right(|_| Vec::new()).into_inner()),
        )
        .then_ignore(
            optional_whitespace
                .clone()
                .then(just((Kind::RightBracket, '}'))),
        )
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

    // Strip leading and trailing whitespace
    optional_whitespace
        .clone()
        .ignore_then(value)
        .then_ignore(optional_whitespace)
        .then_ignore(end())
}

/// Nullary value.
#[test]
fn json_null() {
    let data = "null";
    let parser = json_parser::<DefaultParseEnvironment>();
    let result = parser.parse_all(data).unwrap();
    expect![[r#"
        Null
    "#]]
    .assert_debug_eq(result.ast_only());

    let tree: rowan::SyntaxNode<Kind> = result.cst().canonize_with(Kind::Root).0;
    assert_eq!(tree.text(), data);
    expect![[r#"
        Root@0..4
          Null@0..4 "null"

    "#]]
    .assert_debug_eq(&tree);
}

/// Boolean `true`.
#[test]
fn json_true() {
    let data = "true";
    let parser = json_parser::<DefaultParseEnvironment>();
    let result = parser.parse_all(data).unwrap();
    expect![[r#"
        Boolean(
            true,
        )
    "#]]
    .assert_debug_eq(result.ast_only());

    let tree: rowan::SyntaxNode<Kind> = result.cst().canonize_with(Kind::Root).0;
    assert_eq!(tree.text(), data);
    expect![[r#"
        Root@0..4
          True@0..4 "true"

    "#]]
    .assert_debug_eq(&tree);
}
/// Boolean `false`.
#[test]
fn json_false() {
    let data = "false";
    let parser = json_parser::<DefaultParseEnvironment>();
    let result = parser.parse_all(data).unwrap();
    expect![[r#"
        Boolean(
            false,
        )
    "#]]
    .assert_debug_eq(result.ast_only());

    let tree: rowan::SyntaxNode<Kind> = result.cst().canonize_with(Kind::Root).0;
    assert_eq!(tree.text(), data);
    expect![[r#"
        Root@0..5
          False@0..5 "false"

    "#]]
    .assert_debug_eq(&tree);
}

/// Number.
#[test]
fn json_num() {
    let data = "12345";
    let parser = json_parser::<DefaultParseEnvironment>();
    let result = parser.parse_all(data).unwrap();
    expect![[r#"
        Number(
            12345,
        )
    "#]]
    .assert_debug_eq(result.ast_only());

    let tree: rowan::SyntaxNode<Kind> = result.cst().canonize_with(Kind::Root).0;
    assert_eq!(tree.text(), data);
    expect![[r#"
        Root@0..5
          Number@0..5 "12345"

    "#]]
    .assert_debug_eq(&tree);
}

/// Array.
#[test]
fn json_array() {
    let data = r#"[ "a", "b", "c", 1,2,3 ]"#;
    let parser = json_parser::<DefaultParseEnvironment>();
    let result = parser.parse_all(data).unwrap();
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
        )
    "#]]
    .assert_debug_eq(result.ast_only());

    let tree: rowan::SyntaxNode<Kind> = result.cst().canonize_with(Kind::Root).0;
    assert_eq!(tree.text(), data);
}

/// Representative data of this crate!
#[test]
fn package_data() {
    let data = indoc! {r#"
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
    let parser = json_parser::<DefaultParseEnvironment>();
    let (ast, cst) = parser.parse_all(data).expect("success!").into_parts();

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
    assert_eq!(tree.text(), data);
    expect![[r#"
        Root@0..143
          Object@0..142
            LeftBracket@0..1 "{"
            Whitespace@1..6 "\n    "
            Pair@6..23
              String@6..12
                DoubleQuote@6..7 "\""
                RawText@7..11 "name"
                DoubleQuote@11..12 "\""
              Colon@12..13 ":"
              Whitespace@13..14 " "
              String@14..23
                DoubleQuote@14..15 "\""
                RawText@15..22 "biparse"
                DoubleQuote@22..23 "\""
            Comma@23..24 ","
            Whitespace@24..29 "\n    "
            Pair@29..41
              String@29..38
                DoubleQuote@29..30 "\""
                RawText@30..37 "version"
                DoubleQuote@37..38 "\""
              Colon@38..39 ":"
              Whitespace@39..40 " "
              Number@40..41 "0"
            Comma@41..42 ","
            Whitespace@42..47 "\n    "
            Pair@47..140
              String@47..57
                DoubleQuote@47..48 "\""
                RawText@48..56 "features"
                DoubleQuote@56..57 "\""
              Colon@57..58 ":"
              Whitespace@58..59 " "
              Array@59..140
                LeftBracket@59..60 "["
                Whitespace@60..69 "\n        "
                String@69..80
                  DoubleQuote@69..70 "\""
                  RawText@70..79 "recursion"
                  DoubleQuote@79..80 "\""
                Comma@80..81 ","
                Whitespace@81..90 "\n        "
                String@90..98
                  DoubleQuote@90..91 "\""
                  RawText@91..97 "forest"
                  DoubleQuote@97..98 "\""
                Comma@98..99 ","
                Whitespace@99..108 "\n        "
                Object@108..134
                  LeftBracket@108..109 "{"
                  Whitespace@109..110 " "
                  Pair@110..132
                    String@110..126
                      DoubleQuote@110..111 "\""
                      RawText@111..125 "left_recursive"
                      DoubleQuote@125..126 "\""
                    Colon@126..127 ":"
                    Whitespace@127..128 " "
                    True@128..132 "true"
                  Whitespace@132..133 " "
                  RightBracket@133..134 "}"
                Whitespace@134..139 "\n    "
                RightBracket@139..140 "]"
            Whitespace@140..141 "\n"
            RightBracket@141..142 "}"
          Whitespace@142..143 "\n"

    "#]]
    .assert_debug_eq(&tree);
}
