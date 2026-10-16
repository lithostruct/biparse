//! Integration test demonstrating a JSON parser.
#![cfg(feature = "regex")]

use std::str::FromStr;

use biparse::prelude::*;
use expect_test::expect;
use indoc::indoc;
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
/// - Instead of the optional or repetition constructs `many`, `some` and `optional`, this parser uses recursion.
///   This is generally not recommended and is written before such constructs were introduced.
/// - Due to the whitespace-preserving nature of `str`-based inputs, optional whitespace is consumed manually.
///   This may be considered verbose, and users may prefer `LexerStream`-based solutions.
///   Refer to [TODO].
fn json_parser<'src, E: ParseEnvironment>() -> impl Parser<'src, str, Json, E> {
    let mut value: Recursive<Indirect<'_, '_, str, Json, E>> = Recursive::declare_as("json");

    // Primitives
    let optional_whitespace = Recursive::new_as("whitespace", |this| {
        just((Kind::Whitespace, ' '))
            .or(just((Kind::Whitespace, '\t')))
            .or(just((Kind::Whitespace, '\n')))
            .then(this)
            .or(success())
            .ignore()
    })
    .node(Kind::Whitespace)
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
    let number = just((Kind::Number, regex_lite::Regex::new(r"-?[0-9]+").unwrap()))
        .map(|inner| BigInt::from_str(inner).unwrap())
        .shared();

    // corresponds to (',' value)*.
    let values: Recursive<dyn Parser<'_, str, Vec<Json>, _>> =
        Recursive::new_as("values", |this| {
            this.then(
                optional_whitespace
                    .clone()
                    .ignore_then(just((Kind::Comma, ',')))
                    .ignore_then(optional_whitespace.clone())
                    .ignore_then(value.clone()),
            )
            .map(|(mut rest, last): (Vec<Json>, Json)| {
                rest.push(last);
                rest
            })
            .or(success())
            .map(|either: Either<Vec<Json>, ()>| either.map_right(|()| Vec::new()).into_inner())
        });
    // corresponds to array ::= '[' (value (',' value)*)? ']'
    let array = just((Kind::LeftBracket, '['))
        .ignore_then(optional_whitespace.clone())
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
        .then_ignore(optional_whitespace.clone())
        .then_ignore(just((Kind::RightBracket, ']')))
        .map(Json::Array)
        .node(Kind::Array);

    // key-value pair.
    let key_value = string
        .clone()
        .then_ignore(optional_whitespace.clone())
        .then_ignore(just((Kind::Colon, ':')))
        .then_ignore(optional_whitespace.clone())
        .then(value.clone())
        .node(Kind::Pair);
    // corresponds to (',' pair)*
    let members: Recursive<dyn Parser<'_, str, Vec<(String, Json)>, _>> = Recursive::new_as(
        "members",
        |this: Recursive<dyn Parser<'_, str, Vec<(String, Json)>, _>>| {
            this.then(
                optional_whitespace
                    .clone()
                    .ignore_then(just((Kind::Comma, ',')))
                    .ignore_then(optional_whitespace.clone())
                    .ignore_then(key_value.clone()),
            )
            .map(|(mut rest, last)| {
                rest.push(last);
                rest
            })
            .or(success())
            .map(|either: Either<Vec<(String, Json)>, ()>| {
                either.map_right(|()| Vec::new()).into_inner()
            })
        },
    );
    // corresponds to object ::= '{' (pair (',' pair)*)? '}'
    let object = just((Kind::LeftBracket, '{'))
        .ignore_then(optional_whitespace.clone())
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
        .then_ignore(optional_whitespace.clone())
        .then_ignore(just((Kind::RightBracket, '}')))
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
          Whitespace@0..0
          Null@0..4 "null"
          Whitespace@4..4

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
          Whitespace@0..0
          True@0..4 "true"
          Whitespace@4..4

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
          Whitespace@0..0
          False@0..5 "false"
          Whitespace@5..5

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
          Whitespace@0..0
          Number@0..5 "12345"
          Whitespace@5..5

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
    let (ast, cst) = parser.parse_all(data)
        .expect("success!")
        .into_parts();

    expect![[r#"
        Success(
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
            ),
        )
    "#]]
    .assert_debug_eq(&ast);

    let tree: rowan::SyntaxNode<Kind> = cst.canonize_with(Kind::Root).0;
    assert_eq!(tree.text(), data);
}
