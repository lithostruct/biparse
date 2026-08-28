// //! Integration test demonstrating a JSON parser.
// #![cfg(feature = "regex")]

// use std::{rc::Rc, sync::OnceLock};

// use biparse::prelude::*;
// use expect_test::expect;
// use indoc::indoc;
// use num_derive::{FromPrimitive, ToPrimitive};
// use num_traits::{FromPrimitive, ToPrimitive};
// use rowan::Language;

// #[derive(Debug, Clone)]
// enum Json {
//     Null,
//     Boolean(bool),
//     String(String),
//     Array(Vec<Self>),
//     Object(Vec<(String, Self)>),
// }

// #[derive(FromPrimitive, ToPrimitive, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
// #[repr(u16)]
// enum Kind {
//     // Literals
//     Null = 0,
//     True,
//     False,
//     RawText,
//     DoubleQuote,
//     LeftBracket,
//     RightBracket,
//     LeftBrace,
//     RightBrace,
//     Colon,
//     Comma,
//     Whitespace,

//     // Nodes
//     String,
//     Array,
//     Pair,
//     Object,
//     Root,
// }
// impl Language for Kind {
//     type Kind = Kind;
//     fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
//         Kind::from_u16(raw.0).unwrap()
//     }
//     fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
//         rowan::SyntaxKind(kind.to_u16().unwrap())
//     }
// }
// impl Into<rowan::SyntaxKind> for Kind {
//     fn into(self) -> rowan::SyntaxKind {
//         Kind::kind_to_raw(self)
//     }
// }

// // fn json_parser<'src, E: ParseEnvironment>() -> impl Parser<'src, str, Json, E> {
// //     let mut value: Recursive<Indirect<'_, str, Json, E>> = Recursive::declare_as("json");

// //     // Primitives
// //     let optional_whitespace = Recursive::new_as("whitespace", |this| {
// //         just(Kind::Whitespace.into(), ' ')
// //             .or(just(Kind::Whitespace.into(), '\t'))
// //             .or(just(Kind::Whitespace.into(), '\n'))
// //             .then(this)
// //             .or(success())
// //             .ignore()
// //     })
// //     .node(Kind::Whitespace.into())
// //     .shared();

// //     let null = just(Kind::Null.into(), "null").map(|_| Json::Null);
// //     let boolean = just(Kind::True.into(), "true")
// //         .or(just(Kind::False.into(), "false"))
// //         .map(|either| match either {
// //             Either::Left(_) => Json::Boolean(true),
// //             Either::Right(_) => Json::Boolean(false),
// //         });

// //     // Simplifying assumption:
// //     // Strings consist of characters matching the following `Regex`, with no regard for escape characters.
// //     // - Alphanumeric, space or underscore.
// //     let string = just(Kind::DoubleQuote.into(), '\"')
// //         .ignore_then(just(
// //             Kind::RawText.into(),
// //             regex_lite::Regex::new(r"[a-zA-Z0-9_ ]+").unwrap(),
// //         ))
// //         .then_ignore(just(Kind::DoubleQuote.into(), '\"'))
// //         .into_owned()
// //         .node(Kind::String.into())
// //         .shared();

// //     // corresponds to (',' value)*.
// //     let values: Recursive<dyn Parser<'_, str, Vec<Json>, _>> =
// //         Recursive::new_as("values", |this| {
// //             this.then(
// //                 optional_whitespace
// //                     .clone()
// //                     .ignore_then(just(Kind::Comma.into(), ','))
// //                     .ignore_then(optional_whitespace.clone())
// //                     .ignore_then(value.clone()),
// //             )
// //             .map(|(mut rest, last): (Vec<Json>, Json)| {
// //                 rest.push(last);
// //                 rest
// //             })
// //             .or(success())
// //             .map(|either: Either<Vec<Json>, ()>| either.map_right(|()| Vec::new()).into_inner())
// //         });
// //     // corresponds to array ::= '[' (value (',' value)*)? ']'
// //     let array = just(Kind::LeftBracket.into(), '[')
// //         .ignore_then(optional_whitespace.clone())
// //         .ignore_then(
// //             value
// //                 .clone()
// //                 .then_ignore(optional_whitespace.clone())
// //                 .then(values)
// //                 .map(|(first, mut rest)| {
// //                     rest.insert(0, first);
// //                     rest
// //                 })
// //                 .or(success())
// //                 .map(|either| either.map_right(|_| Vec::new()).into_inner()),
// //         )
// //         .then_ignore(optional_whitespace.clone())
// //         .then_ignore(just(Kind::RightBracket.into(), ']'))
// //         .map(Json::Array)
// //         .node(Kind::Array.into());

