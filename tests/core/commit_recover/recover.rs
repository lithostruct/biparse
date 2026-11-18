//! Module testing [`Recover`](biparse::combinator::Recover).

use biparse::{cst::NoLang, input::Matches, prelude::*};
use expect_test::expect;
use rowan::SyntaxKind;

// Expects "something" or "strict: requirement". If "strict: " matched, commit and recover to end.
fn make_parser<'src>() -> impl Parser<'src, str, Result<String, (usize, String)>, environment::Rich>
{
    just((SyntaxKind(0), "something"))
        .into_owned()
        .or(just((SyntaxKind(1), "strict: "))
            .into_owned()
            .then(just((SyntaxKind(2), "requirement")).commit())
            .map(|(s1, s2)| s1 + s2))
        .map(Result::Ok)
        .recover(
            just(Matches(SyntaxKind(1), |_: &char| true)).many(),
            |uid, chars| Err((uid, chars.into_iter().collect::<String>())),
            SyntaxKind(999),
        )
}

/// Asserts that recovery only interacts with committed failure.
#[test]
fn do_not_recover() {
    const FIRST_BRANCH: &str = "something";
    make_parser()
        .parse_all(FIRST_BRANCH)
        .expect("Success on first branch!");

    const SPECULATIVE: &str = "not in scope";
    make_parser()
        .parse_all(SPECULATIVE)
        .expect_err("Speculative failure from no match on either branch!");

    // Success after commitment results in a Success payload.
    const SECOND_BRANCH: &str = "strict: requirement";
    make_parser()
        .parse_all(SECOND_BRANCH)
        .expect("Success on second branch!");
}

/// Asserts that recovery turns a committed failure into an AST payload, and that CST remains total.
#[test]
fn recovery_success() {
    const COMMIT_AND_RECOVER: &str = "strict: unexpected sequence!";
    let artifact = make_parser()
        .parse_all(COMMIT_AND_RECOVER)
        .expect("Committed failure and recovery!");
    assert_eq!(
        *artifact.ast_only(),
        Err((0, String::from("unexpected sequence!")))
    );

    let red = artifact.into_cst().canonize::<NoLang>(SyntaxKind(0));
    assert_eq!(red.tree().text(), COMMIT_AND_RECOVER);

    let entry = red.diagnose(&0).expect("In diagnostics!");
    expect![[r#"
        (1, 9) expected:
             | └─ string "requirement"
             | found: 8 at (1, 9)"#]]
    .assert_eq(&entry.0.to_string());
    // assert_eq!(entry.1.text(), "unexpected sequence!");
}

/// Asserts that recovery parser failure propagates the committed failure.
#[test]
fn recovery_nested_failure() {
    let bad_recovery = fail::<str, (), environment::Rich>().commit().recover(
        fail(), // Speculative failure in recovery parser!
        |_, ()| unreachable!(),
        SyntaxKind(999),
    );
    const INPUT: &str = "Lorem ipsum dolor sit amet";
    let (_, outcome) = bad_recovery.parse(INPUT, ParseState::<str, environment::Rich>::new(INPUT));
    assert!(matches!(outcome.ast, ParseOutcome::Committed(_)));
}
