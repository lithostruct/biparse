//! Test regarding typical right-recursion of parser combinators.

use biparse::prelude::*;
use expect_test::expect;
use proptest::{prop_oneof, proptest, strategy::Strategy, test_runner::Config};
use rowan::SyntaxKind;

/// Matches string `s` on zero or more occurences of 'a'.
fn right_recursive_a(s: String) -> String {
    let combinator: Recursive<Direct<'_, str, String, DefaultParseEnvironment>> =
        Recursive::new(|this| {
            just((SyntaxKind(0), 'a'))
                .then(this)
                .map(|(first, mut rest): (&str, String)| {
                    rest.insert_str(0, first);
                    rest
                })
                .or(success())
                .map(|either| either.map_right(|_| String::new()).into_inner())
        });
    let (state, result) = combinator.node(SyntaxKind(1)).parse_from_start(&s);
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
    expect![""].assert_eq(&right_recursive_a("".to_string()));
}
#[test]
fn one_a() {
    expect!["a"].assert_eq(&right_recursive_a("a".to_string()));
}
#[test]
fn five_a() {
    expect!["aaaaa"].assert_eq(&right_recursive_a("aaaaa".to_string()));
}
#[test]
fn other_string() {
    expect![""].assert_eq(&right_recursive_a("lorem ipsum dolor sit amet".to_string()));
}

/// Regression:
///
/// Previous versions of the seed-growing algorithm discard each fixpoint growth after its invocation,
/// and the seed and growth phases ran at least twice, leading to recurrence relation:
/// `T(n) = 2*T(n-1) + O(n)`
/// where no caching occurs for the recurrence,
/// for a worst-case exponential time complexity `O(2^N)` for right-recursive grammars.
///
/// This previously took >60 seconds to run. It should now be with near-linear or low-polynomic time complexity instead.
#[test]
fn too_many_a() {
    expect!["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"].assert_eq(&right_recursive_a("a".repeat(30)));
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
        right_recursive_a(s);
    }
}