// //     // key-value pair.
// //     let key_value = string
// //         .clone()
// //         .then_ignore(optional_whitespace.clone())
// //         .then_ignore(just(Kind::Colon.into(), ':'))
// //         .then_ignore(optional_whitespace.clone())
// //         .then(value.clone())
// //         .node(Kind::Pair.into());
// //     // corresponds to (',' pair)*
// //     let members: Recursive<dyn Parser<'_, str, Vec<(String, Json)>, _>> = Recursive::new_as(
// //         "members",
// //         |this: Recursive<dyn Parser<'_, str, Vec<(String, Json)>, _>>| {
// //             this.then(
// //                 optional_whitespace
// //                     .clone()
// //                     .ignore_then(just(Kind::Comma.into(), ','))
// //                     .ignore_then(optional_whitespace.clone())
// //                     .ignore_then(key_value.clone()),
// //             )
// //             .map(|(mut rest, last)| {
// //                 rest.push(last);
// //                 rest
// //             })
// //             .or(success())
// //             .map(|either: Either<Vec<(String, Json)>, ()>| {
// //                 either.map_right(|()| Vec::new()).into_inner()
// //             })
// //         },
// //     );
// //     // corresponds to object ::= '{' (pair (',' pair)*)? '}'
// //     let object = just(Kind::LeftBracket.into(), '{')
// //         .ignore_then(optional_whitespace.clone())
// //         .ignore_then(
// //             key_value
// //                 .clone()
// //                 .then_ignore(optional_whitespace.clone())
// //                 .then(members)
// //                 .map(|(first, mut rest)| {
// //                     rest.insert(0, first);
// //                     rest
// //                 })
// //                 .or(success())
// //                 .map(|either| either.map_right(|_| Vec::new()).into_inner()),
// //         )
// //         .then_ignore(optional_whitespace.clone())
// //         .then_ignore(just(Kind::RightBracket.into(), '}'))
// //         .map(Json::Object)
// //         .node(Kind::Object.into());

// //     // Finally!
// //     value.define(
// //         null.or(boolean)
// //             .map(Either::into_inner)
// //             .or(string.map(Json::String))
// //             .map(Either::into_inner)
// //             .or(array)
// //             .map(Either::into_inner)
// //             .or(object)
// //             .map(Either::into_inner),
// //     );
// //     optional_whitespace
// //         .clone()
// //         .ignore_then(value)
// //         .then_ignore(optional_whitespace)
// //         .then_ignore(end())
// //         .node(Kind::Root.into())
// // }

// // #[test]
// // fn package_data() {
// //     let data = indoc! {r#"
// //         {
// //             "name": "biparse",
// //             "version": 0,
// //             "features: [
// //                 "recursion",
// //                 "forest",
// //                 { "left_recursive": true }
// //             ]
// //         }
// //     "#};
// //     let parser = json_parser();
// //     let (_, result): (_, ParseOutput<Json, DefaultParseEnvironment>) =
// //         parser.parse_from_start(data);
// //     expect![[r#"
// //         Speculative(
// //             Simple {
// //                 position: LineCol {
// //                     line: 2,
// //                     col: 22,
// //                 },
// //                 messages: [
// //                     "expected character '}', found UTF-8 character ',' at location (2, 22)",
// //                 ],
// //             },
// //         )
// //     "#]]
// //     .assert_debug_eq(&result.ast);

// //     let forest = result.cst.green;
// //     println!("{:#?}", forest);
// //     assert_eq!(forest.len(), 1);
// //     let root = match &forest[0] {
// //         rowan::NodeOrToken::Node(node) => node.clone(),
// //         rowan::NodeOrToken::Token(_) => panic!(),
// //     };
// //     let tree: rowan::SyntaxNode<Kind> = rowan::SyntaxNode::new_root(root);

// //     assert_eq!(tree.text(), data);
// //     expect![].assert_debug_eq(&tree);
// // }
