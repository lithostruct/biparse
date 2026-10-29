//! Tests for the `End` sentinel combinator.

use biparse::prelude::*;
use expect_test::expect;
use rowan::SyntaxKind;

#[test]
fn single_end() {
    const NO_TEXT: &str = "";
    let no_input: ParseOutput<(), DefaultParseEnvironment> = end()
        .parse(
            NO_TEXT,
            ParseState::<str, DefaultParseEnvironment>::new(NO_TEXT),
        )
        .1;
    assert!(matches!(no_input.ast, ParseOutcome::Success(_)));

    const SOME_TEXT: &str = "lorem ipsum dolor sit amet";
    let (state, some_input): (_, ParseOutput<(), DefaultParseEnvironment>) = end().parse(
        SOME_TEXT,
        ParseState::<str, DefaultParseEnvironment>::new(SOME_TEXT),
    );
    assert!(matches!(some_input.ast, ParseOutcome::Speculative(_)));
    assert_eq!(state.cursor.into_usize(), 0);
}

#[test]
fn just_then_end() {
    let combinator: With<DefaultParseEnvironment, _> = just((SyntaxKind(0), "a")).then(end());

    let not_enough = combinator.parse_all("");
    let just_right = combinator.parse_all("a");
    let too_much = combinator.parse_all("ab");

    assert!(not_enough.is_err());
    assert_eq!(just_right.expect("success!").into_ast_only(), ("a", ()));
    assert!(too_much.is_err());
}

/// Specific to non-contiguous input:
/// If a non-empty input produces no meaningful tokens, the CST actualized should still be complete.
///
/// This relates to logos-based input and `SkippingStr<'_>`.
#[test]
fn all_trivia_end() {
    use biparse::input::str::SkippingStr;

    let ends: As<'_, SkippingStr<'_>, (), DefaultParseEnvironment, _> = end();

    const ALL_TRIVIA: &str = "     "; // length 5.
    let input = SkippingStr::whitespace(ALL_TRIVIA, SyntaxKind(0));
    let artifact = ends.parse_all(&input).expect("Success!");

    let green = artifact.into_cst();
    assert_eq!(
        Into::<usize>::into(green.forest()[0].text_len()),
        ALL_TRIVIA.len()
    );

    // Actual well-formedness not tested; see [`end_then_end`].
}

/// Regression:
///
/// Prevent duplicate actualization of all-trivia input.
#[test]
fn end_then_end() {
    use biparse::{cst::NoLang, input::str::SkippingStr};

    const ALL_TRIVIA: &str = "     "; // length 5.
    let input = SkippingStr::whitespace(ALL_TRIVIA, SyntaxKind(0));

    let ends: Box<dyn Parser<'_, SkippingStr<'_>, ((), ()), DefaultParseEnvironment>> =
        end().then(end()).boxed();

    // `parse_all` adds one more `end`, so the text is tripled if not handled correctly.
    let artifact = ends.parse_all(&input).expect("Success!");

    let red = artifact.into_cst().canonize::<NoLang>(SyntaxKind(0));
    expect![[r#"
        5
    "#]]
    .assert_debug_eq(&red.tree().text().len());
    assert_eq!(red.tree().text(), ALL_TRIVIA)
}
