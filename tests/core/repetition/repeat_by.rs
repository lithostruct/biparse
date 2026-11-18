//! Test relating to [`ParserExt::repeat_by`] and more generally [`Repetition`](biparse::combinator::Repetition).

use biparse::prelude::*;
use expect_test::expect;

#[test]
#[should_panic]
fn invalid_construction() {
    let _ = success::<str, environment::Rich>().repeat_by(0..0);
}

#[test]
fn bounded_repetition() {
    fn a_parser_1to5<'src>() -> impl Parser<'src, str, Vec<&'src str>, environment::Rich> {
        just((SyntaxKind(0), 'a')).repeat_by(1..=5)
    }

    const EMPTY: &str = "";
    let err = a_parser_1to5()
        .parse_all(EMPTY)
        .expect_err("Must have at least one 'a'!");
    expect![[r#"
        (1, 1) expected:
             | └─ character 'a'
             | found: end of input at (1, 1)
        supplemental diagnostics:
            (1, 1) insufficient repetition:
                 | expect at least 1 repetitions;
                 | failed in iteration 1."#]]
    .assert_eq(&err.terminating.to_string());

    const SOME: &str = "aaa";
    a_parser_1to5().parse_all(SOME).expect("Success!");

    const TOO_MUCH: &str = "aaaaaaaaaa"; // 10 characters.
    let err = a_parser_1to5()
        .parse_all(TOO_MUCH)
        .expect_err("Did not reach end-of-input!");
    expect![[r#"
        (1, 6) expected:
             | └─ end of input
             | found: character 'a' at (1, 6)"#]]
    .assert_eq(&err.terminating.to_string());
}
