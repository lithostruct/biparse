//! Module testing [`TryMap`](biparse::combinator::TryMap).

use std::str::FromStr;

use biparse::{input::Matches, prelude::*};
use rowan::SyntaxKind;

/// Creates a fallible parser that parses a [`u16`] value with no delimiters or whitespace.
fn parser_u16<'src, E: ParseEnvironment>() -> impl Parser<'src, str, u16, E> {
    just((SyntaxKind(0), regex::Regex::new(r"[0-9]+").unwrap())).try_map(<u16 as FromStr>::from_str)
}

#[test]
fn try_map_u16() {
    parser_u16::<DefaultParseEnvironment>()
        .parse_all("324")
        .expect("Successful parse!");

    parser_u16::<DefaultParseEnvironment>()
        .parse_all("unexpected")
        .expect_err("No parse!");
    parser_u16::<DefaultParseEnvironment>()
        .parse_all("123456789")
        .expect_err("Conversion fail!");
}

#[test]
fn try_map_with_recovery() {
    let parser = parser_u16::<DefaultParseEnvironment>()
        .map(Option::Some)
        .recover(
            just(Matches::always::<str>(SyntaxKind(0))).many(),
            |_, _| None,
            SyntaxKind(999),
        );
    assert!(
        parser
            .parse_all("123456789") // exceeds u16::MAX!
            .expect("Conversion failure recovered!")
            .into_ast_only()
            .is_none()
    )
}
