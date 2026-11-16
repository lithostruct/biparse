//! Module testing [`Commit`](biparse::combinator::Commit).

use std::rc::Rc;

use biparse::{input::Matches, prelude::*};
use expect_test::expect;
use rowan::SyntaxKind;

// Assert that success is unaffected,
// and speculative failure is promoted to committed failure.
#[test]
fn commit_semantics() {
    assert!(
        success::<str, DefaultParseEnvironment>()
            .commit()
            .parse_all("")
            .is_ok()
    );
    assert!(
        fail::<str, !, DefaultParseEnvironment>()
            .commit()
            .parse_all("")
            .is_err()
    );

    let combinator = fail::<str, !, DefaultParseEnvironment>().commit();
    let (_, output) = combinator.parse("", ParseState::<str, _>::new(""));
    assert!(matches!(output.ast, ParseOutcome::Committed(_)));
}

/// Assert that committed failure follows exception semantics.
///
/// Commitment occurs within the `then` continuation ie. if the first match succeeds.
#[test]
fn commit_within_then() {
    /// Expects "id" then "ent" (committed)
    fn make_parser<'src>() -> impl Parser<'src, str, (), DefaultParseEnvironment> {
        just((SyntaxKind(0), "id"))
            .then(just((SyntaxKind(0), "ent")).commit())
            .ignore()
            .shared()
    }

    const SPECULATIVE: &str = "complete mismatch";
    assert!(matches!(
        make_parser()
            .parse(SPECULATIVE, ParseState::<str, _>::new(SPECULATIVE))
            .1
            .ast,
        ParseOutcome::Speculative(_),
    ));

    const SUCCESS: &str = "ident";
    assert!(matches!(
        make_parser()
            .parse(SUCCESS, ParseState::<str, _>::new(SUCCESS))
            .1
            .ast,
        ParseOutcome::Success(_),
    ));

    const COMMITTED: &str = "id_error";
    assert!(matches!(
        make_parser()
            .parse(COMMITTED, ParseState::<str, _>::new(COMMITTED))
            .1
            .ast,
        ParseOutcome::Committed(_),
    ));
}

/// Assert that committed failure follows exception semantics.
///
/// Commitment occurs within the first `or` branch ie. if the first match succeeds.
#[test]
fn commit_within_or() {
    // Expects "var_" then an alphabetic identifier + '_' (committed), or succeed unconditionally.
    fn make_parser<'src>() -> impl Parser<'src, str, (), DefaultParseEnvironment> {
        (just((SyntaxKind(0), "var_"))
            .then(just((SyntaxKind(0), regex::Regex::new(r"[a-zA-Z_]+").unwrap())).commit()))
        .either_or(success())
        .ignore()
        .shared()
    }

    const PERMISSIVE: &str = "different syntactic category!";
    assert!(matches!(
        make_parser()
            .parse(PERMISSIVE, ParseState::<str, _>::new(PERMISSIVE))
            .1
            .ast,
        ParseOutcome::Success(_),
    ));

    const SUCCESS: &str = "var_some_name";
    assert!(matches!(
        make_parser()
            .parse(SUCCESS, ParseState::<str, _>::new(SUCCESS))
            .1
            .ast,
        ParseOutcome::Success(_),
    ));

    const COMMITTED: &str = "var_123_invalid!";
    assert!(matches!(
        make_parser()
            .parse(COMMITTED, ParseState::<str, _>::new(COMMITTED))
            .1
            .ast,
        ParseOutcome::Committed(_),
    ));
}

#[test]
fn commit_within_many() {
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct FailReason {
        provenance: usize,
        discarded: String,
    }
    /// Expects a whitespace-separated list delimited by parentheses,
    /// where elements are any alphanumeric identifier OTHER THAN that beginning with "FAIL:".
    ///
    /// If '(' is first identified, ')' becomes a checkpoint, and is recovered until then.
    fn make_parser<'src>()
    -> impl Parser<'src, str, Result<Vec<String>, FailReason>, DefaultParseEnvironment> {
        let whitespace = just((SyntaxKind(20), regex::Regex::new(r"[ \t\r\n]+").unwrap())).shared();
        let identifier: Rc<dyn Parser<'_, _, String, _>> = just((SyntaxKind(10), "FAIL:"))
            .then_ignore(fail::<_, !, _>().commit())
            .or(just((
                SyntaxKind(11),
                regex::Regex::new(r"[a-zA-Z0-9_]+").unwrap(),
            )))
            .into_owned()
            .shared();

        just((SyntaxKind(0), '('))
            .ignore_then(whitespace.clone().optional())
            .ignore_then(
                // BODY of list.
                identifier
                    .clone()
                    .then(whitespace.clone().ignore_then(identifier).many())
                    .map(|(first, mut rest)| {
                        rest.insert(0, first);
                        rest
                    })
                    .optional()
                    .map(Option::unwrap_or_default),
            )
            .then_ignore(whitespace.clone().optional())
            .then_ignore(just((SyntaxKind(1), ')')).commit())
            .map(Result::Ok)
            // RECOVERY: up to and including checkpoint character ')'.
            .recover(
                just(Matches(SyntaxKind(99), |c: &char| *c != ')'))
                    .many()
                    .then(just((SyntaxKind(1), ')'))),
                |uid: usize, collected| {
                    Result::Err(FailReason {
                        provenance: uid,
                        discarded: collected.0.iter().collect::<String>(), // closing parenthesis not included.
                    })
                },
                SyntaxKind(999),
            )
    }

    const SUCCESS: &str = "(lorem ipsum dolor sit amet)";
    assert_eq!(
        make_parser()
            .parse_all(SUCCESS)
            .expect("Success!")
            .into_ast_only()
            .expect("Total success in materializing Vec!")
            .as_ref(),
        ["lorem", "ipsum", "dolor", "sit", "amet"],
    );

    // Recovers and synchronizes up to recovery character ')', then end of input.
    const FAIL_PRAGMA: &str = "(prefix FAIL:pragma suffix)";
    let outcome = make_parser()
        .parse_all(FAIL_PRAGMA)
        .expect("Recovered success!");
    let ast = outcome.ast_only();
    assert_eq!(
        *ast,
        Err(FailReason {
            provenance: 0,
            discarded: String::from("pragma suffix")
        })
    );

    expect![[r#"
        [
            Rich {
                position: LineColumn {
                    line: 1,
                    column: 14,
                },
                kind: FailPrimitive {
                    message: None,
                },
                supplemental: [],
            },
        ]
    "#]]
    .assert_debug_eq(&outcome.diagnostics());
    let index = ast
        .as_ref()
        .expect_err("Failed to materialize Vec!")
        .provenance;
    assert_eq!(
        outcome
            .cst()
            .diagnostics()
            .get(&index)
            .map(|(rich, _)| rich.clone()),
        Some(outcome.diagnostics()[0].clone())
    );

    // Missing closing parenthesis. This failure is terminal.
    const TERMINAL: &str = "( missing paren";
    expect![[r#"
        (1, 16) expected:
              | └─ character ')'
              | found: end of input at (1, 16)"#]]
    .assert_eq(
        &make_parser()
            .parse_all(TERMINAL)
            .expect_err("Terminal failure!")
            .terminating
            .to_string(),
    );
}
