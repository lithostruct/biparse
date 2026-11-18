//! Tests relating to the `many` combinator.
//!
//! These tests were written before [`ParserExt::many`] was augmented/lifted to [`Repetition`](biparse::combinator::Repetition).
//! They remain here because this is a useful dual to the tests in [`crate::recursive`],
//! and also that this would be the most common repetition construct used.
//!
//! Refer to [`super::repeat_by`] for invariant testing regarding that.

use biparse::prelude::*;
use expect_test::expect;
use proptest::{proptest, strategy::Strategy, test_runner::Config};
use rowan::SyntaxKind;

/// Matches string `s` on zero or more occurrences of 'a'.
fn many_a(s: String) -> String {
    let parser = just::<'_, str, environment::Rich, _>((SyntaxKind(0), 'a'))
        .many()
        .map(|vec| vec.join(""));
    let (state, output) = parser
        .node(SyntaxKind(1))
        .parse(&s, ParseState::<str, environment::Rich>::new(s.as_str()));

    match output.ast {
        ParseOutcome::Success(header) => {
            if state.cursor.into_usize() == 0 {
                assert_eq!(&header, "");
            } else {
                assert_eq!(header, s);
            }
            header
        }
        _ => unreachable!(),
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
    (0..50_usize).prop_map(|num: usize| "a".repeat(num))
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

#[test]
fn non_growing_termination() {
    let non_growing = success().many();
    let (_, outcome) = non_growing.parse("", ParseState::<str, environment::Rich>::new(""));
    assert!(matches!(outcome.ast, ParseOutcome::Fatal(_)))
}
