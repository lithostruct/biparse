//! Tests relating to the `Many` combinator.

use biparse::prelude::*;
use expect_test::expect;
use proptest::{prop_oneof, proptest, strategy::Strategy, test_runner::Config};
use rowan::SyntaxKind;

/// Matches string `s` on zero or more occurences of 'a'.
fn many_a(s: String) -> String {
    let parser = just::<'_, str, DefaultParseEnvironment, _>((SyntaxKind(0), 'a'))
        .many()
        .map(|vec| vec.join(""));
    let (state, result) = parser
        .node(SyntaxKind(1))
        .parse(&s, ParseState::<<str as Input>::Cursor, _>::new(s.as_str()));

    match result.ast {
        ParseOutcome::Success(header) => {
            if state.cursor.into_usize() == 0 {
                assert_eq!(&header, "");
            } else {
                assert_eq!(header, s);
            }
            header
        }
        ParseOutcome::Speculative(_) | ParseOutcome::Committed(_) => unreachable!(),
    }
}

#[test]
fn zero_a() {
    expect![""].assert_eq(&many_a("".to_string()));
}
#[test]
fn one_a() {
    expect!["a"].assert_eq(&many_a("a".to_string()));
}
#[test]
fn five_a() {
    expect!["aaaaa"].assert_eq(&many_a("aaaaa".to_string()));
}
#[test]
fn other_string() {
    expect![""].assert_eq(&many_a("lorem ipsum dolor sit amet".to_string()));
}

/// Linear execution. This should not be surprising.
#[test]
fn too_many_a() {
    expect!["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"].assert_eq(&many_a("a".repeat(30)));
}

fn some_amount_of_a() -> impl Strategy<Value = String> {
    prop_oneof![
        "[^a]{0, 100}",
        (0..50_usize).prop_map(|num: usize| "a".repeat(num)),
    ]
}
proptest! {
    #![proptest_config(Config {
        // Timeout in milliseconds.
        timeout: 10000,
        .. Config::default()
    })]
    #[test]
    fn zero_or_more_a(s in some_amount_of_a()) {
        many_a(s);
    }
}
