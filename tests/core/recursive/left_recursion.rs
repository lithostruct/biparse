//! Test regarding left-recursion of parser combinators involving the seed-growing algorithm.

use biparse::prelude::*;
use expect_test::expect;
use proptest::{prop_oneof, proptest, strategy::Strategy, test_runner::Config};
use rowan::SyntaxKind;

/// Matches string `s` on zero or more occurences of 'a'.
fn left_recursive_a(s: String) -> String {
    let combinator: Recursive<Direct<'_, str, String, DefaultParseEnvironment>> =
        Recursive::new_as("left-recursive-a", |this| {
            this.then(just((SyntaxKind(0), 'a')))
                .map(|(mut rest, last)| {
                    rest += last;
                    rest
                })
                .or(success())
                .map(|either| either.left_or_default())
        });
    let (state, result) = combinator.node(SyntaxKind(1)).parse_from_start(&s);
    println!("{:#?}", state.recursive_state);
    match result.ast {
        ParseResult::Success(header) => {
            if state.cursor.into_usize() == 0 {
                assert_eq!(&header, "");
            } else {
                assert_eq!(header, s);
            }
            header
        }
        ParseResult::Speculative(_) | ParseResult::Committed(_) => unreachable!(),
    }
}

#[test]
fn zero_a() {
    expect![""].assert_eq(&left_recursive_a("".to_string()));
}
#[test]
fn one_a() {
    expect!["a"].assert_eq(&left_recursive_a("a".to_string()));
}
#[test]
fn five_a() {
    expect!["aaaaa"].assert_eq(&left_recursive_a("aaaaa".to_string()));
}
#[test]
fn other_string() {
    expect![""].assert_eq(&left_recursive_a("lorem ipsum dolor sit amet".to_string()));
}

/// The seed algorithm should be near-linear.
#[test]
fn too_many_a() {
    expect!["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"].assert_eq(&left_recursive_a("a".repeat(30)));
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
        left_recursive_a(s);
    }
}
